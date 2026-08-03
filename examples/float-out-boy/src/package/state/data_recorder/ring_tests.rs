use super::DataRecorderRing;

#[test]
fn default_ring_is_empty() {
    assert_eq!(DataRecorderRing::default().len(24), 0);
}

#[test]
fn large_capacity_wrap_preserves_newest_samples() {
    let capacity = 2_539;
    let mut ring = DataRecorderRing::default();

    for _ in 0..=capacity {
        assert!(ring.write_slot(capacity).is_some());
        ring.commit_write(capacity);
    }

    assert_eq!(ring.len(capacity), capacity);
    assert_eq!(ring.slot_at(0, capacity), Some(1));
    assert_eq!(ring.slot_at(capacity - 1, capacity), Some(0));
    assert_eq!(ring.slot_at(capacity, capacity), None);
}
