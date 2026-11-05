//! Linear resample. Speech keys do not need a studio converter.

pub fn resample_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if input.is_empty() || from_hz == 0 || to_hz == 0 {
        return Vec::new();
    }
    if from_hz == to_hz {
        return input.to_vec();
    }
    let ratio = from_hz as f64 / to_hz as f64;
    let out_len = ((input.len() as f64) / ratio).round().max(1.0) as usize;
    let last = input.len() - 1;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let position = i as f64 * ratio;
        let left = (position.floor() as usize).min(last);
        let right = (left + 1).min(last);
        let blend = (position - left as f64) as f32;
        let sample = input[left] * (1.0 - blend) + input[right] * blend;
        out.push(sample);
    }
    out
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|sample| sample * sample).sum();
    (sum / samples.len() as f32).sqrt()
}

pub fn trim_lead(samples: Vec<f32>, sample_rate: u32, lead_ms: u64) -> Vec<f32> {
    if sample_rate == 0 || lead_ms == 0 {
        return samples;
    }
    let drop_n = (sample_rate as u64 * lead_ms / 1000) as usize;
    if drop_n + (sample_rate as usize / 10) >= samples.len() {
        return samples;
    }
    samples[drop_n..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_rate_is_a_copy() {
        let input = vec![0.0, 0.25, 0.5];
        assert_eq!(resample_linear(&input, 16_000, 16_000), input);
    }

    #[test]
    fn constant_signal_stays_constant() {
        let input = vec![0.4; 8];
        let out = resample_linear(&input, 8_000, 4_000);
        assert!(out.len() >= 3 && out.len() <= 5, "len {}", out.len());
        assert!(out.iter().all(|sample| (sample - 0.4).abs() < 0.001));
    }

    #[test]
    fn empty_and_rms() {
        assert!(resample_linear(&[], 48_000, 16_000).is_empty());
        assert!((rms(&[0.0, 0.0]) - 0.0).abs() < f32::EPSILON);
        let loud = rms(&[0.5, -0.5]);
        assert!(loud > 0.4 && loud < 0.6, "{loud}");
    }

    #[test]
    fn trim_keeps_short_clips() {
        let samples = vec![1.0; 100];
        assert_eq!(trim_lead(samples.clone(), 16_000, 90).len(), 100);
        let long = vec![1.0; 16_000];
        assert!(trim_lead(long, 16_000, 90).len() < 16_000);
    }
}
