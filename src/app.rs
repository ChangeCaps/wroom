use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::DOT,
    text::Spans,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};

use crate::audio::Audio;

#[repr(i32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Play = 0,
    Settings = 1,
}

impl Tab {
    pub const COUNT: usize = 2;

    pub fn name(&self) -> &'static str {
        match self {
            Tab::Play => "Play",
            Tab::Settings => "Settings",
        }
    }

    pub fn rotate(&mut self, offset: i32) {
        let index = (*self as i32 + offset) % Self::COUNT as i32;
        let tab = unsafe { std::mem::transmute::<i32, Tab>(index) };
        *self = tab;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditMode {
    #[default]
    None,
    Host,
    InputDevice,
    OutputDevice,
    SampleRate,
    BufferSize,
    Delay,
    Bpm,
    Beats,
    RecordTrack,
    RemoveTrack,
    TrackVolume(Option<usize>),
}

#[derive(Default)]
pub struct Settings {
    pub host_state: ListState,
    pub input_device_state: ListState,
    pub output_device_state: ListState,
    pub sample_rate_state: ListState,
    pub buffer_size_state: ListState,
}

pub struct App {
    pub running: bool,
    pub frame_rate: Duration,
    pub audio: Audio,
    pub tab: Tab,
    pub edit_mode: EditMode,
    pub settings: Settings,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            frame_rate: Duration::from_millis(1000 / 60),
            audio: Audio::new(),
            tab: Tab::Play,
            edit_mode: EditMode::default(),
            settings: Settings::default(),
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        self.running = true;
        let mut last_frame = Instant::now();

        loop {
            let timeout = self.frame_rate.saturating_sub(last_frame.elapsed());
            if crossterm::event::poll(timeout)? {
                self.event(crossterm::event::read()?);
            }

            if last_frame.elapsed() >= self.frame_rate {
                terminal.draw(|frame| self.render(frame))?;
                last_frame = Instant::now();
            }

            if !self.running {
                break Ok(());
            }
        }
    }

    pub fn event(&mut self, event: Event) {
        match event {
            Event::Key(event) => self.key(event),
            _ => {}
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::F(5) => {
                let _ = self.audio.launch_streams();
            }
            KeyCode::Esc => self.edit_mode = EditMode::None,
            KeyCode::Tab => {
                self.tab.rotate(1);
                self.edit_mode = EditMode::None;
            }
            _ => {}
        }

        match self.tab {
            Tab::Play => self.play_key(key),
            Tab::Settings => self.settings_key(key),
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('j') => self.rotate(1),
            KeyCode::Down | KeyCode::Char('k') => self.rotate(-1),
            _ => {}
        }
    }

    // called when a key is pressed in the play tab
    pub fn play_key(&mut self, key: KeyEvent) {
        for i in 0..self.audio.tracks.len() {
            let Some(digit) = char::from_digit((i as u32 + 1) % 10, 10) else {
                continue;
            };

            if key.code == KeyCode::Char(digit) {
                self.track_key(i);
            }
        }

        match key.code {
            KeyCode::Char('b') => self.edit_mode = EditMode::Bpm,
            KeyCode::Char('B') => self.edit_mode = EditMode::Beats,
            KeyCode::Char('r') => self.edit_mode = EditMode::RecordTrack,
            KeyCode::Char('R') => self.edit_mode = EditMode::RemoveTrack,
            KeyCode::Char('v') => self.edit_mode = EditMode::TrackVolume(None),
            KeyCode::Char('M') => {
                let metronome = self.audio.engine.metronome();
                self.audio.engine.set_metronome(!metronome);
                self.audio.update_tracks();
            }
            _ => {}
        }
    }

    // called when a track key is pressed
    pub fn track_key(&mut self, index: usize) {
        match self.edit_mode {
            EditMode::TrackVolume(_) => self.edit_mode = EditMode::TrackVolume(Some(index)),
            EditMode::RemoveTrack => {
                self.audio.tracks[index].clip = None;
                self.audio.update_tracks();
            }
            EditMode::RecordTrack => {
                if let Some(clip) = self.audio.get_clip() {
                    if let Some(ref mut current_clip) = self.audio.tracks[index].clip {
                        let new_clip = current_clip.add(&clip, 1.0);
                        self.audio.tracks[index].clip = Some(new_clip);
                    } else {
                        self.audio.tracks[index].clip = Some(clip);
                    }

                    self.audio.update_tracks();
                }

                self.edit_mode = EditMode::None;
            }
            _ => {
                self.audio.tracks[index].toggle_mute();
                self.audio.update_tracks();
            }
        }
    }

    // called when a key is pressed in the settings tab
    pub fn settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('h') => self.edit_mode = EditMode::Host,
            KeyCode::Char('i') => self.edit_mode = EditMode::InputDevice,
            KeyCode::Char('o') => self.edit_mode = EditMode::OutputDevice,
            KeyCode::Char('r') => self.edit_mode = EditMode::SampleRate,
            KeyCode::Char('b') => self.edit_mode = EditMode::BufferSize,
            KeyCode::Char('d') => self.edit_mode = EditMode::Delay,
            _ => {}
        }
    }

    // called when a key is pressed to rotate the value of the current edit mode
    pub fn rotate(&mut self, offset: i32) {
        match self.edit_mode {
            EditMode::Host => self.audio.settings.rotate_host(offset),
            EditMode::InputDevice => self.audio.settings.rotate_input_device(offset),
            EditMode::OutputDevice => self.audio.settings.rotate_output_device(offset),
            EditMode::SampleRate => self.audio.settings.rotate_sample_rate(offset),
            EditMode::BufferSize => self.audio.settings.rotate_buffer_size(offset),
            EditMode::Delay => {
                self.audio.settings.delay =
                    (self.audio.settings.delay as i32 - offset).max(0) as u32;
            }
            EditMode::Bpm => {
                let bpm = self.audio.engine.bpm();
                let new = (bpm as i32 - offset).max(0) as u64;
                self.audio.engine.set_bpm(new);
            }
            EditMode::Beats => {
                let beats = self.audio.engine.beats();
                let new = (beats as i32 - offset).max(0) as u64;
                self.audio.engine.set_beats(new);
            }
            EditMode::TrackVolume(Some(index)) => {
                let track = &mut self.audio.tracks[index];
                track.volume = (track.volume as i32 - offset * 5).clamp(0, 200) as u32;
                self.audio.update_tracks();
            }
            _ => {}
        }

        if self.tab == Tab::Settings && self.edit_mode != EditMode::None {
            if let Some(sample_rate) = self.audio.settings.get_sample_rate() {
                self.audio.tracks.resample(sample_rate);
            }

            self.audio.launch_streams();
        }
    }

    pub fn render<B: Backend>(&mut self, frame: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Percentage(100)])
            .split(frame.size());

        self.render_tab_select(frame, chunks[0]);
        self.render_main_tab(frame, chunks[1]);
    }

    pub fn render_tab_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let tabs = Tabs::new(vec![Spans::from("Play"), Spans::from("Settings")])
            .select(self.tab as usize)
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(DOT);

        frame.render_widget(tabs, area);
    }

    pub fn render_main_tab<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.tab.name());

        frame.render_widget(block, area);

        match self.tab {
            Tab::Play => self.render_play(frame, area),
            Tab::Settings => self.render_settings(frame, area),
        }
    }

    pub fn render_settings<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .margin(1)
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(16), Constraint::Max(16)])
            .split(area);

        self.render_device_select(frame, chunks[0]);
        self.render_device_config(frame, chunks[1]);
    }

    pub fn render_device_config<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Min(1),
            ])
            .split(area);

        self.render_sample_rate_select(frame, chunks[0]);
        self.render_buffer_size_select(frame, chunks[1]);
        self.render_delay_select(frame, chunks[2]);
        self.render_error(frame, chunks[3]);
    }

    pub fn render_sample_rate_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let sample_rates = self
            .audio
            .settings
            .sample_rates
            .iter()
            .map(|rate| ListItem::new(format!("{}", rate.0)));

        self.settings
            .sample_rate_state
            .select(self.audio.settings.sample_rate);

        let mut block = Block::default()
            .title("Sample Rate 'r'")
            .borders(Borders::ALL);

        if self.edit_mode == EditMode::SampleRate {
            block = block.border_style(Style::default().fg(Color::Red));
        }

        let list = List::new(sample_rates.collect::<Vec<_>>())
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.settings.sample_rate_state);
    }

    pub fn render_buffer_size_select<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let buffer_sizes = self
            .audio
            .settings
            .buffer_sizes
            .iter()
            .map(|size| ListItem::new(format!("{}", size)));

        self.settings
            .buffer_size_state
            .select(self.audio.settings.buffer_size);

        let mut block = Block::default()
            .title("Buffer Size 'b'")
            .borders(Borders::ALL);

        if self.edit_mode == EditMode::BufferSize {
            block = block.border_style(Style::default().fg(Color::Red));
        }

        let list = List::new(buffer_sizes.collect::<Vec<_>>())
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, area, &mut self.settings.buffer_size_state);
    }

    pub fn render_delay_select<B: Backend>(&mut self, frame: &mut Frame<B>, mut area: Rect) {
        let mut block = Block::default().borders(Borders::ALL).title("Delay 'd'");

        if self.edit_mode == EditMode::Delay {
            block = block.border_style(Style::default().fg(Color::Red));
        }

        let paragraph = Paragraph::new(format!("{}ms", self.audio.settings.delay))
            .alignment(Alignment::Right)
            .block(block);

        area.height = 3;
        frame.render_widget(paragraph, area);
    }

    pub fn render_error<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        let Some(ref error) = self.audio.error else {
            return;
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Error")
            .border_style(Style::default().fg(Color::Red));

        let paragraph = Paragraph::new(error.to_string())
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);
    }
}
