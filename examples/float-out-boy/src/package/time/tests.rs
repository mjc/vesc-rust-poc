use super::*;

#[test]
fn expire_timer_subtracts_seconds_in_vesc_system_ticks_with_wraparound() {
    assert_eq!(
        float_out_boy_expire_timer(TimestampTicks::from_ticks(1_000_000), 60),
        TimestampTicks::from_ticks(400_000)
    );
    assert_eq!(
        float_out_boy_expire_timer(TimestampTicks::from_ticks(0), 60),
        TimestampTicks::from_ticks(0_u32.wrapping_sub(600_000))
    );
}
