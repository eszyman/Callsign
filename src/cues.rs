//! Short tones so a pilot in a headset can hear the gesture without looking.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueKind {
    Listen,
    Ready,
    Working,
    ChatSent,
    CommandSent,
    Cancel,
    Problem,
    AllPilots,
    Friendly,
}

struct Shared {
    samples: Mutex<VecDeque<f32>>,
}

pub struct CuePlayer {
    stream: Option<cpal::Stream>,
    shared: Arc<Shared>,
    sample_rate: u32,
    error: Option<String>,
}

impl CuePlayer {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            samples: Mutex::new(VecDeque::new()),
        });
        let mut player = Self {
            stream: None,
            shared,
            sample_rate: 48_000,
            error: None,
        };
        player.open();
        player
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn play(&self, kind: CueKind, volume: f32) {
        let Some(shared) = self.stream.as_ref().map(|_| &self.shared) else {
            return;
        };
        let amp = volume.clamp(0.0, 0.4);
        if amp <= 0.0 {
            return;
        }
        let rendered = render_cue(kind, amp, self.sample_rate);
        if let Ok(mut queue) = shared.samples.lock() {
            queue.extend(rendered);
        }
    }

    fn open(&mut self) {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(device) => device,
            None => {
                self.error = Some("No audio output for cue tones.".into());
                return;
            }
        };
        let supported = match device.default_output_config() {
            Ok(config) => config,
            Err(err) => {
                self.error = Some(err.to_string());
                return;
            }
        };
        self.sample_rate = supported.sample_rate().0.max(1);
        let channels = supported.channels() as usize;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let shared = self.shared.clone();
        let stream = match format {
            cpal::SampleFormat::F32 => {
                build_out::<f32>(&device, &config, channels, shared, sample_f32)
            }
            cpal::SampleFormat::I16 => {
                build_out::<i16>(&device, &config, channels, shared, sample_i16)
            }
            other => Err(format!("Cue output format {other:?} isn't supported.")),
        };
        match stream {
            Ok(stream) => {
                if let Err(err) = stream.play() {
                    self.error = Some(err.to_string());
                    return;
                }
                self.stream = Some(stream);
                self.error = None;
            }
            Err(err) => self.error = Some(err),
        }
    }
}

fn sample_f32(sample: f32) -> f32 {
    sample
}

fn sample_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

fn build_out<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    shared: Arc<Shared>,
    convert: fn(f32) -> T,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + Send + 'static,
{
    let channels = channels.max(1);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let mut queue = match shared.samples.lock() {
                    Ok(queue) => queue,
                    Err(_) => return,
                };
                for frame in data.chunks_mut(channels) {
                    let sample = queue.pop_front().unwrap_or(0.0);
                    let value = convert(sample);
                    for channel in frame.iter_mut() {
                        *channel = value;
                    }
                }
            },
            |err| eprintln!("cue output: {err}"),
            None,
        )
        .map_err(|err| err.to_string())
}

fn render_cue(kind: CueKind, amp: f32, rate: u32) -> Vec<f32> {
    match kind {
        CueKind::Listen => tone(880.0, 45, amp, rate),
        CueKind::Ready => tone(1480.0, 28, amp * 0.5, rate),
        CueKind::Working => tone(660.0, 55, amp, rate),
        CueKind::ChatSent => {
            let mut samples = tone(523.0, 70, amp, rate);
            samples.extend(silence(18, rate));
            samples.extend(tone(784.0, 90, amp, rate));
            samples
        }
        CueKind::CommandSent => tone(698.0, 90, amp, rate),
        CueKind::Cancel => tone(294.0, 100, amp, rate),
        CueKind::Problem => {
            let mut samples = tone(196.0, 80, amp, rate);
            samples.extend(silence(40, rate));
            samples.extend(tone(174.0, 120, amp, rate));
            samples
        }
        CueKind::AllPilots => {
            let mut samples = tone(784.0, 40, amp * 0.7, rate);
            samples.extend(silence(16, rate));
            samples.extend(tone(1174.0, 55, amp * 0.7, rate));
            samples
        }
        CueKind::Friendly => {
            let mut samples = tone(1174.0, 40, amp * 0.7, rate);
            samples.extend(silence(16, rate));
            samples.extend(tone(784.0, 55, amp * 0.7, rate));
            samples
        }
    }
}

fn silence(ms: u64, rate: u32) -> Vec<f32> {
    vec![0.0; frames(ms, rate)]
}

fn tone(freq: f32, ms: u64, amp: f32, rate: u32) -> Vec<f32> {
    let count = frames(ms, rate).max(1);
    let fade = (rate as usize / 250).clamp(1, count / 2);
    (0..count)
        .map(|i| {
            let env = if i < fade {
                i as f32 / fade as f32
            } else if i + fade >= count {
                (count - i) as f32 / fade as f32
            } else {
                1.0
            };
            let t = i as f32 / rate as f32;
            (t * freq * std::f32::consts::TAU).sin() * amp * env
        })
        .collect()
}

fn frames(ms: u64, rate: u32) -> usize {
    (rate as u64 * ms / 1000) as usize
}
