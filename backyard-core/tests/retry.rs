use backyard_core::retry::next_retry_at;
use chrono::Utc;

#[test]
fn test_exponential_backoff() {
    let now = Utc::now();
    let t1 = next_retry_at(0);
    let t2 = next_retry_at(1);
    let t3 = next_retry_at(2);

    assert!(t1 > now, "retry 0 should be in the future");
    assert!(t2 > t1, "retry 1 should be later than retry 0");
    assert!(t3 > t2, "retry 2 should be later than retry 1");
}

#[test]
fn test_backoff_cap() {
    let t100 = next_retry_at(100);
    let t101 = next_retry_at(101);

    let diff = t101.signed_duration_since(t100);
    assert!(
        diff.num_seconds() < 3600,
        "backoff should be capped at 1 hour"
    );
}
