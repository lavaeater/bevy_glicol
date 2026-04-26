use bevy::prelude::*;
use glicol_synth::GlicolPara;
use std::collections::{HashMap, HashSet};

// ── Parameter / node type metadata ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    Number,
    Reference,
    Pattern,
    Event,
    Code,
}

#[derive(Debug, Clone)]
pub struct NodeParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub optional: bool,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeCategory {
    Oscillator,
    Filter,
    Effect,
    Modulator,
    Math,
    IO,
    Utility,
}

#[derive(Debug, Clone)]
pub struct NodeTypeDefinition {
    pub name: String,
    pub parameters: Vec<NodeParameter>,
    pub description: String,
    pub category: NodeCategory,
}

// ── NodeRegistry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Resource)]
pub struct NodeRegistry {
    definitions: Vec<NodeTypeDefinition>,
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeRegistry {
    pub fn new() -> Self {
        let mut r = Self { definitions: Vec::new() };
        r.register_all();
        r
    }

    pub fn get_definition(&self, node_type: &str) -> Option<&NodeTypeDefinition> {
        self.definitions.iter().find(|d| d.name == node_type)
    }

    pub fn definitions(&self) -> &[NodeTypeDefinition] {
        &self.definitions
    }

    fn register_all(&mut self) {
        // Oscillators
        for (name, desc) in [
            ("sin", "Sine wave oscillator"),
            ("saw", "Sawtooth wave oscillator"),
            ("squ", "Square wave oscillator"),
            ("tri", "Triangle wave oscillator"),
        ] {
            self.definitions.push(NodeTypeDefinition {
                name: name.into(),
                parameters: vec![NodeParameter {
                    name: "frequency".into(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Frequency in Hz".into(),
                }],
                description: desc.into(),
                category: NodeCategory::Oscillator,
            });
        }
        for (name, desc) in [
            ("sawsynth", "Sawtooth synthesizer with envelope"),
            ("squsynth", "Square synthesizer with envelope"),
            ("trisynth", "Triangle synthesizer with envelope"),
        ] {
            self.definitions.push(NodeTypeDefinition {
                name: name.into(),
                parameters: vec![
                    NodeParameter { name: "frequency".into(), parameter_type: ParameterType::Number, optional: false, description: "Frequency in Hz".into() },
                    NodeParameter { name: "amplitude".into(), parameter_type: ParameterType::Number, optional: false, description: "Amplitude (0–1)".into() },
                ],
                description: desc.into(),
                category: NodeCategory::Oscillator,
            });
        }
        // Filters
        for (name, desc) in [("lpf", "Low-pass filter"), ("hpf", "High-pass filter")] {
            self.definitions.push(NodeTypeDefinition {
                name: name.into(),
                parameters: vec![
                    NodeParameter { name: "cutoff".into(), parameter_type: ParameterType::Number, optional: false, description: "Cutoff frequency in Hz".into() },
                    NodeParameter { name: "q".into(), parameter_type: ParameterType::Number, optional: false, description: "Resonance Q".into() },
                ],
                description: desc.into(),
                category: NodeCategory::Filter,
            });
        }
        self.definitions.push(NodeTypeDefinition {
            name: "onepole".into(),
            parameters: vec![NodeParameter { name: "coefficient".into(), parameter_type: ParameterType::Number, optional: false, description: "Filter coefficient".into() }],
            description: "One-pole filter".into(),
            category: NodeCategory::Filter,
        });
        // Effects
        self.definitions.push(NodeTypeDefinition {
            name: "reverb".into(),
            parameters: vec![
                NodeParameter { name: "room_size".into(), parameter_type: ParameterType::Number, optional: false, description: "Room size".into() },
                NodeParameter { name: "damping".into(), parameter_type: ParameterType::Number, optional: false, description: "Damping".into() },
                NodeParameter { name: "wet_gain".into(), parameter_type: ParameterType::Number, optional: false, description: "Wet gain".into() },
                NodeParameter { name: "dry_gain".into(), parameter_type: ParameterType::Number, optional: false, description: "Dry gain".into() },
                NodeParameter { name: "width".into(), parameter_type: ParameterType::Number, optional: false, description: "Stereo width".into() },
            ],
            description: "Reverb effect".into(),
            category: NodeCategory::Effect,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "delay".into(),
            parameters: vec![NodeParameter { name: "time".into(), parameter_type: ParameterType::Number, optional: false, description: "Delay time in seconds".into() }],
            description: "Delay effect".into(),
            category: NodeCategory::Effect,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "pan".into(),
            parameters: vec![NodeParameter { name: "position".into(), parameter_type: ParameterType::Number, optional: false, description: "Pan position (−1 to 1)".into() }],
            description: "Stereo panner".into(),
            category: NodeCategory::Effect,
        });
        // Math
        for (name, desc, pname, pdesc) in [
            ("mul", "Multiply input by a constant", "factor", "Multiplication factor"),
            ("add", "Add a constant to input", "value", "Value to add"),
        ] {
            self.definitions.push(NodeTypeDefinition {
                name: name.into(),
                parameters: vec![NodeParameter { name: pname.into(), parameter_type: ParameterType::Number, optional: false, description: pdesc.into() }],
                description: desc.into(),
                category: NodeCategory::Math,
            });
        }
        // Modulators
        self.definitions.push(NodeTypeDefinition {
            name: "envperc".into(),
            parameters: vec![
                NodeParameter { name: "attack".into(), parameter_type: ParameterType::Number, optional: false, description: "Attack time in seconds".into() },
                NodeParameter { name: "release".into(), parameter_type: ParameterType::Number, optional: false, description: "Release time in seconds".into() },
            ],
            description: "Percussive envelope".into(),
            category: NodeCategory::Modulator,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "adsr".into(),
            parameters: vec![
                NodeParameter { name: "attack".into(), parameter_type: ParameterType::Number, optional: false, description: "Attack".into() },
                NodeParameter { name: "decay".into(), parameter_type: ParameterType::Number, optional: false, description: "Decay".into() },
                NodeParameter { name: "sustain".into(), parameter_type: ParameterType::Number, optional: false, description: "Sustain".into() },
                NodeParameter { name: "release".into(), parameter_type: ParameterType::Number, optional: false, description: "Release".into() },
            ],
            description: "ADSR envelope".into(),
            category: NodeCategory::Modulator,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "seq".into(),
            parameters: vec![NodeParameter { name: "pattern".into(), parameter_type: ParameterType::Pattern, optional: false, description: "Sequence pattern".into() }],
            description: "Pattern sequencer".into(),
            category: NodeCategory::Modulator,
        });
        // Utility / IO
        self.definitions.push(NodeTypeDefinition {
            name: "speed".into(),
            parameters: vec![NodeParameter { name: "rate".into(), parameter_type: ParameterType::Number, optional: false, description: "Playback rate multiplier".into() }],
            description: "Speed/rate control".into(),
            category: NodeCategory::Utility,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "const".into(),
            parameters: vec![NodeParameter { name: "value".into(), parameter_type: ParameterType::Number, optional: false, description: "Constant value".into() }],
            description: "Constant value generator".into(),
            category: NodeCategory::Utility,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "mix".into(),
            parameters: vec![NodeParameter { name: "inputs".into(), parameter_type: ParameterType::Reference, optional: false, description: "Input nodes to mix".into() }],
            description: "Mix multiple inputs".into(),
            category: NodeCategory::IO,
        });
        self.definitions.push(NodeTypeDefinition {
            name: "out".into(),
            parameters: vec![],
            description: "Audio output".into(),
            category: NodeCategory::IO,
        });
    }
}

// ── GlicolNode / GlicolGraph ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GlicolNode {
    pub id: String,
    pub node_type: String,
    pub parameters: Vec<GlicolPara<String>>,
    pub inputs: Vec<String>,
    pub position: Vec2,
}

impl GlicolNode {
    pub fn new(id: impl Into<String>, node_type: impl Into<String>, position: Vec2) -> Self {
        Self {
            id: id.into(),
            node_type: node_type.into(),
            parameters: Vec::new(),
            inputs: Vec::new(),
            position,
        }
    }

    pub fn with_param(mut self, p: GlicolPara<String>) -> Self {
        self.parameters.push(p);
        self
    }

    pub fn with_input(mut self, id: impl Into<String>) -> Self {
        let id = id.into();
        if !self.inputs.contains(&id) {
            self.inputs.push(id);
        }
        self
    }
}

/// The Glicol node graph held as a Bevy `Resource`.
#[derive(Debug, Clone, Default, Resource)]
pub struct GlicolGraph {
    pub nodes: HashMap<String, GlicolNode>,
}

impl GlicolGraph {
    pub fn add_node(&mut self, node: GlicolNode, registry: &NodeRegistry) -> Result<(), String> {
        if registry.get_definition(&node.node_type).is_none() {
            return Err(format!("Unknown node type: {}", node.node_type));
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);
        for node in self.nodes.values_mut() {
            node.inputs.retain(|i| i != id);
        }
    }

    pub fn connect(&mut self, from_id: &str, to_id: &str) -> Result<(), String> {
        if !self.nodes.contains_key(from_id) || !self.nodes.contains_key(to_id) {
            return Err("Node not found".into());
        }
        if let Some(node) = self.nodes.get_mut(to_id) {
            let from = from_id.to_string();
            if !node.inputs.contains(&from) {
                node.inputs.push(from);
            }
        }
        Ok(())
    }

    pub fn to_glicol_code(&self) -> String {
        if self.nodes.is_empty() {
            return String::new();
        }

        let mut consumers: HashMap<&str, Vec<&str>> = HashMap::new();
        for (id, node) in &self.nodes {
            for input_id in &node.inputs {
                consumers.entry(input_id.as_str()).or_default().push(id.as_str());
            }
        }

        let mut assigned: HashSet<String> = HashSet::new();
        let mut lines = Vec::new();
        let mut out_counter = 0usize;

        let terminals: Vec<&str> = self
            .nodes
            .keys()
            .filter(|id| !consumers.contains_key(id.as_str()))
            .map(String::as_str)
            .collect();

        for &tid in &terminals {
            let chain = self.build_chain(tid, &consumers, &mut assigned);
            let name = if out_counter == 0 { "o".into() } else { format!("o{}", out_counter) };
            out_counter += 1;
            lines.push(format!("{}: {}", name, self.chain_to_code(&chain)));
        }

        let remaining: Vec<String> = self
            .nodes
            .keys()
            .filter(|id| !assigned.contains(id.as_str()))
            .cloned()
            .collect();
        for id in &remaining {
            let chain = self.build_chain(id, &consumers, &mut assigned);
            lines.push(format!("~{}: {}", sanitize_id(id), self.chain_to_code(&chain)));
        }

        lines.join("\n")
    }

    fn build_chain<'a>(
        &'a self,
        start: &'a str,
        consumers: &HashMap<&str, Vec<&str>>,
        assigned: &mut HashSet<String>,
    ) -> Vec<&'a str> {
        let mut chain = vec![start];
        assigned.insert(start.to_string());
        let mut cur = start;
        loop {
            let node = match self.nodes.get(cur) { Some(n) => n, None => break };
            if node.inputs.len() != 1 { break; }
            let inp = node.inputs[0].as_str();
            if assigned.contains(inp) || !self.nodes.contains_key(inp) { break; }
            match consumers.get(inp) {
                Some(cs) if cs.len() == 1 => {}
                _ => break,
            }
            chain.push(inp);
            assigned.insert(inp.to_string());
            cur = inp;
        }
        chain.reverse();
        chain
    }

    fn chain_to_code(&self, chain: &[&str]) -> String {
        let mut parts = Vec::new();
        for (i, &id) in chain.iter().enumerate() {
            let node = &self.nodes[id];
            if i == 0 {
                for inp in &node.inputs {
                    if !chain.contains(&inp.as_str()) {
                        parts.push(format!("~{}", sanitize_id(inp)));
                    }
                }
            }
            let mut s = node.node_type.clone();
            for p in &node.parameters {
                s.push(' ');
                s.push_str(&format_param(p));
            }
            parts.push(s);
        }
        parts.join(" >> ")
    }
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .flat_map(|c| if c == '-' { '_'.to_lowercase() } else { c.to_lowercase() })
        .collect()
}

fn format_param(param: &GlicolPara<String>) -> String {
    match param {
        GlicolPara::Number(n) => {
            if n.is_finite() && *n == n.floor() && n.abs() < 1e9 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        GlicolPara::Reference(r) => {
            if r.starts_with('~') { r.clone() } else { format!("~{}", r) }
        }
        GlicolPara::SampleSymbol(s) => format!("\\{}", s),
        _ => "0".to_string(),
    }
}
