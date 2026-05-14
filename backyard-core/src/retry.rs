use chrono::{DateTime, Utc, Duration};
use std::time;

pub fn next_retry_at(attempts: u32) -> DateTime<Utc> {
    let base_secs: u64 = 10;
    let cap_secs: u64 = 3600;

    let backoff = base_secs
        .saturating_mul(2u64.saturating_pow(attempts))
        .min(cap_secs);

    let jitter = rand::random::<u64>() % (backoff / 4 + 1);
    let delay = time::Duration::from_secs(backoff + jitter);
    Utc::now() + Duration::from_std(delay).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_growth() {
        let t0 = next_retry_at(0);
        let t1 = next_retry_at(1);
        let t2 = next_retry_at(2);

        assert!(t1 > t0);
        assert!(t2 > t1);
    }

    #[test]
    fn test_backoff_cap() {
        let t100 = next_retry_at(100);
        let t101 = next_retry_at(101);

        let diff = t101.signed_duration_since(t100);
        assert!(diff.num_seconds() < 3600);
    }
}
