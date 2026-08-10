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

#[test]
fn ring_slots_match_a_bounded_fifo_across_capacities_and_wraps() {
    for capacity in 0_usize..=32 {
        for writes in 0..=capacity.saturating_mul(3).saturating_add(1) {
            let mut ring = DataRecorderRing::default();
            for _ in 0..writes {
                if ring.write_slot(capacity).is_some() {
                    ring.commit_write(capacity);
                }
            }

            let len = writes.min(capacity);
            assert_eq!(
                ring.len(capacity),
                len,
                "capacity {capacity}, writes {writes}"
            );
            for index in 0..len {
                let expected = writes.saturating_sub(len).saturating_add(index) % capacity;
                assert_eq!(
                    ring.slot_at(index, capacity),
                    Some(expected),
                    "capacity {capacity}, writes {writes}, index {index}"
                );
            }
            assert_eq!(ring.slot_at(len, capacity), None);
        }
    }
}
