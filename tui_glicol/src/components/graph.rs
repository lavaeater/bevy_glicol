use crate::components::Component;
use color_eyre::Result;
use glicol_synth::GlicolPara;
use hashbrown::HashMap;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};
use uuid::Uuid;

use crate::graph::{Graph, Node, ParameterType};

use crate::action::Action;

#[derive(Clone)]
pub struct GraphComponent {
    graph: Graph,
    selected_node_id: Option<String>,
    selected_param: Option<(String, usize)>, // (node_id, param_index)
    selected_category: Option<usize>,
    editing: bool,
    input_buffer: String,
    param_input_active: bool,
    bpm: f32,
    error: Option<String>,
}

impl GraphComponent {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            selected_node_id: None,
            selected_param: None,
            selected_category: None,
            editing: false,
            input_buffer: String::new(),
            param_input_active: false,
            bpm: 120.0,
            error: None,
        }
    }

    pub fn handle_action(&mut self, action: Action) -> Result<Option<Action>, String> {
        let result = match action {
            Action::GraphNextNode => {
                let nodes: Vec<_> = self.graph.nodes().keys().cloned().collect();
                if nodes.is_empty() {
                    return Ok(None);
                }

                let current_idx = self
                    .selected_node_id
                    .as_ref()
                    .and_then(|id| nodes.iter().position(|n| n == id))
                    .unwrap_or(0);

                let next_idx = (current_idx + 1) % nodes.len();
                self.selected_node_id = Some(nodes[next_idx].clone());
                self.selected_param = None;
                None
            }

            Action::GraphPrevNode => {
                let nodes: Vec<_> = self.graph.nodes().keys().cloned().collect();
                if nodes.is_empty() {
                    return Ok(None);
                }

                let current_idx = self
                    .selected_node_id
                    .as_ref()
                    .and_then(|id| nodes.iter().position(|n| n == id))
                    .unwrap_or(0);

                let prev_idx = if current_idx == 0 {
                    nodes.len() - 1
                } else {
                    current_idx - 1
                };

                self.selected_node_id = Some(nodes[prev_idx].clone());
                self.selected_param = None;
                None
            }

            Action::GraphNextCategory => {
                let categories = 7; // Total number of categories
                let current = self.selected_category.unwrap_or(0);
                self.selected_category = Some((current + 1) % categories);
                None
            }

            Action::GraphPrevCategory => {
                let categories = 7; // Total number of categories
                let current = self.selected_category.unwrap_or(0);
                self.selected_category = Some(if current == 0 {
                    categories - 1
                } else {
                    current - 1
                });
                None
            }

            Action::GraphAddNode(node_type) => match self.add_node(&node_type, (0.0, 0.0)) {
                Ok(id) => {
                    self.selected_node_id = Some(id);
                    self.error = None;
                    None
                }
                Err(e) => {
                    return Ok(Some(Action::GraphShowError(e)));
                }
            },

            Action::GraphRemoveNode => {
                self.remove_selected_node();
                self.selected_param = None;
                None
            }

            Action::GraphConnectNodes(from_id, to_id) => {
                if let Err(e) = self.connect_nodes(&from_id, &to_id) {
                    return Ok(Some(Action::GraphShowError(e)));
                }
                None
            }

            Action::GraphEditParam(node_id, param_idx, value) => {
                // First get the node type
                let node_type = if let Some(node) = self.graph.nodes().get(&node_id) {
                    node.node_type.clone()
                } else {
                    return Ok(None);
                };

                // Then get the definition
                let definition = self.graph.get_registry()
                    .get_definition(&node_type)
                    .ok_or_else(|| format!("Unknown node type: {}", node_type))?;

                // Validate parameter index
                if param_idx >= definition.parameters.len() {
                    return Ok(Some(Action::GraphShowError(
                        format!("Invalid parameter index: {}", param_idx)
                    )));
                }

                // Parse the value based on parameter type
                let param = &definition.parameters[param_idx];
                let glicol_param = match param.parameter_type {
                    ParameterType::Number => {
                        match value.parse::<f32>() {
                            Ok(n) => GlicolPara::Number(n),
                            Err(_) => return Ok(Some(Action::GraphShowError(
                                format!("Invalid number: {}", value)
                            ))),
                        }
                    }
                    ParameterType::Reference => GlicolPara::Reference(value),
                    _ => return Ok(Some(Action::GraphShowError(
                        format!("Unsupported parameter type: {:?}", param.parameter_type)
                    ))),
                };

                // Now we can mutably borrow the node and update its parameters
                if let Some(node) = self.graph.nodes_mut().get_mut(&node_id) {
                    // Ensure we have enough space in parameters vector
                    while node.parameters.len() <= param_idx {
                        node.parameters.push(GlicolPara::Number(0.0));
                    }
                    node.parameters[param_idx] = glicol_param;
                    self.error = None;
                }
                None
            }

            Action::GraphStartEditing => {
                self.editing = true;
                // Auto-select param 0 so the cursor is visible immediately
                if let Some(node_id) = self.selected_node_id.clone() {
                    self.selected_param = Some((node_id, 0));
                }
                None
            },

            Action::GraphStopEditing => {
                self.editing = false;
                self.selected_param = None;
                None
            },

            Action::GraphNextParam => {
                if !self.editing {
                    return Ok(None);
                }

                if let Some(node_id) = self.selected_node_id.clone() {
                    if let Some(node) = self.graph.nodes().get(&node_id) {
                        let definition = self.graph.get_registry()
                            .get_definition(&node.node_type)
                            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?;

                        let next_idx = match self.selected_param.as_ref().map(|(_, i)| *i) {
                            None => 0,
                            Some(i) if i + 1 >= definition.parameters.len() => 0,
                            Some(i) => i + 1,
                        };

                        self.selected_param = Some((node_id.clone(), next_idx));
                    }
                }
                None
            },

            Action::GraphPrevParam => {
                if !self.editing {
                    return Ok(None);
                }

                if let Some(node_id) = self.selected_node_id.clone() {
                    if let Some(node) = self.graph.nodes().get(&node_id) {
                        let definition = self.graph.get_registry()
                            .get_definition(&node.node_type)
                            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?;

                        let prev_idx = match self.selected_param.as_ref().map(|(_, i)| *i) {
                            None | Some(0) => definition.parameters.len().saturating_sub(1),
                            Some(i) => i - 1,
                        };

                        self.selected_param = Some((node_id.clone(), prev_idx));
                    }
                }
                None
            },

            Action::GraphStartParamInput => {
                if !self.editing || self.selected_param.is_none() {
                    return Ok(Some(Action::GraphShowError(
                        "Select a parameter first (Tab/Shift-Tab)".to_string(),
                    )));
                }
                self.input_buffer = self.current_param_value_str();
                self.param_input_active = true;
                None
            }

            Action::GraphInputChar(c) => {
                if self.param_input_active {
                    self.input_buffer.push(c);
                }
                None
            }

            Action::GraphInputBackspace => {
                if self.param_input_active {
                    self.input_buffer.pop();
                }
                None
            }

            Action::GraphConfirmParam => {
                if self.param_input_active {
                    if let Some((node_id, param_idx)) = self.selected_param.clone() {
                        let value = self.input_buffer.clone();
                        self.input_buffer.clear();
                        self.param_input_active = false;
                        return Ok(Some(Action::GraphEditParam(node_id, param_idx, value)));
                    }
                }
                self.param_input_active = false;
                self.input_buffer.clear();
                None
            }

            Action::GraphCancelParam => {
                self.input_buffer.clear();
                self.param_input_active = false;
                None
            }

            Action::GraphShowError(error) => {
                self.error = Some(error);
                None
            },

            Action::GraphClearError => {
                self.error = None;
                None
            },

            _ => None,
        };
        Ok(result)
    }

    fn current_param_value_str(&self) -> String {
        let Some((node_id, param_idx)) = &self.selected_param else {
            return String::new();
        };
        let Some(node) = self.graph.nodes().get(node_id) else {
            return String::new();
        };
        match node.parameters.get(*param_idx) {
            Some(GlicolPara::Number(n)) => {
                if n.fract() == 0.0 && n.abs() < 1e9 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Some(GlicolPara::Reference(r)) => r.clone(),
            Some(GlicolPara::SampleSymbol(s)) => s.clone(),
            _ => String::new(),
        }
    }

    pub fn get_ast(&self) -> HashMap<String, (Vec<String>, Vec<Vec<GlicolPara>>)> {
        self.graph.to_glicol_ast()
    }

    pub fn get_glicol_code(&self) -> String {
        self.graph.to_glicol_code()
    }

    pub fn update_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
    }

    pub fn add_node(&mut self, node_type: &str, position: (f32, f32)) -> Result<String, String> {
        let id = format!("node_{}", Uuid::new_v4());
        let node = Node::new(id.clone(), node_type.to_string(), position);
        self.graph.add_node(node)?;
        Ok(id)
    }

    pub fn remove_selected_node(&mut self) {
        if let Some(id) = self.selected_node_id.take() {
            self.graph.remove_node(&id);
        }
    }

    pub fn connect_nodes(&mut self, from_id: &str, to_id: &str) -> Result<(), String> {
        self.graph.connect(from_id, to_id)
    }
}

impl Component for GraphComponent {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Categories
                Constraint::Min(0),     // Main content
                Constraint::Length(10), // Node details/parameters
                Constraint::Length(1),  // Error display
            ])
            .split(area);

        self.draw_title(f, layout[0]);
        self.draw_categories(f, layout[1]);
        self.draw_nodes(f, layout[2]);
        self.draw_node_details(f, layout[3]);
        self.draw_error(f, layout[4]);

        Ok(())
    }
}

impl GraphComponent {
    fn visual_mode(&self) -> (&'static str, Color) {
        if self.param_input_active {
            ("TYPING", Color::Green)
        } else if self.editing {
            ("EDITING", Color::Yellow)
        } else {
            ("NAVIGATE", Color::Cyan)
        }
    }

    fn draw_title(&self, f: &mut Frame<'_>, area: Rect) {
        let (mode_label, mode_color) = self.visual_mode();
        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(mode_color));

        let title = Line::from(vec![
            Span::raw(" Glicol Graph  "),
            Span::styled(
                format!(" {} ", mode_label),
                Style::default()
                    .fg(Color::Black)
                    .bg(mode_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  BPM: {}", self.bpm),
                Style::default().fg(Color::White),
            ),
        ]);
        let paragraph = Paragraph::new(title).block(title_block);
        f.render_widget(paragraph, area);
    }

    fn draw_categories(&self, f: &mut Frame<'_>, area: Rect) {
        let categories = vec![
            "Oscillators",
            "Filters",
            "Effects",
            "Modulators",
            "Math",
            "IO",
            "Utility",
        ];

        let tabs = Tabs::new(categories)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow))
            .select(self.selected_category.unwrap_or(0));

        f.render_widget(tabs, area);
    }

    fn draw_nodes(&self, f: &mut Frame<'_>, area: Rect) {
        let (_, mode_color) = self.visual_mode();
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Nodes  [j/k to move]")
            .border_style(Style::default().fg(mode_color));

        // Stable sort by ID so the list order doesn't jump around
        let mut sorted: Vec<(&String, &crate::graph::Node)> =
            self.graph.nodes().iter().collect();
        sorted.sort_by_key(|(id, _)| id.as_str());

        // Count how many nodes of each type exist, for disambiguation labels
        let mut type_totals: HashMap<&str, usize> = HashMap::new();
        for (_, node) in &sorted {
            *type_totals.entry(node.node_type.as_str()).or_default() += 1;
        }
        let mut type_seen: HashMap<&str, usize> = HashMap::new();

        let items: Vec<ListItem> = sorted
            .iter()
            .map(|(id, node)| {
                let is_selected = Some(*id) == self.selected_node_id.as_ref();

                // Build a short display label: type name, disambiguated if needed
                let total = type_totals[node.node_type.as_str()];
                let n = {
                    let seen = type_seen.entry(node.node_type.as_str()).or_default();
                    *seen += 1;
                    *seen
                };
                let label = if total > 1 {
                    format!("{} #{}", node.node_type, n)
                } else {
                    node.node_type.clone()
                };

                // Show input connections as node types, not raw IDs
                let input_types: Vec<String> = node
                    .inputs
                    .iter()
                    .map(|input_id| {
                        self.graph
                            .nodes()
                            .get(input_id)
                            .map(|n| n.node_type.clone())
                            .unwrap_or_else(|| "?".to_string())
                    })
                    .collect();
                let inputs_str = if input_types.is_empty() {
                    String::new()
                } else {
                    format!("  ← {}", input_types.join(", "))
                };

                let line = if is_selected {
                    Line::from(Span::styled(
                        format!("  ► {}{}", label, inputs_str),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(vec![
                        Span::raw(format!("    {}", label)),
                        Span::styled(inputs_str, Style::default().fg(Color::DarkGray)),
                    ])
                };
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    fn draw_node_details(&self, f: &mut Frame<'_>, area: Rect) {
        let (mode_label, mode_color) = self.visual_mode();
        let details_title = if self.editing {
            format!("Node Details  [ {} ]", mode_label)
        } else {
            "Node Details  [ Enter to edit ]".to_string()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(details_title)
            .border_style(Style::default().fg(mode_color));

        let content = if let Some(node_id) = &self.selected_node_id {
            if let Some(node) = self.graph.nodes().get(node_id) {
                if let Some(definition) = self.graph.get_registry().get_definition(&node.node_type)
                {
                    let mut text = Text::from(vec![
                        Line::from(vec![
                            Span::raw("Type: "),
                            Span::styled(&definition.name, Style::default().fg(Color::Cyan)),
                        ]),
                        Line::from(vec![
                            Span::raw("Category: "),
                            Span::styled(
                                format!("{:?}", definition.category),
                                Style::default().fg(Color::Yellow),
                            ),
                        ]),
                        Line::from(Span::raw("")),
                        Line::from(Span::styled(
                            "Parameters:",
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                    ]);

                    for (i, param) in definition.parameters.iter().enumerate() {
                        let is_selected = self.editing
                            && self
                                .selected_param
                                .as_ref()
                                .map(|(id, idx)| id == node_id && *idx == i)
                                .unwrap_or(false);
                        let is_typing = is_selected && self.param_input_active;

                        let (prefix, label_style) = if is_selected {
                            (">", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        } else {
                            (" ", Style::default().fg(Color::Green))
                        };

                        let value_span = if is_typing {
                            // Show input buffer with blinking cursor
                            Span::styled(
                                format!("{}|", self.input_buffer),
                                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                            )
                        } else {
                            let value = if i < node.parameters.len() {
                                node.parameters[i].to_string()
                            } else {
                                "[not set]".to_string()
                            };
                            Span::styled(value, label_style)
                        };

                        let param_line = Line::from(vec![
                            Span::raw(format!("{}  {}: ", prefix, param.name)),
                            value_span,
                            Span::raw(format!(" ({})", param.parameter_type)),
                        ]);
                        text.extend(Text::from(param_line));
                    }

                    if self.editing {
                        text.extend(Text::from(""));
                        if self.param_input_active {
                            text.extend(Text::from(Line::from(vec![
                                Span::styled("Type value, ", Style::default().fg(Color::DarkGray)),
                                Span::styled("Enter", Style::default().fg(Color::White)),
                                Span::styled(" to confirm, ", Style::default().fg(Color::DarkGray)),
                                Span::styled("Esc", Style::default().fg(Color::White)),
                                Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
                            ])));
                        } else {
                            text.extend(Text::from(Line::from(vec![
                                Span::styled("Tab", Style::default().fg(Color::White)),
                                Span::styled(
                                    " to select param, ",
                                    Style::default().fg(Color::DarkGray),
                                ),
                                Span::styled("Enter", Style::default().fg(Color::White)),
                                Span::styled(" to edit, ", Style::default().fg(Color::DarkGray)),
                                Span::styled("Esc", Style::default().fg(Color::White)),
                                Span::styled(" to stop editing", Style::default().fg(Color::DarkGray)),
                            ])));
                        }
                    }

                    text
                } else {
                    Text::from("Unknown node type")
                }
            } else {
                Text::from("Node not found")
            }
        } else {
            Text::from("No node selected")
        };

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    fn draw_error(&self, f: &mut Frame<'_>, area: Rect) {
        if let Some(error) = &self.error {
            let text = Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::styled(error, Style::default().fg(Color::Red)),
            ]);
            let paragraph = Paragraph::new(text)
                .style(Style::default())
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }
}
