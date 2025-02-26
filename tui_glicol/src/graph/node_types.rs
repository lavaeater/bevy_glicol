use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    Number,
    Reference,
    Pattern,
    Event,
    Code,
}

impl Display for ParameterType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterType::Number => write!(f, "Number"),
            ParameterType::Reference => write!(f, "Reference"),
            ParameterType::Pattern => write!(f, "Pattern"),
            ParameterType::Event => write!(f, "Event"),
            ParameterType::Code => write!(f, "Code"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub optional: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct NodeTypeDefinition {
    pub name: String,
    pub parameters: Vec<NodeParameter>,
    pub description: String,
    pub category: NodeCategory,
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

#[derive(Clone, Debug)]
pub struct NodeRegistry {
    definitions: Vec<NodeTypeDefinition>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            definitions: Vec::new(),
        };
        registry.register_default_nodes();
        registry
    }

    pub fn get_definition(&self, node_type: &str) -> Option<&NodeTypeDefinition> {
        self.definitions.iter().find(|def| def.name == node_type)
    }

    fn register_default_nodes(&mut self) {
        // Oscillators
        self.register_oscillators();
        // Filters
        self.register_filters();
        // Effects
        self.register_effects();
        // Math operations
        self.register_math_ops();
        // Modulators
        self.register_modulators();
        // Utility
        self.register_utility();
        // IO
        self.register_io();
    }

    fn register_oscillators(&mut self) {
        // Basic oscillators
        for (name, desc) in [
            ("sin", "Sine wave oscillator"),
            ("saw", "Sawtooth wave oscillator"),
            ("squ", "Square wave oscillator"),
            ("tri", "Triangle wave oscillator"),
        ] {
            self.definitions.push(NodeTypeDefinition {
                name: name.to_string(),
                parameters: vec![NodeParameter {
                    name: "frequency".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Frequency in Hz".to_string(),
                }],
                description: desc.to_string(),
                category: NodeCategory::Oscillator,
            });
        }

        // Synthesizer oscillators
        for (name, desc) in [
            ("sawsynth", "Sawtooth synthesizer with envelope"),
            ("squsynth", "Square synthesizer with envelope"),
            ("trisynth", "Triangle synthesizer with envelope"),
        ] {
            self.definitions.push(NodeTypeDefinition {
                name: name.to_string(),
                parameters: vec![
                    NodeParameter {
                        name: "frequency".to_string(),
                        parameter_type: ParameterType::Number,
                        optional: false,
                        description: "Frequency in Hz".to_string(),
                    },
                    NodeParameter {
                        name: "amplitude".to_string(),
                        parameter_type: ParameterType::Number,
                        optional: false,
                        description: "Amplitude (0-1)".to_string(),
                    },
                ],
                description: desc.to_string(),
                category: NodeCategory::Oscillator,
            });
        }
    }

    fn register_filters(&mut self) {
        // Low-pass filter
        self.definitions.push(NodeTypeDefinition {
            name: "lpf".to_string(),
            parameters: vec![
                NodeParameter {
                    name: "cutoff".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Cutoff frequency in Hz".to_string(),
                },
                NodeParameter {
                    name: "q".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Resonance/Q factor".to_string(),
                },
            ],
            description: "Low-pass filter".to_string(),
            category: NodeCategory::Filter,
        });

        // High-pass filter
        self.definitions.push(NodeTypeDefinition {
            name: "hpf".to_string(),
            parameters: vec![
                NodeParameter {
                    name: "cutoff".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Cutoff frequency in Hz".to_string(),
                },
                NodeParameter {
                    name: "q".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Resonance/Q factor".to_string(),
                },
            ],
            description: "High-pass filter".to_string(),
            category: NodeCategory::Filter,
        });

        // One-pole filter
        self.definitions.push(NodeTypeDefinition {
            name: "onepole".to_string(),
            parameters: vec![NodeParameter {
                name: "coefficient".to_string(),
                parameter_type: ParameterType::Number,
                optional: false,
                description: "Filter coefficient".to_string(),
            }],
            description: "One-pole filter".to_string(),
            category: NodeCategory::Filter,
        });
    }

    fn register_effects(&mut self) {
        // Reverb
        self.definitions.push(NodeTypeDefinition {
            name: "reverb".to_string(),
            parameters: vec![
                NodeParameter {
                    name: "room_size".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Size of the reverb room".to_string(),
                },
                NodeParameter {
                    name: "damping".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Damping factor".to_string(),
                },
                NodeParameter {
                    name: "wet_gain".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Wet signal gain".to_string(),
                },
                NodeParameter {
                    name: "dry_gain".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Dry signal gain".to_string(),
                },
                NodeParameter {
                    name: "width".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Stereo width".to_string(),
                },
            ],
            description: "Reverb effect".to_string(),
            category: NodeCategory::Effect,
        });

        // Delay
        self.definitions.push(NodeTypeDefinition {
            name: "delay".to_string(),
            parameters: vec![NodeParameter {
                name: "time".to_string(),
                parameter_type: ParameterType::Number,
                optional: false,
                description: "Delay time in seconds".to_string(),
            }],
            description: "Delay effect".to_string(),
            category: NodeCategory::Effect,
        });

        // Pan
        self.definitions.push(NodeTypeDefinition {
            name: "pan".to_string(),
            parameters: vec![NodeParameter {
                name: "position".to_string(),
                parameter_type: ParameterType::Number,
                optional: false,
                description: "Pan position (-1 to 1)".to_string(),
            }],
            description: "Stereo panner".to_string(),
            category: NodeCategory::Effect,
        });
    }

    fn register_math_ops(&mut self) {
        // Basic math operations
        for (name, desc, param_name, param_desc) in [
            (
                "mul",
                "Multiply input by a constant",
                "factor",
                "Multiplication factor",
            ),
            ("add", "Add a constant to input", "value", "Value to add"),
        ] {
            self.definitions.push(NodeTypeDefinition {
                name: name.to_string(),
                parameters: vec![NodeParameter {
                    name: param_name.to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: param_desc.to_string(),
                }],
                description: desc.to_string(),
                category: NodeCategory::Math,
            });
        }
    }

    fn register_modulators(&mut self) {
        // Envelope
        self.definitions.push(NodeTypeDefinition {
            name: "envperc".to_string(),
            parameters: vec![
                NodeParameter {
                    name: "attack".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Attack time in seconds".to_string(),
                },
                NodeParameter {
                    name: "release".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Release time in seconds".to_string(),
                },
            ],
            description: "Percussive envelope".to_string(),
            category: NodeCategory::Modulator,
        });

        // ADSR
        self.definitions.push(NodeTypeDefinition {
            name: "adsr".to_string(),
            parameters: vec![
                NodeParameter {
                    name: "attack".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Attack time in seconds".to_string(),
                },
                NodeParameter {
                    name: "decay".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Decay time in seconds".to_string(),
                },
                NodeParameter {
                    name: "sustain".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Sustain level (0-1)".to_string(),
                },
                NodeParameter {
                    name: "release".to_string(),
                    parameter_type: ParameterType::Number,
                    optional: false,
                    description: "Release time in seconds".to_string(),
                },
            ],
            description: "ADSR envelope".to_string(),
            category: NodeCategory::Modulator,
        });

        // Sequencer
        self.definitions.push(NodeTypeDefinition {
            name: "seq".to_string(),
            parameters: vec![NodeParameter {
                name: "pattern".to_string(),
                parameter_type: ParameterType::Pattern,
                optional: false,
                description: "Sequence pattern".to_string(),
            }],
            description: "Pattern sequencer".to_string(),
            category: NodeCategory::Modulator,
        });
    }

    fn register_utility(&mut self) {
        // Speed control
        self.definitions.push(NodeTypeDefinition {
            name: "speed".to_string(),
            parameters: vec![NodeParameter {
                name: "rate".to_string(),
                parameter_type: ParameterType::Number,
                optional: false,
                description: "Playback rate multiplier".to_string(),
            }],
            description: "Speed/rate control".to_string(),
            category: NodeCategory::Utility,
        });

        // Constant value
        self.definitions.push(NodeTypeDefinition {
            name: "const".to_string(),
            parameters: vec![NodeParameter {
                name: "value".to_string(),
                parameter_type: ParameterType::Number,
                optional: false,
                description: "Constant value".to_string(),
            }],
            description: "Constant value generator".to_string(),
            category: NodeCategory::Utility,
        });
    }

    fn register_io(&mut self) {
        // Mix inputs
        self.definitions.push(NodeTypeDefinition {
            name: "mix".to_string(),
            parameters: vec![NodeParameter {
                name: "inputs".to_string(),
                parameter_type: ParameterType::Reference,
                optional: false,
                description: "Input nodes to mix".to_string(),
            }],
            description: "Mix multiple inputs".to_string(),
            category: NodeCategory::IO,
        });

        // Output
        self.definitions.push(NodeTypeDefinition {
            name: "out".to_string(),
            parameters: vec![],
            description: "Audio output".to_string(),
            category: NodeCategory::IO,
        });
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
