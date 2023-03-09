use std::{
    mem,
    ops::Range,
    sync::{atomic::Ordering, Arc},
};

use anyhow::anyhow;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, Host, HostId, InputCallbackInfo, OutputCallbackInfo, SampleRate, Stream,
    StreamConfig, SupportedBufferSize,
};
use ringbuf::HeapRb;

use crate::{clip::Clip, engine::AudioEngine, gag, track::Tracks};

fn device_eq(a: &Device, b: &Device) -> bool {
    if let (Ok(a_name), Ok(b_name)) = (a.name(), b.name()) {
        a_name == b_name
    } else {
        false
    }
}

fn device_index(devices: &[Device], device: &Option<Device>) -> Option<usize> {
    let device = device.as_ref()?;
    devices.iter().position(|d| device_eq(d, device))
}

fn input_sample_rates(device: &Device) -> Vec<Range<u32>> {
    if let Ok(configs) = device.supported_input_configs() {
        configs
            .map(|c| c.min_sample_rate().0..c.max_sample_rate().0)
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    }
}

fn output_sample_rates(device: &Device) -> Vec<Range<u32>> {
    if let Ok(configs) = device.supported_output_configs() {
        configs
            .map(|c| c.min_sample_rate().0..c.max_sample_rate().0)
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    }
}

fn input_buffer_sizes(device: &Device) -> Vec<SupportedBufferSize> {
    if let Ok(configs) = device.supported_input_configs() {
        configs
            .map(|c| c.buffer_size().clone())
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    }
}

fn output_buffer_sizes(device: &Device) -> Vec<SupportedBufferSize> {
    if let Ok(configs) = device.supported_output_configs() {
        configs
            .map(|c| c.buffer_size().clone())
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    }
}

const SAMPLE_RATES: &[u32] = &[44100, 48000, 88200, 96000, 176400, 192000];
const BUFFER_SIZES: &[u32] = &[32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384];

fn sample_rate_supported(sample_rates: &[Range<u32>], sample_rate: u32) -> bool {
    sample_rates
        .iter()
        .any(|range| range.contains(&sample_rate))
}

fn buffer_size_supported(buffer_sizes: &[SupportedBufferSize], buffer_size: u32) -> bool {
    buffer_sizes.iter().any(|size| match size {
        SupportedBufferSize::Range { min, max } => {
            buffer_size >= *min as u32 && buffer_size <= *max as u32
        }
        SupportedBufferSize::Unknown => true,
    })
}

fn sample_rates(input_device: Option<&Device>, output_device: Option<&Device>) -> Vec<SampleRate> {
    let mut sample_rates = Vec::new();

    match (input_device, output_device) {
        (Some(input_device), Some(output_device)) => {
            let input = input_sample_rates(input_device);
            let output = output_sample_rates(output_device);

            for &sample_rate in SAMPLE_RATES {
                let input_supported = sample_rate_supported(&input, sample_rate);
                let output_supported = sample_rate_supported(&output, sample_rate);

                if input_supported && output_supported {
                    sample_rates.push(SampleRate(sample_rate));
                }
            }
        }
        (Some(input_device), None) => {
            let input = input_sample_rates(input_device);

            for &sample_rate in SAMPLE_RATES {
                if sample_rate_supported(&input, sample_rate) {
                    sample_rates.push(SampleRate(sample_rate));
                }
            }
        }
        (None, Some(output_device)) => {
            let output = output_sample_rates(output_device);

            for &sample_rate in SAMPLE_RATES {
                if sample_rate_supported(&output, sample_rate) {
                    sample_rates.push(SampleRate(sample_rate));
                }
            }
        }
        (None, None) => (),
    }

    sample_rates.sort();
    sample_rates.dedup();
    sample_rates
}

fn buffer_sizes(input_device: Option<&Device>, output_device: Option<&Device>) -> Vec<u32> {
    let mut buffer_sizes = Vec::new();

    match (input_device, output_device) {
        (Some(input_device), Some(output_device)) => {
            let input = input_buffer_sizes(input_device);
            let output = output_buffer_sizes(output_device);

            for &buffer_size in BUFFER_SIZES {
                let input_supported = buffer_size_supported(&input, buffer_size);
                let output_supported = buffer_size_supported(&output, buffer_size);

                if input_supported && output_supported {
                    buffer_sizes.push(buffer_size);
                }
            }
        }
        (Some(input_device), None) => {
            let input = input_buffer_sizes(input_device);

            for &buffer_size in BUFFER_SIZES {
                if buffer_size_supported(&input, buffer_size) {
                    buffer_sizes.push(buffer_size);
                }
            }
        }
        (None, Some(output_device)) => {
            let output = output_buffer_sizes(output_device);

            for &buffer_size in BUFFER_SIZES {
                if buffer_size_supported(&output, buffer_size) {
                    buffer_sizes.push(buffer_size);
                }
            }
        }
        (None, None) => (),
    }

    buffer_sizes.sort();
    buffer_sizes.dedup();
    buffer_sizes
}

pub struct AudioSettings {
    pub available_hosts: Vec<HostId>,
    pub host: Host,
    pub input_devices: Vec<Device>,
    pub output_devices: Vec<Device>,
    pub input_device: Option<usize>,
    pub output_device: Option<usize>,
    pub sample_rates: Vec<SampleRate>,
    pub sample_rate: Option<usize>,
    pub buffer_sizes: Vec<u32>,
    pub buffer_size: Option<usize>,
    pub delay: u32,
}

impl AudioSettings {
    pub fn new() -> Self {
        gag!();

        let available_hosts = cpal::available_hosts();
        let host = cpal::default_host();

        Self {
            available_hosts,
            host,
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            input_device: None,
            output_device: None,
            sample_rates: Vec::new(),
            sample_rate: None,
            buffer_sizes: Vec::new(),
            buffer_size: None,
            delay: 15,
        }
    }

    pub fn query_devices(&mut self) {
        gag!();

        self.input_devices = self.host.input_devices().unwrap().collect();
        self.output_devices = self.host.output_devices().unwrap().collect();

        self.query_default_devices();
    }

    pub fn query_default_devices(&mut self) {
        gag!();

        self.input_device = device_index(&self.input_devices, &self.host.default_input_device());
        self.output_device = device_index(&self.output_devices, &self.host.default_output_device());

        self.query_sample_rates();
        self.query_buffer_sizes();
    }

    pub fn query_sample_rates(&mut self) {
        self.sample_rates = sample_rates(
            self.input_device.map(|i| &self.input_devices[i]),
            self.output_device.map(|i| &self.output_devices[i]),
        );

        self.query_default_sample_rate();
    }

    pub fn query_default_sample_rate(&mut self) {
        if let Some(index) = self.sample_rates.iter().position(|s| s.0 == 48000) {
            self.sample_rate = Some(index);
        } else if !self.sample_rates.is_empty() {
            self.sample_rate = Some(0);
        } else {
            self.sample_rate = None;
        }
    }

    pub fn query_buffer_sizes(&mut self) {
        self.buffer_sizes = buffer_sizes(
            self.input_device.map(|i| &self.input_devices[i]),
            self.output_device.map(|i| &self.output_devices[i]),
        );

        self.query_default_buffer_size();
    }

    pub fn query_default_buffer_size(&mut self) {
        if let Some(index) = self.buffer_sizes.iter().position(|s| *s == 128) {
            self.buffer_size = Some(index);
        } else if !self.buffer_sizes.is_empty() {
            self.buffer_size = Some(0);
        } else {
            self.buffer_size = None;
        }
    }

    pub fn host_names(&self) -> Vec<&'static str> {
        self.available_hosts.iter().map(|id| id.name()).collect()
    }

    pub fn host_index(&self) -> usize {
        self.available_hosts
            .iter()
            .position(|id| id == &self.host.id())
            .unwrap()
    }

    pub fn rotate_host(&mut self, offset: i32) {
        gag!();

        let mut index = self.host_index() as i32 + offset;
        index = index.rem_euclid(self.available_hosts.len() as i32);
        self.host = cpal::host_from_id(self.available_hosts[index as usize]).unwrap();

        self.query_devices();
    }

    pub fn input_device_names(&self) -> Vec<String> {
        self.input_devices
            .iter()
            .filter_map(|device| device.name().ok())
            .collect()
    }

    pub fn rotate_input_device(&mut self, offset: i32) {
        if let Some(index) = self.input_device {
            let index = (index as i32 + offset).rem_euclid(self.input_devices.len() as i32);
            self.input_device = Some(index as usize);
        } else if !self.input_devices.is_empty() {
            self.input_device = Some(0);
        }

        self.query_sample_rates();
    }

    pub fn output_device_names(&self) -> Vec<String> {
        self.output_devices
            .iter()
            .filter_map(|device| device.name().ok())
            .collect()
    }

    pub fn rotate_output_device(&mut self, offset: i32) {
        if let Some(index) = self.output_device {
            let index = (index as i32 + offset).rem_euclid(self.output_devices.len() as i32);
            self.output_device = Some(index as usize);
        } else if !self.output_devices.is_empty() {
            self.output_device = Some(0);
        }

        self.query_sample_rates();
    }

    pub fn rotate_sample_rate(&mut self, offset: i32) {
        if let Some(index) = self.sample_rate {
            let index = (index as i32 + offset).rem_euclid(self.sample_rates.len() as i32);
            self.sample_rate = Some(index as usize);
        } else if !self.sample_rates.is_empty() {
            self.sample_rate = Some(0);
        }
    }

    pub fn rotate_buffer_size(&mut self, offset: i32) {
        if let Some(index) = self.buffer_size {
            let index = (index as i32 + offset).rem_euclid(self.buffer_sizes.len() as i32);
            self.buffer_size = Some(index as usize);
        } else if !self.buffer_sizes.is_empty() {
            self.buffer_size = Some(0);
        }
    }

    pub fn get_input_device(&self) -> Option<&Device> {
        self.input_device.map(|i| &self.input_devices[i])
    }

    pub fn get_output_device(&self) -> Option<&Device> {
        self.output_device.map(|i| &self.output_devices[i])
    }

    pub fn get_sample_rate(&self) -> Option<SampleRate> {
        self.sample_rate.map(|i| self.sample_rates[i])
    }

    pub fn get_buffer_size(&self) -> Option<BufferSize> {
        Some(BufferSize::Fixed(self.buffer_sizes[self.buffer_size?]))
    }

    pub fn launch_stream(
        &self,
        engine: Arc<AudioEngine>,
        tracks: &Tracks,
    ) -> anyhow::Result<(Stream, Stream)> {
        gag!();

        let input_device = self.get_input_device().ok_or(anyhow!("no input device"))?;
        let output_device = self
            .get_output_device()
            .ok_or(anyhow!("no output device"))?;
        let sample_rate = self.get_sample_rate().ok_or(anyhow!("no sample rate"))?;
        let buffer_size = self.get_buffer_size().unwrap_or(BufferSize::Default);

        let default_input_config = input_device.default_input_config().unwrap();
        let default_output_config = output_device.default_output_config().unwrap();

        let input_channels = default_input_config.channels();
        let output_channels = default_output_config.channels();

        let input_config = StreamConfig {
            channels: input_channels,
            sample_rate,
            buffer_size,
        };
        let output_config = StreamConfig {
            channels: output_channels,
            sample_rate,
            buffer_size,
        };

        if input_channels != output_channels {
            return Err(anyhow!("input and output channels must match"));
        }

        let buffer_size = input_channels as u32 * sample_rate.0 * self.delay / 1000;
        let (mut prod, mut cons) = HeapRb::new(buffer_size as usize * 2).split();

        for _ in 0..buffer_size {
            prod.push(0.0).unwrap();
        }

        let error = |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        let input_stream = input_device.build_input_stream(
            &input_config,
            move |data: &[f32], _: &InputCallbackInfo| {
                for &sample in data {
                    let _ = prod.push(sample);
                }
            },
            error,
            None,
        )?;

        engine.set_sample_rate(sample_rate.0 as u64);
        let mut tracks = tracks.clone();
        let mut recording = Vec::new();
        let mut channel = 0;
        let mut last_feedback = 0.0;

        let data = move |data: &mut [f32], _: &OutputCallbackInfo| {
            for target in data {
                if engine.is_on_beat() {
                    // if tracks have been updated, use them
                    if let Some(new_tracks) = engine.take_tracks() {
                        tracks = new_tracks;
                    }
                }

                channel += 1;

                if channel == input_channels {
                    engine.sample.fetch_add(1, Ordering::AcqRel);
                    channel = 0;
                }

                let feedback = cons.pop().unwrap_or(last_feedback);
                last_feedback = feedback;
                recording.push(feedback);

                *target = get_sample(&engine, &tracks, channel as u64, feedback);

                if engine.should_loop() {
                    engine.set_sample(0);

                    let clip = Clip {
                        channels: input_channels,
                        sample_rate,
                        samples: Arc::from(mem::take(&mut recording)),
                    };

                    engine.set_recorded_clip(Some(clip));
                }
            }
        };

        let output_stream = output_device.build_output_stream(&output_config, data, error, None)?;

        input_stream.play()?;
        output_stream.play()?;

        Ok((input_stream, output_stream))
    }
}

fn metronome(time: f32) -> f32 {
    (time * 880.0).sin() * (1.0 - time * 2.0).clamp(0.0, 1.0) * 0.5
}

fn get_sample(engine: &AudioEngine, tracks: &Tracks, channel: u64, feedback: f32) -> f32 {
    let mut sample = 0.0;
    let beat_offset = engine.beat().fract();

    // add in the feedback
    sample += feedback;

    // add in the metronome

    if engine.metronome() {
        sample += metronome(beat_offset);
    }

    let sample_index = engine.sample() as usize;

    // add in the tracks
    for track in tracks.iter() {
        let Some(ref clip) = track.clip else {
            continue;
        };

        let mut track_sample = clip.sample(sample_index, channel as usize);
        track_sample *= track.volume_factor();

        sample += track_sample;
    }

    sample
}

pub struct Audio {
    pub settings: AudioSettings,
    pub input_stream: Option<Stream>,
    pub output_stream: Option<Stream>,
    pub engine: Arc<AudioEngine>,
    pub tracks: Tracks,
    pub clip: Option<Clip>,
    pub error: Option<anyhow::Error>,
}

impl Audio {
    pub fn new() -> Audio {
        let mut settings = AudioSettings::new();
        settings.rotate_host(0);
        settings.query_devices();

        let mut audio = Audio {
            settings,
            input_stream: None,
            output_stream: None,
            engine: Arc::new(AudioEngine::default()),
            tracks: Tracks::default(),
            clip: None,
            error: None,
        };

        audio.launch_streams();

        audio
    }

    pub fn get_clip(&mut self) -> Option<Clip> {
        if let Some(clip) = self.engine.recorded_clip.take() {
            self.clip = Some(clip.clone());
            Some(clip)
        } else {
            self.clip.clone()
        }
    }

    pub fn update_tracks(&mut self) {
        self.engine.set_tracks(Some(self.tracks.clone()));
    }

    pub fn launch_streams(&mut self) {
        match self
            .settings
            .launch_stream(self.engine.clone(), &self.tracks)
        {
            Ok((input_stream, output_stream)) => {
                self.input_stream = Some(input_stream);
                self.output_stream = Some(output_stream);
                self.error = None;
            }
            Err(err) => self.error = Some(err),
        }
    }
}
