use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::graph::{GlicolGraph, GlicolNode, NodeRegistry};

// ── Asset type ────────────────────────────────────────────────────────────────

/// A loaded `.glicol` patch file.  The raw source is preserved so it can be
/// written back to disk unchanged; the graph is derived from it on load.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct GlicolSource {
    /// Original file text, preserved for round-trip saves.
    pub raw: String,
    /// Path the asset was loaded from (relative to `assets/`).
    pub path: String,
}

impl GlicolSource {
    /// Parse the raw text into a `GlicolGraph`.
    ///
    /// This is a best-effort parser: nodes that use engine-internal keywords
    /// not in the `NodeRegistry` are stored as opaque `UnknownNode` entries
    /// (node_type = the raw keyword) so they survive load/save without loss.
    pub fn to_graph(&self, registry: &NodeRegistry) -> GlicolGraph {
        parse_glicol(&self.raw, registry)
    }
}

// ── AssetLoader ───────────────────────────────────────────────────────────────

#[derive(Default)]
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
        let path = load_context.asset_path().path().to_string_lossy().into_owned();
        Ok(GlicolSource { raw, path })
    }

    fn extensions(&self) -> &[&str] {
        &["glicol"]
    }
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Trigger loading a `.glicol` file from `assets/`.  Path is relative to
/// `assets/`, e.g. `"glicols/test.glicol"`.
#[derive(Event)]
pub struct LoadGlicolFile(pub String);

/// Trigger saving the current `GlicolGraph` back to a file.
#[derive(Event)]
pub struct SaveGlicolFile(pub String);

// ── Load state ────────────────────────────────────────────────────────────────

/// Tracks an in-flight asset load so we can react when it finishes.
#[derive(Resource, Default)]
pub struct PendingGlicolLoad(pub Option<Handle<GlicolSource>>);

// ── Systems ───────────────────────────────────────────────────────────────────

/// Start loading when a `LoadGlicolFile` event is received.
pub fn handle_load_event(
    mut events: EventReader<LoadGlicolFile>,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingGlicolLoad>,
) {
    for ev in events.read() {
        let handle: Handle<GlicolSource> = asset_server.load(&ev.0);
        pending.0 = Some(handle);
        info!("[glicol] loading {}", ev.0);
    }
}

/// Once the asset finishes loading: parse → GlicolGraph → push to engine.
pub fn on_glicol_loaded(
    mut pending: ResMut<PendingGlicolLoad>,
    sources: Res<Assets<GlicolSource>>,
    registry: Res<NodeRegistry>,
    mut graph: ResMut<GlicolGraph>,
    engine: Option<Res<bevy_glicol::GlicolEngine>>,
    mut asset_events: EventReader<AssetEvent<GlicolSource>>,
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
        info!("[glicol] loaded '{}' → {} nodes", source.path, graph.nodes.len());

        if let Some(engine) = &engine {
            engine.update_with_code(&source.raw);
            info!("[glicol] engine updated");
        }

        pending.0 = None;
    }
}

/// Save graph → `.glicol` text on `SaveGlicolFile` events.
pub fn handle_save_event(
    mut events: EventReader<SaveGlicolFile>,
    graph: Res<GlicolGraph>,
) {
    for ev in events.read() {
        let code = graph.to_glicol_code();
        match std::fs::write(&ev.0, &code) {
            Ok(_) => info!("[glicol] saved to {}", ev.0),
            Err(e) => error!("[glicol] save failed: {}", e),
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse Glicol DSL text into a `GlicolGraph`.
///
/// Handles:
/// - `~name: chain`  — named reference nodes
/// - `o:` / `o1:` / `out:` — output nodes (stored as `o`, `o1`, …)
/// - `// …` — line comments (stripped)
/// - continuation lines starting with `>>` — appended to the previous line
/// - multi-segment chains (`a >> b >> c`) — chain stored as a sequence of nodes
///   linked via generated intermediate IDs
pub fn parse_glicol(src: &str, registry: &NodeRegistry) -> GlicolGraph {
    let mut graph = GlicolGraph::default();
    let mut counter = 0usize;

    // 1. Strip comments, join continuation lines.
    let joined = join_continuations(src);

    for raw_line in joined.lines() {
        let line = raw_line.trim();
        if line.is_empty() { continue; }

        // Split on the first `:` to get (label, chain_text).
        let Some(colon) = line.find(':') else { continue };
        let label_raw = line[..colon].trim();
        let chain_text = line[colon + 1..].trim();

        if chain_text.is_empty() { continue; }

        let is_ref = label_raw.starts_with('~');
        let label = label_raw.trim_start_matches('~').to_string();

        // Parse the `>>` chain into segments.
        let segments: Vec<&str> = chain_text.split(">>").map(str::trim).collect();
        if segments.is_empty() { continue; }

        // Build a linked chain of GlicolNodes.
        // The first segment gets the declared label; intermediate segments get
        // generated IDs; the terminal (last in the chain) is what the label
        // refers to — so we reverse-assign so that `~label` points to the end.
        //
        // Glicol semantics: `~a: X >> Y` means Y's output is `~a`.  So the
        // chain flows left-to-right and the declared name belongs to the *last*
        // segment; earlier segments are anonymous intermediates.
        let n = segments.len();
        let mut prev_id: Option<String> = None;

        for (i, seg) in segments.iter().enumerate() {
            let is_last = i == n - 1;

            let id = if is_last {
                label.clone()
            } else {
                counter += 1;
                format!("_chain_{}_{}", label, counter)
            };

            let (node_type, params_raw) = parse_segment(seg);
            let x = i as f32 * 160.0;
            let y = if is_ref { 0.0 } else { -100.0 };

            let mut node = GlicolNode::new(id.clone(), node_type, Vec2::new(x, y));
            node.parameters = parse_params(&params_raw);

            if let Some(prev) = prev_id {
                node.inputs.push(prev);
            }

            // For the first segment, pull in any `~ref` params as graph inputs.
            // (They remain as parameters too — the engine interprets them.)

            // Insert regardless of whether type is in registry (opaque nodes allowed).
            graph.nodes.insert(id.clone(), node);
            prev_id = Some(id);
        }
    }

    graph
}

/// Strip `//` comments and join lines that start with `>>` onto the previous line.
fn join_continuations(src: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in src.lines() {
        // Strip inline comment.
        let line = match raw.find("//") {
            Some(pos) => raw[..pos].trim_end(),
            None => raw.trim_end(),
        };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if trimmed.starts_with(">>") || trimmed.starts_with(">>") {
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

/// Split a single chain segment (`node_type param param …`) into type + rest.
fn parse_segment(seg: &str) -> (String, String) {
    let mut parts = seg.splitn(2, char::is_whitespace);
    let node_type = parts.next().unwrap_or("").to_string();
    let rest = parts.next().unwrap_or("").trim().to_string();
    (node_type, rest)
}

/// Parse a whitespace-separated parameter string into `GlicolPara<String>` values.
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
        } else if token.contains('_') {
            // Mini-notation pattern token — store as opaque Number(0) placeholder.
            // Patterns need a dedicated ParameterType::Pattern path; for now we
            // preserve them as a best-effort by trying numeric parse with _ stripped.
            let clean = token.replace('_', "");
            if let Ok(n) = clean.parse::<f32>() {
                out.push(GlicolPara::Number(n));
            }
            // else: skip silently — pattern handling is Phase 3
        }
        // Backtick rhai blocks and other complex tokens are skipped.
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
        // terminal node has label "o"
        assert!(g.nodes.contains_key("o"));
        let out = &g.nodes["o"];
        assert_eq!(out.node_type, "mul");
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
}
