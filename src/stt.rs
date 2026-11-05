//! Speech engines. Local Whisper when the feature is compiled in, or any
//! server that speaks the OpenAI audio transcriptions API.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use serde::Deserialize;

use crate::bus::WorkerEvent;
use crate::config::{Config, SttEngine};
use crate::resample;
use crate::wav;

pub const MODEL_EN_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin";
pub const MODEL_MULTI_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";

#[derive(Clone)]
pub struct SttSettings {
    pub engine: SttEngine,
    pub model_path: PathBuf,
    pub language: String,
    pub api_base: String,
    pub api_key: String,
    pub api_model: String,
}

pub enum SttCommand {
    Transcribe {
        epoch: u64,
        samples_16k: Vec<f32>,
        settings: SttSettings,
    },
}

pub fn settings_from(config: &Config) -> SttSettings {
    SttSettings {
        engine: config.stt.engine,
        model_path: config.resolved_model_path(),
        language: config.stt.language.clone(),
        api_base: config.stt.api_base.clone(),
        api_key: config.stt.api_key.clone(),
        api_model: config.stt.api_model.clone(),
    }
}

pub fn model_is_ready(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() > 1_000_000)
        .unwrap_or(false)
}

pub fn worker(rx: Receiver<SttCommand>, tx: Sender<WorkerEvent>) {
    let mut cache = ModelCache::new();
    while let Ok(command) = rx.recv() {
        match command {
            SttCommand::Transcribe {
                epoch,
                samples_16k,
                settings,
            } => {
                let samples = pad(&samples_16k);
                let result = transcribe(&samples, &settings, &mut cache);
                let _ = tx.send(WorkerEvent::Transcript { epoch, result });
            }
        }
    }
}

pub fn transcribe_wav_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    let pcm = wav::decode_wav(&bytes)?;
    let samples = resample::resample_linear(&pcm.samples, pcm.sample_rate, 16_000);
    let config = Config::load().0;
    let settings = settings_from(&config);
    let mut cache = ModelCache::new();
    transcribe(&pad(&samples), &settings, &mut cache)
}

pub fn download_model(
    url: &str,
    dest: &Path,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let part = dest.with_extension("part");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .user_agent("callsign/0.1")
        .build()
        .map_err(|err| err.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|err| format!("Couldn't reach the model host. {err}"))?;
    if !response.status().is_success() {
        return Err(format!("Model download failed ({})", response.status()));
    }
    let total = response.content_length();
    let mut file = File::create(&part).map_err(|err| err.to_string())?;
    let mut buf = [0u8; 64 * 1024];
    let mut got = 0u64;
    loop {
        let n = response.read(&mut buf).map_err(|err| err.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|err| err.to_string())?;
        got += n as u64;
        progress(got, total);
    }
    file.sync_all().map_err(|err| err.to_string())?;
    drop(file);
    if got < 1_000_000 {
        let _ = fs::remove_file(&part);
        return Err("The download was too small to be a speech model.".into());
    }
    fs::rename(&part, dest).map_err(|err| err.to_string())?;
    Ok(())
}

fn pad(samples: &[f32]) -> Vec<f32> {
    let mut out = samples.to_vec();
    out.extend(std::iter::repeat(0.0).take(3_200));
    out
}

fn transcribe(
    samples: &[f32],
    settings: &SttSettings,
    cache: &mut ModelCache,
) -> Result<String, String> {
    match settings.engine {
        SttEngine::Http => transcribe_http(samples, settings),
        SttEngine::Local => transcribe_local(samples, settings, cache),
    }
}

fn transcribe_http(samples: &[f32], settings: &SttSettings) -> Result<String, String> {
    if settings.api_key.trim().is_empty() {
        return Err("Add an API key in Setup, or use the local model.".into());
    }
    let wav = wav::encode_wav_mono_i16(samples, 16_000);
    let part = reqwest::blocking::multipart::Part::bytes(wav)
        .file_name("speech.wav")
        .mime_str("audio/wav")
        .map_err(|err| err.to_string())?;
    let mut form = reqwest::blocking::multipart::Form::new()
        .text("model", settings.api_model.clone())
        .part("file", part);
    if !settings.language.is_empty() && settings.language != "auto" {
        form = form.text("language", settings.language.clone());
    }
    let base = settings.api_base.trim().trim_end_matches('/');
    let url = format!("{base}/audio/transcriptions");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("callsign/0.1")
        .build()
        .map_err(|err| err.to_string())?;
    let response = client
        .post(&url)
        .bearer_auth(settings.api_key.trim())
        .multipart(form)
        .send()
        .map_err(|err| format!("Couldn't reach the speech server. {err}"))?;
    let status = response.status();
    let body = response.text().map_err(|err| err.to_string())?;
    if !status.is_success() {
        let snippet: String = body.chars().take(180).collect();
        return Err(format!("Speech server returned {status}. {snippet}"));
    }
    let parsed: Transcript = serde_json::from_str(&body)
        .map_err(|_| "Speech server returned something that wasn't a transcript.".to_string())?;
    Ok(parsed.text)
}

#[derive(Deserialize)]
struct Transcript {
    text: String,
}

struct ModelCache {
    #[cfg(feature = "whisper")]
    loaded: Option<(PathBuf, whisper_rs::WhisperContext)>,
}

impl ModelCache {
    fn new() -> Self {
        Self {
            #[cfg(feature = "whisper")]
            loaded: None,
        }
    }
}

#[cfg(not(feature = "whisper"))]
fn transcribe_local(
    _samples: &[f32],
    settings: &SttSettings,
    _cache: &mut ModelCache,
) -> Result<String, String> {
    let _ = &settings.model_path;
    Err("This build has no local speech model. Switch Setup to a speech server.".into())
}

#[cfg(feature = "whisper")]
fn transcribe_local(
    samples: &[f32],
    settings: &SttSettings,
    cache: &mut ModelCache,
) -> Result<String, String> {
    if !model_is_ready(&settings.model_path) {
        return Err(format!(
            "Download a speech model in Setup. Looked for {}.",
            settings.model_path.display()
        ));
    }
    let path = settings.model_path.clone();
    let reload = match &cache.loaded {
        Some((loaded, _)) => loaded != &path,
        None => true,
    };
    if reload {
        let ctx = whisper_rs::WhisperContext::new_with_params(
            &path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|err| format!("Couldn't load the speech model. {err}"))?;
        cache.loaded = Some((path, ctx));
    }
    let ctx = &cache.loaded.as_ref().unwrap().1;
    let mut state = ctx
        .create_state()
        .map_err(|err| format!("Couldn't start the speech model. {err}"))?;
    let mut params =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(2)
        .clamp(1, 4);
    params.set_n_threads(threads);
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_single_segment(true);
    params.set_no_context(true);
    params.set_suppress_blank(true);
    let language = settings.language.trim().to_string();
    if !language.is_empty() && language != "auto" {
        params.set_language(Some(&language));
    }
    state
        .full(params, samples)
        .map_err(|err| format!("Transcription failed. {err}"))?;
    let mut text = String::new();
    let count = state.full_n_segments();
    for index in 0..count {
        if let Some(segment) = state.get_segment(index) {
            if let Ok(piece) = segment.to_str() {
                text.push_str(piece);
            }
        }
    }
    Ok(text)
}
