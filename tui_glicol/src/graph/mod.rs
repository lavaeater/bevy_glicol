use glicol_synth::GlicolPara;
use hashbrown::HashMap;

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
    nodes: HashMap<String, Node>,
    registry: NodeRegistry,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            registry: NodeRegistry::new(),
        }
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
        let definition = self.registry.get_definition(&node.node_type)
            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?;

        // Check if we have the right number of parameters
        let required_params = definition.parameters.iter()
            .filter(|p| !p.optional)
            .count();

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
                let params: Vec<GlicolPara> = node.parameters.iter().cloned().collect();
                node_params.push(params);
            }

            ast.insert(id.clone(), (node_inputs, node_params));
        }

        ast
    }

    pub fn get_registry(&self) -> &NodeRegistry {
        &self.registry
    }
}
