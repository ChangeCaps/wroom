use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, EditMode};

impl App {
    pub fn render_device_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Length(20),
            ])
            .split(area);

        self.render_host_select(frame, chunks[0]);
        self.render_input_device_select(frame, chunks[1]);
        self.render_output_device_select(frame, chunks[2]);
    }

    pub fn render_host_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let host_names = self
            .audio
            .settings
            .host_names()
            .into_iter()
            .map(|name| ListItem::new(name));

        self.settings
            .host_state
            .select(Some(self.audio.settings.host_index()));

        let mut block = Block::default().title("Host 'h'").borders(Borders::ALL);

        if self.edit_mode == EditMode::Host {
            block = block.border_style(Style::default().fg(Color::Red));
        }

        let list = List::new(host_names.collect::<Vec<_>>())
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.settings.host_state);
    }

    pub fn render_input_device_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let input_device_names = self
            .audio
            .settings
            .input_device_names()
            .into_iter()
            .map(|name| ListItem::new(name));

        self.settings
            .input_device_state
            .select(self.audio.settings.input_device);

        let mut block = Block::default()
            .title("Input Device 'i'")
            .borders(Borders::ALL);

        if self.edit_mode == EditMode::InputDevice {
            block = block.border_style(Style::default().fg(Color::Red));
        }

        let list = List::new(input_device_names.collect::<Vec<_>>())
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.settings.input_device_state);
    }

    pub fn render_output_device_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let output_device_names = self
            .audio
            .settings
            .output_device_names()
            .into_iter()
            .map(|name| ListItem::new(name));

        self.settings
            .output_device_state
            .select(self.audio.settings.output_device);

        let mut block = Block::default()
            .title("Output Device 'o'")
            .borders(Borders::ALL);

        if self.edit_mode == EditMode::OutputDevice {
            block = block.border_style(Style::default().fg(Color::Red));
        }

        let list = List::new(output_device_names.collect::<Vec<_>>())
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.settings.output_device_state);
    }
}
