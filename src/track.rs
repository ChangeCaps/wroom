use cpal::SampleRate;
use deref_derive::{Deref, DerefMut};

use crate::clip::Clip;

#[derive(Clone)]
pub struct Track {
    pub clip: Option<Clip>,
    pub volume: u32,
    pub muted: bool,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            clip: None,
            volume: 100,
            muted: false,
        }
    }
}

impl Track {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn volume_factor(&self) -> f32 {
        if !self.muted {
            self.volume as f32 / 100.0
        } else {
            0.0
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    pub fn resample(&mut self, sample_rate: SampleRate) {
        if let Some(ref mut clip) = self.clip {
            *clip = clip.resample(sample_rate);
        }
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct Tracks {
    #[deref]
    pub tracks: Vec<Track>,
}

impl Default for Tracks {
    fn default() -> Self {
        Self {
            tracks: vec![Track::default(); 10],
        }
    }
}

impl Tracks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resample(&mut self, sample_rate: SampleRate) {
        for track in self.tracks.iter_mut() {
            track.resample(sample_rate);
        }
    }
}
