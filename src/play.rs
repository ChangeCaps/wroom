use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Spans,
    widgets::{BarChart, Block, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, EditMode},
    track::Track,
};

const RAINBOW: [Color; 6] = [
    Color::Red,
    Color::Yellow,
    Color::Green,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
];

impl App {
    pub fn render_play<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .margin(1)
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(7), Constraint::Min(1)])
            .split(area);

        self.render_beat(frame, chunks[0]);
        self.render_right(frame, chunks[1]);
    }

    pub fn render_right<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_tracks(frame, chunks[0]);
        self.render_bottom(frame, chunks[1]);
    }

    pub fn render_bottom<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(1)])
            .split(area);

        self.render_play_settings(frame, chunks[0]);
    }

    pub fn render_play_settings<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Settings");

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .margin(1)
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        self.render_bpm_select(frame, chunks[0]);
        self.render_beats_select(frame, chunks[1]);
        self.render_metronome_select(frame, chunks[2]);
    }

    pub fn render_bpm_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let mut block = Block::default().borders(Borders::ALL).title("BPM 'b'");

        if self.edit_mode == EditMode::Bpm {
            block = block.style(Style::default().fg(Color::Red));
        }

        let text = format!("{}", self.audio.engine.bpm());
        let paragraph = Paragraph::new(Spans::from(text)).block(block);
        frame.render_widget(paragraph, area);
    }

    pub fn render_beats_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let mut block = Block::default().borders(Borders::ALL).title("Beats 'B'");

        if self.edit_mode == EditMode::Beats {
            block = block.style(Style::default().fg(Color::Red));
        }

        let text = format!("{}", self.audio.engine.beats());
        let paragraph = Paragraph::new(Spans::from(text)).block(block);
        frame.render_widget(paragraph, area);
    }

    pub fn render_metronome_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Metronome 'M'");

        let text = if self.audio.engine.metronome() {
            "On"
        } else {
            "Off"
        };

        let paragraph = Paragraph::new(Spans::from(text)).block(block);

        frame.render_widget(paragraph, area);
    }

    pub fn render_beat<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let half_beat = (self.audio.engine.beat() * 2.0).round() as usize;
        let color = RAINBOW[half_beat % RAINBOW.len()];

        let beat = self.audio.engine.beat().round() as u64;
        let data = [("", beat)];
        let bar = BarChart::default()
            .data(&data)
            .bar_width(6)
            .bar_gap(0)
            .max(self.audio.engine.beats())
            .bar_style(Style::default().fg(color))
            .value_style(Style::default().fg(Color::White).bg(color));

        frame.render_widget(bar, area);
    }

    pub fn render_tracks<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Tracks");
        frame.render_widget(block, area);

        let mut constraints = vec![Constraint::Length(6); self.audio.tracks.len()];
        constraints.push(Constraint::Length(30));
        constraints.push(Constraint::Min(1));

        let chunks = Layout::default()
            .margin(1)
            .horizontal_margin(2)
            .direction(Direction::Horizontal)
            .constraints(constraints.clone())
            .split(area);

        for (i, track) in self.audio.tracks.iter().enumerate() {
            let color = RAINBOW[i % RAINBOW.len()];
            self.render_track(frame, chunks[i], i, track, color);
        }

        self.render_track_edit(frame, chunks[chunks.len() - 2]);
        self.render_track_info(frame, chunks[chunks.len() - 1]);
    }

    pub fn render_track_edit<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Edit Tracks");
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .margin(1)
            .horizontal_margin(2)
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_track_volume(frame, chunks[0]);
        self.render_track_record(frame, chunks[1]);
        self.render_track_remove(frame, chunks[2]);
    }

    pub fn render_track_volume<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let mut volume = Paragraph::new("volume 'v'");

        if matches!(self.edit_mode, EditMode::TrackVolume(_)) {
            volume = volume.style(Style::default().fg(Color::Red));
        }

        frame.render_widget(volume, area);
    }

    pub fn render_track_record<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let mut record = Paragraph::new("record 'r'");

        if matches!(self.edit_mode, EditMode::RecordTrack) {
            record = record.style(Style::default().fg(Color::Red));
        }

        frame.render_widget(record, area);
    }

    pub fn render_track_remove<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let mut remove = Paragraph::new("remove 'R'");

        if matches!(self.edit_mode, EditMode::RemoveTrack) {
            remove = remove.style(Style::default().fg(Color::Red));
        }

        frame.render_widget(remove, area);
    }

    pub fn render_track_info<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Info");
        frame.render_widget(block, area);
    }

    pub fn render_track<B: Backend>(
        &self,
        frame: &mut Frame<B>,
        mut area: Rect,
        index: usize,
        track: &Track,
        color: Color,
    ) {
        area.width = 6;

        let block = Block::default()
            .borders(Borders::ALL)
            .title(((index + 1) % 10).to_string());

        frame.render_widget(block, area);

        let chunks = Layout::default()
            .margin(1)
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .split(area);

        let sample_index = self.audio.engine.sample();

        if let Some(ref clip) = track.clip {
            let mut sample = 0.0f32;
            for i in 0..512 {
                let s = clip.average_sample(sample_index as usize + i);
                sample = sample.max(s.abs());
            }

            sample *= track.volume_factor();

            let data = [("", (sample * 200.0) as u64)];
            let bar = BarChart::default()
                .data(&data)
                .bar_width(3)
                .bar_gap(0)
                .max(100)
                .bar_style(Style::default().fg(color))
                .value_style(Style::default().fg(Color::White).bg(color));

            frame.render_widget(bar, chunks[0]);
        }

        let mut volume_color = Color::White;

        if track.muted {
            volume_color = Color::Gray;
        }

        if track.clip.is_none() {
            volume_color = Color::DarkGray;
        }

        if self.edit_mode == EditMode::TrackVolume(Some(index)) {
            volume_color = Color::Red;
        }

        let data = [("", track.volume as u64)];
        let bar = BarChart::default()
            .data(&data)
            .bar_width(1)
            .bar_gap(0)
            .max(200)
            .bar_style(Style::default().fg(volume_color));

        frame.render_widget(bar, chunks[1]);
    }
}
