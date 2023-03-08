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

    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    pub fn duration(&self) -> f32 {
        self.frame_count() as f32 / self.sample_rate.0 as f32
    }

    fn fade_factor(&self, index: usize) -> f32 {
        let sample_count = self.frame_count();
        let fade_samples = sample_count * self.channels as usize / 1000;
        if index < fade_samples {
            index as f32 / fade_samples as f32
        } else if index > sample_count - fade_samples {
            (sample_count - index) as f32 / fade_samples as f32
        } else {
            1.0
        }
    }

    pub fn sample(&self, index: usize, channel: usize) -> f32 {
        let sample = self
            .samples
            .get(index * self.channels as usize + channel)
            .copied()
            .unwrap_or(0.0);

        sample * self.fade_factor(index)
    }

    /// Returns the average of all channels at the given index.
    pub fn average_sample(&self, index: usize) -> f32 {
        let mut sum = 0.0;
        for channel in 0..self.channels as usize {
            sum += self.sample(index, channel);
        }
        sum / self.channels as f32
    }

    /// Creates a new clip with the given sample rate.
    /// The new clip will be resampled using linear interpolation.
    pub fn resample(&self, sample_rate: SampleRate) -> Self {
        let new_frame_count =
            self.frame_count() * sample_rate.0 as usize / self.sample_rate.0 as usize;

        let mut samples = Vec::with_capacity(new_frame_count * self.channels as usize);

        for frame in 0..new_frame_count {
            let point = frame as f64 * self.sample_rate.0 as f64 / sample_rate.0 as f64;
            let index = point.floor() as usize;
            let fraction = point - index as f64;

            for channel in 0..self.channels as usize {
                let sample = self.sample(index, channel);
                let next_sample = self.sample(index + 1, channel);
                let new_sample = sample + (next_sample - sample) * fraction as f32;
                samples.push(new_sample);
            }
        }

        Self::new(self.channels, sample_rate, samples.into())
    }
}
