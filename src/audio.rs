//! Microphone capture. Levels are always measured. Samples are kept only
//! during a talk gesture.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

struct Shared {
    capture: AtomicBool,
    samples: Mutex<Vec<f32>>,
    rms_bits: AtomicU32,
    stream_error: Mutex<Option<String>>,
}

pub struct AudioEngine {
    stream: Option<cpal::Stream>,
    shared: Arc<Shared>,
    device_name: String,
    sample_rate: u32,
    error: Option<String>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            stream: None,
            shared: Arc::new(Shared {
                capture: AtomicBool::new(false),
                samples: Mutex::new(Vec::new()),
                rms_bits: AtomicU32::new(0.0f32.to_bits()),
                stream_error: Mutex::new(None),
            }),
            device_name: "No microphone".into(),
            sample_rate: 48_000,
            error: None,
        }
    }

    pub fn open(&mut self, preferred: Option<&str>) {
        self.stream = None;
        self.shared.capture.store(false, Ordering::Relaxed);
        match open_input(preferred, self.shared.clone()) {
            Ok(opened) => {
                self.device_name = opened.name;
                self.sample_rate = opened.sample_rate;
                self.stream = Some(opened.stream);
                self.error = None;
            }
            Err(err) => {
                self.device_name = "No microphone".into();
                self.error = Some(err);
            }
        }
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn take_stream_error(&self) -> Option<String> {
        self.shared
            .stream_error
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    pub fn rms(&self) -> f32 {
        f32::from_bits(self.shared.rms_bits.load(Ordering::Relaxed))
    }

    pub fn start_capture(&self) {
        if let Ok(mut samples) = self.shared.samples.lock() {
            samples.clear();
        }
        self.shared.capture.store(true, Ordering::Relaxed);
    }

    pub fn discard(&self) {
        self.shared.capture.store(false, Ordering::Relaxed);
        if let Ok(mut samples) = self.shared.samples.lock() {
            samples.clear();
        }
    }

    pub fn stop_capture(&self) -> (Vec<f32>, u32) {
        self.shared.capture.store(false, Ordering::Relaxed);
        let samples = self
            .shared
            .samples
            .lock()
            .map(|mut samples| std::mem::take(&mut *samples))
            .unwrap_or_default();
        (samples, self.sample_rate.max(1))
    }
}

pub fn list_inputs() -> Vec<String> {
    let host = cpal::default_host();
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices.filter_map(|device| device.name().ok()).collect()
}

struct Opened {
    stream: cpal::Stream,
    name: String,
    sample_rate: u32,
}

fn open_input(preferred: Option<&str>, shared: Arc<Shared>) -> Result<Opened, String> {
    let host = cpal::default_host();
    let device = pick_input(&host, preferred)?;
    let name = device.name().unwrap_or_else(|_| "Microphone".into());
    let supported = device
        .default_input_config()
        .map_err(|err| err.to_string())?;
    let sample_rate = supported.sample_rate().0.max(1);
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let stream = match format {
        cpal::SampleFormat::F32 => {
            build::<f32>(&device, &config, channels, shared.clone(), f32_of_f32)
        }
        cpal::SampleFormat::I16 => {
            build::<i16>(&device, &config, channels, shared.clone(), f32_of_i16)
        }
        cpal::SampleFormat::U16 => {
            build::<u16>(&device, &config, channels, shared.clone(), f32_of_u16)
        }
        cpal::SampleFormat::I32 => {
            build::<i32>(&device, &config, channels, shared.clone(), f32_of_i32)
        }
        other => return Err(format!("Microphone format {other:?} isn't supported.")),
    }?;
    stream.play().map_err(|err| err.to_string())?;
    Ok(Opened {
        stream,
        name,
        sample_rate,
    })
}

fn pick_input(host: &cpal::Host, preferred: Option<&str>) -> Result<cpal::Device, String> {
    if let Some(name) = preferred {
        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if device.name().ok().as_deref() == Some(name) {
                    return Ok(device);
                }
            }
        }
    }
    host.default_input_device()
        .ok_or_else(|| "No microphone found.".into())
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    shared: Arc<Shared>,
    convert: fn(T) -> f32,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + Send + 'static,
{
    let channels = channels.max(1);
    let errors = shared.clone();
    device
        .build_input_stream(
            config,
            move |data: &[T], _| push(data, channels, &shared, convert),
            move |err| {
                if let Ok(mut slot) = errors.stream_error.lock() {
                    *slot = Some(err.to_string());
                }
            },
            None,
        )
        .map_err(|err| err.to_string())
}

fn push<T: Copy>(data: &[T], channels: usize, shared: &Shared, convert: fn(T) -> f32) {
    let frames = data.len() / channels;
    if frames == 0 {
        return;
    }
    let mut mono = Vec::with_capacity(frames);
    let mut sum_sq = 0.0f32;
    for frame in data.chunks(channels) {
        let mut acc = 0.0f32;
        for sample in frame {
            acc += convert(*sample);
        }
        let value = acc / channels as f32;
        sum_sq += value * value;
        mono.push(value);
    }
    let rms = (sum_sq / frames as f32).sqrt();
    shared.rms_bits.store(rms.to_bits(), Ordering::Relaxed);
    if shared.capture.load(Ordering::Relaxed) {
        if let Ok(mut buffer) = shared.samples.lock() {
            buffer.extend_from_slice(&mono);
            let cap = 48_000 * 60;
            if buffer.len() > cap {
                let drop_n = buffer.len() - cap;
                buffer.drain(0..drop_n);
            }
        }
    }
}

fn f32_of_f32(sample: f32) -> f32 {
    sample
}
fn f32_of_i16(sample: i16) -> f32 {
    sample as f32 / i16::MAX as f32
}
fn f32_of_u16(sample: u16) -> f32 {
    (sample as f32 / u16::MAX as f32) * 2.0 - 1.0
}
fn f32_of_i32(sample: i32) -> f32 {
    sample as f32 / i32::MAX as f32
}
