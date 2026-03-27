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

                        let current_idx = self.selected_param
                            .as_ref()
                            .map(|(_, idx)| *idx)
                            .unwrap_or(0);
                        
                        let next_idx = if current_idx + 1 >= definition.parameters.len() {
                            0
                        } else {
                            current_idx + 1
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

                        let current_idx = self.selected_param
                            .as_ref()
                            .map(|(_, idx)| *idx)
                            .unwrap_or(0);
                        
                        let prev_idx = if current_idx == 0 {
                            definition.parameters.len().saturating_sub(1)
                        } else {
                            current_idx - 1
                        };

                        self.selected_param = Some((node_id.clone(), prev_idx));
                    }
                }
                None
            },

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
    fn draw_title(&self, f: &mut Frame<'_>, area: Rect) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let title = format!("Glicol Graph (BPM: {})", self.bpm);
        let title = Paragraph::new(title).block(title_block);
        f.render_widget(title, area);
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
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Nodes")
            .style(Style::default());

        let items: Vec<ListItem> = self
            .graph
            .nodes()
            .iter()
            .map(|(id, node)| {
                let mut style = Style::default();
                // Verify the node type exists
                self.graph
                    .get_registry()
                    .get_definition(&node.node_type)
                    .unwrap_or_else(|| panic!("Unknown node type: {}", node.node_type));

                let prefix = if Some(id) == self.selected_node_id.as_ref() {
                    style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                    ">"
                } else {
                    " "
                };

                let inputs = if node.inputs.is_empty() {
                    "[no inputs]".to_string()
                } else {
                    format!("[{}]", node.inputs.join(", "))
                };

                let content = Line::from(vec![
                    Span::styled(format!("{} {}", prefix, id), style),
                    Span::raw(" ("),
                    Span::styled(&node.node_type, style.fg(Color::Cyan)),
                    Span::raw(") "),
                    Span::styled(inputs, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(list, area);
    }

    fn draw_node_details(&self, f: &mut Frame<'_>, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Node Details")
            .style(Style::default());

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
                        let value = if i < node.parameters.len() {
                            node.parameters[i].to_string()
                        } else {
                            "[not set]".to_string()
                        };

                        let (prefix, style) = if self.editing
                            && self
                                .selected_param
                                .as_ref()
                                .map(|(id, idx)| id == node_id && *idx == i)
                                .unwrap_or(false)
                        {
                            (
                                ">",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            (" ", Style::default().fg(Color::Green))
                        };

                        let param_line = Line::from(vec![
                            Span::raw(format!("{}  {}: ", prefix, param.name)),
                            Span::styled(value, style),
                            Span::raw(format!(" ({})", param.parameter_type)),
                        ]);
                        text.extend(Text::from(param_line));
                    }

                    if self.editing {
                        text.extend(Text::from(""));
                        text.extend(Text::from(Line::from(vec![
                            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                            Span::styled("Enter", Style::default().fg(Color::White)),
                            Span::styled(
                                " to edit parameter, ",
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled("Esc", Style::default().fg(Color::White)),
                            Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
                        ])));
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
