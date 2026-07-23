//! Usage-shaped access to the distinct firmware clock domains.

use vescpkg_rs::{FirmwareClock, TimerInstant, TimestampTicks, VescSeconds};

/// Read system ticks, firmware float uptime, and the high-resolution timer together.
pub fn read_probe(clock: &FirmwareClock) -> (TimestampTicks, VescSeconds, TimerInstant) {
    (clock.now(), clock.uptime(), clock.timer_now())
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::read_probe;
    use vescpkg_rs::test_support::FirmwareTest;
    use vescpkg_rs::{TimerInstant, TimestampTicks, VescSeconds};

    #[test]
    fn package_clock_probe_keeps_domains_typed() {
        let firmware = FirmwareTest::new();
        firmware.set_clock_ticks(12_345);
        firmware.set_timer_ticks(77);

        assert_eq!(
            read_probe(firmware.clock()),
            (
                TimestampTicks::from_ticks(12_345),
                VescSeconds::from_seconds(1.2345),
                TimerInstant::from_raw(77),
            )
        );
    }
}
