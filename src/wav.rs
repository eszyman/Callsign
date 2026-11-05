//! PCM WAV, just enough to ship audio to a speech server and read it back.

#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

pub fn encode_wav_mono_i16(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut data = Vec::with_capacity(44 + data_len as usize);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + data_len).to_le_bytes());
    data.extend_from_slice(b"WAVE");
    data.extend_from_slice(b"fmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    data.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&16u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        data.extend_from_slice(&value.to_le_bytes());
    }
    data
}

pub fn decode_wav(bytes: &[u8]) -> Result<Pcm, String> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("That file is not a WAV.".into());
    }
    let mut offset = 12;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end - start >= 16 {
            format = Some(Fmt {
                audio_format: u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                channels: u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                sample_rate: u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                bits: u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
            });
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = start + size + (size & 1);
    }
    let fmt = format.ok_or("The WAV has no format chunk.")?;
    let data = data.ok_or("The WAV has no audio data.")?;
    if fmt.audio_format != 1 || fmt.bits != 16 {
        return Err("Callsign can read 16-bit PCM WAV files.".into());
    }
    if fmt.channels == 0 || fmt.sample_rate == 0 {
        return Err("The WAV header looks empty.".into());
    }
    let mut samples = Vec::with_capacity(data.len() / 2);
    let channels = fmt.channels as usize;
    let mut frame = Vec::with_capacity(channels);
    for chunk in data.chunks_exact(2) {
        let value = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / i16::MAX as f32;
        frame.push(value);
        if frame.len() == channels {
            let mixed = frame.iter().sum::<f32>() / channels as f32;
            samples.push(mixed);
            frame.clear();
        }
    }
    Ok(Pcm {
        sample_rate: fmt.sample_rate,
        samples,
    })
}

struct Fmt {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    bits: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_mono() {
        let original = vec![0.0, 0.5, -0.5, 0.25];
        let encoded = encode_wav_mono_i16(&original, 16_000);
        assert_eq!(&encoded[0..4], b"RIFF");
        assert_eq!(&encoded[8..12], b"WAVE");
        let decoded = decode_wav(&encoded).unwrap();
        assert_eq!(decoded.sample_rate, 16_000);
        assert_eq!(decoded.samples.len(), original.len());
        for (got, expected) in decoded.samples.iter().zip(original.iter()) {
            assert!((got - expected).abs() < 0.001, "{got} vs {expected}");
        }
    }
}
