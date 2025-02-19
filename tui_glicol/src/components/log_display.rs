use std::collections::VecDeque;
use color_eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use crate::action::Action;
use super::Component;

const MAX_LOGS: usize = 5;

#[derive(Default)]
pub struct LogDisplay {
    logs: VecDeque<(String, Style)>,
}

impl LogDisplay {
    pub fn add_error(&mut self, message: String) {
        self.logs.push_back((message, Style::default().fg(Color::Red)));
        if self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }

    #[allow(unused)]
    pub fn add_info(&mut self, message: String) {
        self.logs.push_back((message, Style::default().fg(Color::Green)));
        if self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }
}

impl Component for LogDisplay {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let block = Block::default()
            .title("Logs")
            .borders(Borders::ALL);
        
        let inner_area = block.inner(area);
        
        let text: Vec<Line> = self.logs
            .iter()
            .map(|(msg, style)| Line::styled(msg.clone(), *style))
            .collect();

        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Left),
            inner_area,
        );
        
        Ok(())
    }

    #[allow(unused)]
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
}
