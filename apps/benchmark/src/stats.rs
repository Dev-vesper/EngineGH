use crate::report::Distribution;

/// Nearest-rank percentile of an ascending-sorted slice. `p` in (0, 100].
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn sorted(samples: &[f64]) -> Vec<f64> {
    let mut v = samples.to_vec();
    v.sort_by(f64::total_cmp);
    v
}

/// `samples` must not be empty.
pub fn distribution(samples: &[f64]) -> Distribution {
    let s = sorted(samples);
    let n = s.len() as f64;
    let mean = s.iter().sum::<f64>() / n;
    let var = s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    Distribution {
        mean,
        std_dev: var.sqrt(),
        min: s[0],
        p50: percentile(&s, 50.0),
        p95: percentile(&s, 95.0),
        p99: percentile(&s, 99.0),
        max: s[s.len() - 1],
    }
}

/// FPS averaged over the slowest 1% of frames (at least one frame). `frame_ms` must not be empty.
pub fn fps_1pct_low(frame_ms: &[f64]) -> f64 {
    let s = sorted(frame_ms);
    let count = (s.len() / 100).max(1);
    let worst_mean = s[s.len() - count..].iter().sum::<f64>() / count as f64;
    1000.0 / worst_mean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_percentiles() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        let d = distribution(&v);
        assert_eq!(d.p50, 50.0);
        assert_eq!(d.p95, 95.0);
        assert_eq!(d.p99, 99.0);
        assert_eq!((d.min, d.max), (1.0, 100.0));
        assert!((d.mean - 50.5).abs() < 1e-12);
    }

    #[test]
    fn single_sample() {
        let d = distribution(&[7.0]);
        assert_eq!((d.min, d.p50, d.p99, d.max, d.std_dev), (7.0, 7.0, 7.0, 7.0, 0.0));
    }

    #[test]
    fn one_percent_low_uses_slowest_frames() {
        let mut v = vec![10.0; 99];
        v.push(100.0);
        assert!((fps_1pct_low(&v) - 10.0).abs() < 1e-9); // slowest 1% = the single 100 ms frame
    }
}
