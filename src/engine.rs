use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crossbeam::atomic::AtomicCell;

use crate::{clip::Clip, track::Tracks};

pub struct AudioEngine {
    pub bpm: AtomicU64,
    pub beats: AtomicU64,
    pub sample: AtomicU64,
    pub sample_rate: AtomicU64,
    pub metronome: AtomicBool,
    pub tracks: AtomicCell<Option<Tracks>>,
    pub recorded_clip: AtomicCell<Option<Clip>>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self {
            bpm: AtomicU64::new(120),
            beats: AtomicU64::new(16),
            sample: AtomicU64::new(0),
            sample_rate: AtomicU64::new(0),
            metronome: AtomicBool::new(false),
            tracks: AtomicCell::new(None),
            recorded_clip: AtomicCell::new(None),
        }
    }
}

impl AudioEngine {
    pub fn sample(&self) -> u64 {
        self.sample.load(Ordering::Acquire)
    }

    pub fn bpm(&self) -> u64 {
        self.bpm.load(Ordering::Acquire)
    }

    pub fn beats(&self) -> u64 {
        self.beats.load(Ordering::Acquire)
    }

    pub fn seconds(&self) -> f32 {
        self.sample() as f32 / self.sample_rate() as f32
    }

    pub fn metronome(&self) -> bool {
        self.metronome.load(Ordering::Acquire)
    }

    pub fn beat(&self) -> f32 {
        self.seconds() * self.bpm() as f32 / 60.0
    }

    pub fn sample_rate(&self) -> u64 {
        self.sample_rate.load(Ordering::Acquire)
    }

    pub fn take_tracks(&self) -> Option<Tracks> {
        self.tracks.take()
    }

    pub fn is_on_beat(&self) -> bool {
        self.beat() % 1.0 < 0.001
    }

    pub fn set_bpm(&self, bpm: u64) {
        self.bpm.store(bpm, Ordering::Release);
    }

    pub fn set_beats(&self, beats: u64) {
        self.beats.store(beats, Ordering::Release);
    }

    pub fn set_sample(&self, sample: u64) {
        self.sample.store(sample, Ordering::Release);
    }

    pub fn set_sample_rate(&self, sample_rate: u64) {
        self.sample_rate.store(sample_rate, Ordering::Release);
    }

    pub fn set_metronome(&self, metronome: bool) {
        self.metronome.store(metronome, Ordering::Release);
    }

    pub fn set_tracks(&self, tracks: Option<Tracks>) {
        self.tracks.store(tracks);
    }

    pub fn set_recorded_clip(&self, clip: Option<Clip>) {
        self.recorded_clip.store(clip);
    }

    pub fn should_loop(&self) -> bool {
        self.beat() >= self.beats() as f32
    }
}
