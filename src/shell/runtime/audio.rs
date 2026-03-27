use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BuildStreamError, DefaultStreamConfigError, FromSample, PlayStreamError, Sample, SampleFormat,
    SizedSample, Stream, StreamConfig, SupportedStreamConfig,
};
use ringbuf::{
    HeapCons, HeapProd, HeapRb,
    traits::{Consumer, Producer, Split},
};
use thiserror::Error;

use super::session::RuntimePreferences;

const AUDIO_BUFFER_CAPACITY: usize = 16_384;

#[derive(Debug, Error)]
pub enum AudioInitError {
    #[error("no default audio output device is available")]
    NoOutputDevice,
    #[error("failed to read the default audio output configuration: {source}")]
    DefaultConfig {
        #[source]
        source: DefaultStreamConfigError,
    },
    #[error("failed to build the audio output stream: {source}")]
    BuildStream {
        #[source]
        source: BuildStreamError,
    },
    #[error("failed to start the audio output stream: {source}")]
    PlayStream {
        #[source]
        source: PlayStreamError,
    },
    #[error("unsupported audio sample format: {sample_format:?}")]
    UnsupportedSampleFormat { sample_format: SampleFormat },
}

impl AudioInitError {
    pub fn diagnostic_message(&self) -> String {
        format!("RustNES could not start audio playback: {self}.")
    }
}

pub struct RuntimeAudio {
    producer: HeapProd<f32>,
    controls: Arc<AudioControls>,
    sample_rate: u32,
    stream: Option<Stream>,
    offline_consumer: Option<HeapCons<f32>>,
}

impl std::fmt::Debug for RuntimeAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeAudio")
            .field("sample_rate", &self.sample_rate)
            .field("output_available", &self.output_available())
            .finish()
    }
}

impl RuntimeAudio {
    pub fn without_output(sample_rate: u32) -> Self {
        let rb = HeapRb::<f32>::new(AUDIO_BUFFER_CAPACITY);
        let (producer, consumer) = rb.split();
        Self {
            producer,
            controls: Arc::new(AudioControls::default()),
            sample_rate: sample_rate.max(1),
            stream: None,
            offline_consumer: Some(consumer),
        }
    }

    pub fn new_default(preferred_sample_rate: u32) -> Result<Self, AudioInitError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioInitError::NoOutputDevice)?;
        let supported_config = device
            .default_output_config()
            .map_err(|source| AudioInitError::DefaultConfig { source })?;

        Self::from_supported_config(device, supported_config, preferred_sample_rate)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn output_available(&self) -> bool {
        self.stream.is_some()
    }

    pub fn apply_preferences(&self, preferences: RuntimePreferences, paused: bool) {
        self.controls.set_paused(paused);
        self.controls.set_muted(preferences.muted);
        self.controls.set_volume(preferences.master_volume);
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        for sample in samples.iter().copied() {
            let _ = self.producer.try_push(sample);
        }
    }

    pub fn render_offline(&mut self, frame_count: usize, channels: usize) -> Vec<f32> {
        let Some(consumer) = self.offline_consumer.as_mut() else {
            return Vec::new();
        };

        let channel_count = channels.max(1);
        let mut data = vec![0.0; frame_count * channel_count];
        write_output::<f32>(&mut data, consumer, &self.controls, channel_count);
        data
    }

    fn from_supported_config(
        device: cpal::Device,
        supported_config: SupportedStreamConfig,
        _preferred_sample_rate: u32,
    ) -> Result<Self, AudioInitError> {
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        let channels = usize::from(config.channels.max(1));
        let sample_rate = config.sample_rate.max(1);
        let rb = HeapRb::<f32>::new(AUDIO_BUFFER_CAPACITY);
        let (producer, consumer) = rb.split();
        let controls = Arc::new(AudioControls::default());

        let stream = match sample_format {
            SampleFormat::F32 => {
                build_output_stream::<f32>(&device, &config, channels, consumer, controls.clone())
            }
            SampleFormat::I16 => {
                build_output_stream::<i16>(&device, &config, channels, consumer, controls.clone())
            }
            SampleFormat::U16 => {
                build_output_stream::<u16>(&device, &config, channels, consumer, controls.clone())
            }
            sample_format => return Err(AudioInitError::UnsupportedSampleFormat { sample_format }),
        }?;

        stream
            .play()
            .map_err(|source| AudioInitError::PlayStream { source })?;

        Ok(Self {
            producer,
            controls,
            sample_rate,
            stream: Some(stream),
            offline_consumer: None,
        })
    }
}

fn build_output_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    mut consumer: HeapCons<f32>,
    controls: Arc<AudioControls>,
) -> Result<Stream, AudioInitError>
where
    T: SizedSample + Sample + FromSample<f32>,
{
    let err_fn = |error| eprintln!("RustNES audio stream reported an error: {error}.");
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_output::<T>(data, &mut consumer, &controls, channels),
            err_fn,
            None,
        )
        .map_err(|source| AudioInitError::BuildStream { source })
}

fn write_output<T>(
    data: &mut [T],
    consumer: &mut HeapCons<f32>,
    controls: &AudioControls,
    channels: usize,
) where
    T: Sample + FromSample<f32>,
{
    for frame in data.chunks_mut(channels.max(1)) {
        let silence = controls.paused() || controls.muted();
        let sample = match consumer.try_pop() {
            Some(sample) if !silence => sample * controls.volume(),
            Some(_) | None => 0.0,
        };

        for output in frame {
            *output = T::from_sample(sample);
        }
    }
}

#[derive(Debug)]
struct AudioControls {
    paused: AtomicBool,
    muted: AtomicBool,
    volume_bits: AtomicU32,
}

impl Default for AudioControls {
    fn default() -> Self {
        Self {
            paused: AtomicBool::new(false),
            muted: AtomicBool::new(false),
            volume_bits: AtomicU32::new(1.0f32.to_bits()),
        }
    }
}

impl AudioControls {
    fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    fn muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    fn volume(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    fn set_volume(&self, volume: f32) {
        self.volume_bits
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}
