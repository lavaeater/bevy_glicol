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

use crate::graph::{Graph, Node, NodeCategory, NodeRegistry, ParameterType};

#[derive(Clone)]
pub struct GraphComponent {
    graph: Graph,
    selected_node_id: Option<String>,
    selected_param: Option<(String, usize)>, // (node_id, param_index)
    selected_category: Option<usize>,
    editing: bool,
    bpm: f32,
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
        }
    }

    pub fn get_ast(&self) -> HashMap<String, (Vec<String>, Vec<Vec<GlicolPara>>)> {
        self.graph.to_glicol_ast()
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
                Constraint::Length(3),    // Title
                Constraint::Length(3),    // Categories
                Constraint::Min(0),       // Main content
                Constraint::Length(10),   // Node details/parameters
            ])
            .split(area);

        self.draw_title(f, layout[0]);
        self.draw_categories(f, layout[1]);
        self.draw_nodes(f, layout[2]);
        self.draw_node_details(f, layout[3]);

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

        let items: Vec<ListItem> = self.graph
            .nodes()
            .iter()
            .map(|(id, node)| {
                let mut style = Style::default();
                let definition = self.graph.get_registry().get_definition(&node.node_type)
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
                if let Some(definition) = self.graph.get_registry().get_definition(&node.node_type) {
                    let mut text = Text::from(vec![
                        Line::from(vec![
                            Span::raw("Type: "),
                            Span::styled(&definition.name, Style::default().fg(Color::Cyan)),
                        ]),
                        Line::from(vec![
                            Span::raw("Category: "),
                            Span::styled(format!("{:?}", definition.category), Style::default().fg(Color::Yellow)),
                        ]),
                        Line::from(Span::raw("")),
                        Line::from(Span::styled("Parameters:", Style::default().add_modifier(Modifier::BOLD))),
                    ]);

                    for (i, param) in definition.parameters.iter().enumerate() {
                        let value = if i < node.parameters.len() {
                            node.parameters[i].to_string()
                        } else {
                            "[not set]".to_string()
                        };

                        let param_line = Line::from(vec![
                            Span::raw(format!("  {}: ", param.name)),
                            Span::styled(value, Style::default().fg(Color::Green)),
                            Span::raw(format!(" ({})", param.parameter_type)),
                        ]);
                        text.extend(Text::from(param_line));
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
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                ListItem::new(content).style(Style::default().fg(if i == 0 {
                    Color::Yellow
                } else {
                    Color::White
                }))
            })
            .collect();

        let list = List::new(items)
            .block(content_block)
            .highlight_style(Style::default().fg(Color::LightGreen));
        f.render_widget(list, layout[1]);

        Ok(())
    }
}
