use glicol_synth::GlicolPara;
use hashbrown::{HashMap, HashSet};

mod node_types;
pub use node_types::*;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub node_type: String,
    pub parameters: Vec<GlicolPara>,
    pub inputs: Vec<String>,  // IDs of nodes connected as inputs
    pub position: (f32, f32), // UI position
}

impl Node {
    pub fn new(id: String, node_type: String, position: (f32, f32)) -> Self {
        Self {
            id,
            node_type,
            parameters: Vec::new(),
            inputs: Vec::new(),
            position,
        }
    }

    pub fn add_parameter(&mut self, value: GlicolPara) {
        self.parameters.push(value);
    }

    pub fn connect_input(&mut self, input_id: String) {
        if !self.inputs.contains(&input_id) {
            self.inputs.push(input_id);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub nodes: HashMap<String, Node>,
    pub registry: NodeRegistry,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            registry: NodeRegistry::new(),
        }
    }

    pub fn nodes(&self) -> &HashMap<String, Node> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut HashMap<String, Node> {
        &mut self.nodes
    }

    pub fn add_node(&mut self, node: Node) -> Result<(), String> {
        // Validate that the node type exists in our registry
        if self.registry.get_definition(&node.node_type).is_none() {
            return Err(format!("Unknown node type: {}", node.node_type));
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn remove_node(&mut self, id: &str) {
        // Remove the node
        self.nodes.remove(id);

        // Remove any connections to this node
        for node in self.nodes.values_mut() {
            node.inputs.retain(|input_id| input_id != id);
        }
    }

    pub fn connect(&mut self, from_id: &str, to_id: &str) -> Result<(), String> {
        if !self.nodes.contains_key(from_id) || !self.nodes.contains_key(to_id) {
            return Err("Node not found".to_string());
        }

        if let Some(node) = self.nodes.get_mut(to_id) {
            node.connect_input(from_id.to_string());
        }
        Ok(())
    }

    pub fn validate_node_parameters(&self, node: &Node) -> Result<(), String> {
        let definition = self
            .registry
            .get_definition(&node.node_type)
            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?;

        // Check if we have the right number of parameters
        let required_params = definition.parameters.iter().filter(|p| !p.optional).count();

        if node.parameters.len() < required_params {
            return Err(format!(
                "Node {} requires {} parameters, but only {} were provided",
                node.node_type,
                required_params,
                node.parameters.len()
            ));
        }

        Ok(())
    }

    pub fn to_glicol_ast(&self) -> HashMap<String, (Vec<String>, Vec<Vec<GlicolPara>>)> {
        let mut ast = HashMap::new();

        for (id, node) in &self.nodes {
            let mut node_inputs = Vec::new();
            let mut node_params = Vec::new();

            // Add inputs
            for input_id in &node.inputs {
                node_inputs.push(input_id.clone());
            }

            // Add parameters
            if !node.parameters.is_empty() {
                let params: Vec<GlicolPara> = node.parameters.to_vec();
                node_params.push(params);
            }

            ast.insert(id.clone(), (node_inputs, node_params));
        }

        ast
    }

    pub fn get_registry(&self) -> &NodeRegistry {
        &self.registry
    }

    /// Serialize the node graph into Glicol DSL code.
    ///
    /// Detects linear chains (A >> B >> C) and flattens them onto single lines.
    /// Terminal nodes (not used as input by others) become output lines (`o`, `o1`, …).
    /// Intermediate nodes referenced by multiple consumers get their own `~name:` lines.
    pub fn to_glicol_code(&self) -> String {
        if self.nodes.is_empty() {
            return String::new();
        }

        // Build consumers map: node_id -> list of node_ids that reference it as input
        let mut consumers: HashMap<&str, Vec<&str>> = HashMap::new();
        for (id, node) in &self.nodes {
            for input_id in &node.inputs {
                consumers
                    .entry(input_id.as_str())
                    .or_default()
                    .push(id.as_str());
            }
        }

        let mut assigned: HashSet<String> = HashSet::new();
        let mut lines = Vec::new();
        let mut output_counter = 0usize;

        // Terminal nodes: not used as input by any other node -> output lines
        let terminals: Vec<&str> = self
            .nodes
            .keys()
            .filter(|id| !consumers.contains_key(id.as_str()))
            .map(|s| s.as_str())
            .collect();

        for &terminal_id in &terminals {
            let chain = self.build_chain(terminal_id, &consumers, &mut assigned);
            let name = if output_counter == 0 {
                "o".to_string()
            } else {
                format!("o{}", output_counter)
            };
            output_counter += 1;
            lines.push(format!("{}: {}", name, self.chain_to_code(&chain)));
        }

        // Remaining unassigned nodes get ~reference lines
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

    /// Walk backwards from `start_id`, absorbing single-input/single-consumer nodes
    /// into one linear chain. Returns node IDs ordered source-first.
    fn build_chain<'a>(
        &'a self,
        start_id: &'a str,
        consumers: &HashMap<&str, Vec<&str>>,
        assigned: &mut HashSet<String>,
    ) -> Vec<&'a str> {
        let mut chain = vec![start_id];
        assigned.insert(start_id.to_string());

        let mut current = start_id;
        loop {
            let node = match self.nodes.get(current) {
                Some(n) => n,
                None => break,
            };
            if node.inputs.len() != 1 {
                break;
            }
            let input_id = node.inputs[0].as_str();
            if assigned.contains(input_id) {
                break;
            }
            if !self.nodes.contains_key(input_id) {
                break;
            }
            // Only absorb if the input has exactly one consumer
            match consumers.get(input_id) {
                Some(cs) if cs.len() == 1 => {}
                _ => break,
            }
            chain.push(input_id);
            assigned.insert(input_id.to_string());
            current = input_id;
        }

        chain.reverse(); // source-first order
        chain
    }

    /// Convert a chain of node IDs into the right-hand side of a Glicol line.
    fn chain_to_code(&self, chain: &[&str]) -> String {
        let mut parts = Vec::new();

        for (i, &node_id) in chain.iter().enumerate() {
            let node = &self.nodes[node_id];

            // First node in the chain: reference any external inputs not in this chain
            if i == 0 {
                for input_id in &node.inputs {
                    if !chain.contains(&input_id.as_str()) {
                        parts.push(format!("~{}", sanitize_id(input_id)));
                    }
                }
            }

            // node_type followed by its parameters
            let mut node_str = node.node_type.clone();
            for param in &node.parameters {
                node_str.push(' ');
                node_str.push_str(&format_param(param));
            }
            parts.push(node_str);
        }

        parts.join(" >> ")
    }
}

/// Sanitize a node ID into a valid Glicol reference name
/// (lowercase ASCII letters, digits, underscores only).
fn sanitize_id(id: &str) -> String {
    id.chars()
        .flat_map(|c| {
            if c == '-' {
                '_'.to_lowercase()
            } else {
                c.to_lowercase()
            }
        })
        .collect()
}

/// Format a GlicolPara as Glicol DSL text.
fn format_param(param: &GlicolPara) -> String {
    match param {
        GlicolPara::Number(n) => {
            if n.is_finite() && *n == n.floor() && n.abs() < 1e9 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        GlicolPara::Reference(r) => {
            if r.starts_with('~') {
                r.clone()
            } else {
                format!("~{}", r)
            }
        }
        GlicolPara::SampleSymbol(s) => format!("\\{}", s),
        _ => "0".to_string(), // fallback for unsupported param types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_produces_empty_code() {
        let graph = Graph::new();
        assert_eq!(graph.to_glicol_code(), "");
    }

    #[test]
    fn single_source_node() {
        let mut graph = Graph::new();
        let mut node = Node::new("osc".into(), "sin".into(), (0.0, 0.0));
        node.add_parameter(GlicolPara::Number(440.0));
        graph.add_node(node).unwrap();

        let code = graph.to_glicol_code();
        assert_eq!(code, "o: sin 440");
    }

    #[test]
    fn chain_flattening() {
        let mut graph = Graph::new();

        let mut sin_node = Node::new("a".into(), "sin".into(), (0.0, 0.0));
        sin_node.add_parameter(GlicolPara::Number(440.0));
        graph.add_node(sin_node).unwrap();

        let mut mul_node = Node::new("b".into(), "mul".into(), (1.0, 0.0));
        mul_node.add_parameter(GlicolPara::Number(0.3));
        mul_node.connect_input("a".into());
        graph.add_node(mul_node).unwrap();

        let code = graph.to_glicol_code();
        assert_eq!(code, "o: sin 440 >> mul 0.3");
    }

    #[test]
    fn shared_node_gets_own_line() {
        let mut graph = Graph::new();

        // Shared source
        let mut src = Node::new("src".into(), "sin".into(), (0.0, 0.0));
        src.add_parameter(GlicolPara::Number(220.0));
        graph.add_node(src).unwrap();

        // Two consumers
        let mut a = Node::new("a".into(), "mul".into(), (1.0, 0.0));
        a.add_parameter(GlicolPara::Number(0.5));
        a.connect_input("src".into());
        graph.add_node(a).unwrap();

        let mut b = Node::new("b".into(), "add".into(), (1.0, 1.0));
        b.add_parameter(GlicolPara::Number(0.1));
        b.connect_input("src".into());
        graph.add_node(b).unwrap();

        let code = graph.to_glicol_code();
        // src has two consumers so it can't be absorbed — gets its own ~line
        assert!(code.contains("~src: sin 220"));
        // Both consumers reference ~src
        assert!(code.contains("~src >> mul 0.5"));
        assert!(code.contains("~src >> add 0.1"));
    }

    #[test]
    fn reference_param_formatting() {
        assert_eq!(format_param(&GlicolPara::Number(1.0)), "1");
        assert_eq!(format_param(&GlicolPara::Number(0.5)), "0.5");
        assert_eq!(format_param(&GlicolPara::Reference("mod".into())), "~mod");
        assert_eq!(format_param(&GlicolPara::Reference("~mod".into())), "~mod");
    }

    #[test]
    fn sanitize_uuid_style_id() {
        let id = "node_550E8400-E29B-41D4-A716-446655440000";
        let sanitized = sanitize_id(id);
        assert_eq!(sanitized, "node_550e8400_e29b_41d4_a716_446655440000");
        // Must start with ASCII_ALPHA_LOWER per Glicol grammar
        assert!(sanitized.chars().next().unwrap().is_ascii_lowercase());
    }
}
