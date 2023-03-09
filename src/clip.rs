use std::sync::Arc;

use cpal::{ChannelCount, SampleRate};

#[derive(Clone, Debug)]
pub struct Clip {
    pub channels: ChannelCount,
    pub sample_rate: SampleRate,
    pub samples: Arc<[f32]>,
}

impl Clip {
    pub fn new(channels: ChannelCount, sample_rate: SampleRate, samples: Arc<[f32]>) -> Self {
        Self {
            channels,
            sample_rate,
            samples,
        }
    }

    pub fn frame_count(&self) -> u64 {
        self.samples.len() as u64 / self.channels as u64
    }

    pub fn duration(&self) -> f32 {
        self.frame_count() as f32 / self.sample_rate.0 as f32
    }

    fn fade_factor(&self, index: u64) -> f32 {
        let frame_count = self.frame_count();
        let fade_samples = frame_count * self.channels as u64 / 1000;
        if index < fade_samples {
            index as f32 / fade_samples as f32
        } else if index > frame_count - fade_samples {
            (frame_count - index) as f32 / fade_samples as f32
        } else {
            1.0
        }
    }

    pub fn sample(&self, index: u64, channel: u16) -> f32 {
        let sample = self
            .samples
            .get(index as usize * self.channels as usize + channel as usize)
            .copied()
            .unwrap_or(0.0);

        sample * self.fade_factor(index)
    }

    /// Returns the average of all channels at the given index.
    pub fn average_sample(&self, index: u64) -> f32 {
        let mut sum = 0.0;
        for channel in 0..self.channels {
            sum += self.sample(index, channel);
        }
        sum / self.channels as f32
    }

    pub fn add(&self, other: &Self, volume: f32) -> Self {
        assert_eq!(self.channels, other.channels);

        let mut samples = Vec::with_capacity(self.samples.len());

        for (sample, other_sample) in self.samples.iter().zip(other.samples.iter()) {
            samples.push(sample + other_sample * volume);
        }

        Self::new(self.channels, self.sample_rate, samples.into())
    }

    /// Creates a new clip with the given sample rate.
    /// The new clip will be resampled using linear interpolation.
    pub fn resample(&self, sample_rate: SampleRate) -> Self {
        let new_frame_count = self.frame_count() * sample_rate.0 as u64 / self.sample_rate.0 as u64;
        let mut samples = Vec::with_capacity(new_frame_count as usize * self.channels as usize);

        for frame in 0..new_frame_count {
            let point = frame as f64 * self.sample_rate.0 as f64 / sample_rate.0 as f64;
            let index = point.floor() as u64;
            let fraction = point - index as f64;

            for channel in 0..self.channels {
                let sample = self.sample(index, channel);
                let next_sample = self.sample(index + 1, channel);
                let new_sample = sample + (next_sample - sample) * fraction as f32;
                samples.push(new_sample);
            }
        }

        Self::new(self.channels, sample_rate, samples.into())
    }
}
