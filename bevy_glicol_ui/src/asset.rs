use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::graph::{GlicolGraph, GlicolNode, NodeRegistry};

// ── Asset type ────────────────────────────────────────────────────────────────

/// A loaded `.glicol` patch file. Raw source is preserved for round-trip saves.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct GlicolSource {
    pub raw: String,
    pub path: String,
}

impl GlicolSource {
    pub fn to_graph(&self, registry: &NodeRegistry) -> GlicolGraph {
        parse_glicol(&self.raw, registry)
    }
}

// ── AssetLoader ───────────────────────────────────────────────────────────────

#[derive(Default, TypePath)]
pub struct GlicolSourceLoader;

#[derive(Default, Serialize, Deserialize)]
pub struct GlicolSourceSettings;

impl AssetLoader for GlicolSourceLoader {
    type Asset = GlicolSource;
    type Settings = GlicolSourceSettings;
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        bevy::asset::io::Reader::read_to_end(reader, &mut bytes).await?;
        let raw = String::from_utf8_lossy(&bytes).into_owned();
        let path = load_context.path().path().to_string_lossy().into_owned();
        Ok(GlicolSource { raw, path })
    }

    fn extensions(&self) -> &[&str] {
        &["glicol"]
    }
}

// ── Trigger events (Bevy 0.18 observer pattern) ───────────────────────────────

/// Trigger to load a `.glicol` file from `assets/`. Path relative to `assets/`.
#[derive(Event)]
pub struct LoadGlicolFile(pub String);

/// Trigger to save the current graph back to a file path (relative to cwd).
#[derive(Event)]
pub struct SaveGlicolFile(pub String);

// ── In-flight load tracking ───────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct PendingGlicolLoad(pub Option<Handle<GlicolSource>>);

// ── Systems ───────────────────────────────────────────────────────────────────

/// Observer: react to `LoadGlicolFile` trigger.
pub fn on_load_glicol(
    trigger: On<LoadGlicolFile>,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingGlicolLoad>,
) {
    let handle: Handle<GlicolSource> = asset_server.load(&trigger.event().0);
    info!("[glicol] loading {}", trigger.event().0);
    pending.0 = Some(handle);
}

/// Observer: react to `SaveGlicolFile` trigger.
pub fn on_save_glicol(
    trigger: On<SaveGlicolFile>,
    graph: Res<GlicolGraph>,
) {
    let path = &trigger.event().0;
    let code = graph.to_glicol_code();
    match std::fs::write(path, &code) {
        Ok(_) => info!("[glicol] saved to {}", path),
        Err(e) => error!("[glicol] save failed: {}", e),
    }
}

/// System: once a pending load finishes, parse it into the graph and push to engine.
pub fn on_glicol_asset_loaded(
    mut pending: ResMut<PendingGlicolLoad>,
    sources: Res<Assets<GlicolSource>>,
    registry: Res<NodeRegistry>,
    mut graph: ResMut<GlicolGraph>,
    engine: Option<Res<bevy_glicol::prelude::GlicolEngine>>,
    mut asset_events: MessageReader<AssetEvent<GlicolSource>>,
) {
    for event in asset_events.read() {
        let id = match event {
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Added { id } => *id,
            _ => continue,
        };
        let handle = match &pending.0 {
            Some(h) if h.id() == id => h.clone(),
            _ => continue,
        };
        let Some(source) = sources.get(&handle) else { continue };

        *graph = source.to_graph(&registry);
        info!("[glicol] '{}' → {} nodes", source.path, graph.nodes.len());

        if let Some(engine) = &engine {
            engine.update_with_code(&source.raw);
            info!("[glicol] engine updated");
        }

        pending.0 = None;
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub fn parse_glicol(src: &str, registry: &NodeRegistry) -> GlicolGraph {
    let mut graph = GlicolGraph::default();
    let mut counter = 0usize;

    let joined = join_continuations(src);

    for raw_line in joined.lines() {
        let line = raw_line.trim();
        if line.is_empty() { continue; }

        let Some(colon) = line.find(':') else { continue };
        let label_raw = line[..colon].trim();
        let chain_text = line[colon + 1..].trim();
        if chain_text.is_empty() { continue; }

        // Strip the leading `~`; treat `o`, `o1`, `out` as output labels
        let label = label_raw.trim_start_matches('~').to_string();

        let segments: Vec<&str> = chain_text.split(">>").map(str::trim).collect();
        if segments.is_empty() { continue; }

        // Chain: left-to-right flow; the *last* segment gets the declared label.
        // Earlier segments become anonymous intermediates linked by input edges.
        let n = segments.len();
        let mut prev_id: Option<String> = None;

        for (i, seg) in segments.iter().enumerate() {
            let is_last = i == n - 1;
            let id = if is_last {
                label.clone()
            } else {
                counter += 1;
                format!("_c{}_{}", counter, label)
            };

            let (node_type, params_raw) = parse_segment(seg);
            if node_type.is_empty() { continue; }

            let mut node = GlicolNode::new(
                id.clone(),
                node_type,
                Vec2::new(i as f32 * 180.0, 0.0),
            );
            node.parameters = parse_params(&params_raw);
            if let Some(prev) = prev_id {
                node.inputs.push(prev);
            }

            graph.nodes.insert(id.clone(), node);
            prev_id = Some(id);
        }
    }

    graph
}

fn join_continuations(src: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in src.lines() {
        let line = match raw.find("//") {
            Some(pos) => raw[..pos].trim_end(),
            None => raw.trim_end(),
        };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if trimmed.starts_with(">>") {
            if let Some(last) = lines.last_mut() {
                last.push(' ');
                last.push_str(trimmed);
                continue;
            }
        }
        lines.push(trimmed.to_string());
    }
    lines.join("\n")
}

fn parse_segment(seg: &str) -> (String, String) {
    let mut parts = seg.splitn(2, char::is_whitespace);
    let node_type = parts.next().unwrap_or("").to_string();
    let rest = parts.next().unwrap_or("").trim().to_string();
    (node_type, rest)
}

fn parse_params(raw: &str) -> Vec<glicol_synth::GlicolPara<String>> {
    use glicol_synth::GlicolPara;
    let mut out = Vec::new();
    for token in raw.split_whitespace() {
        if token.starts_with('~') {
            out.push(GlicolPara::Reference(token[1..].to_string()));
        } else if token.starts_with('\\') {
            out.push(GlicolPara::SampleSymbol(token[1..].to_string()));
        } else if let Ok(n) = token.parse::<f32>() {
            out.push(GlicolPara::Number(n));
        }
        // Patterns, rhai blocks, etc. are skipped — preserved in raw source only
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> NodeRegistry { NodeRegistry::new() }

    #[test]
    fn parse_simple_out() {
        let src = "o: sin 440 >> mul 0.3";
        let g = parse_glicol(src, &reg());
        assert!(g.nodes.contains_key("o"), "missing output node");
        assert_eq!(g.nodes["o"].node_type, "mul");
    }

    #[test]
    fn parse_ref_node() {
        let src = "~mod: sin 0.2 >> mul 1300";
        let g = parse_glicol(src, &reg());
        assert!(g.nodes.contains_key("mod"));
        assert_eq!(g.nodes["mod"].node_type, "mul");
    }

    #[test]
    fn parse_continuation() {
        let src = "~t2: seq 33\n>> mul 1.0\n>> sawsynth 0.01 0.1";
        let g = parse_glicol(src, &reg());
        assert!(g.nodes.contains_key("t2"));
        assert_eq!(g.nodes["t2"].node_type, "sawsynth");
    }

    #[test]
    fn parse_strips_comments() {
        let src = "// full comment\no: sin 440 // inline";
        let g = parse_glicol(src, &reg());
        assert!(g.nodes.contains_key("o"));
    }

    #[test]
    fn chain_links_inputs() {
        let src = "o: sin 440 >> mul 0.3";
        let g = parse_glicol(src, &reg());
        // "o" (mul) should have one input pointing to the sin intermediate
        assert_eq!(g.nodes["o"].inputs.len(), 1);
    }
}
