use parking_lot::Mutex;
use std::sync::Arc;

use color_eyre::Result;
use glicol::Engine;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::components::Component;

#[derive(Clone)]
pub struct GraphComponent<const N: usize> {
    engine: Option<Arc<Mutex<Engine<N>>>>,
    title: String,
}

impl<const N: usize> GraphComponent<N> {
    pub fn new() -> Self {
        Self {
            engine: None,
            title: "Glicol Graph".to_string(),
        }
    }

    pub fn set_engine(&mut self, engine: Arc<Mutex<Engine<N>>>) {
        self.engine = Some(engine);
    }
}

impl<const N: usize> Component for GraphComponent<N> {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Graph content
            ])
            .split(area);

        // Draw the title block
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let title = Paragraph::new(self.title.clone()).block(title_block);
        f.render_widget(title, layout[0]);

        // Draw the graph content
        let content_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());

        let content = match &self.engine {
            Some(engine) => {
                // For now, just display a simple representation
                format!(
                    "Graph with {} nodes",
                    engine.lock().context.graph.node_count()
                )
            }
            None => "No graph loaded".to_string(),
        };

        let content = Paragraph::new(content).block(content_block);
        f.render_widget(content, layout[1]);

        Ok(())
    }
}
