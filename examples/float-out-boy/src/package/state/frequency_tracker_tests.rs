use super::frequency_tracker::*;
use vescpkg_rs::prelude::SampleRate;

#[test]
fn firmware_602_zero_imu_rate_uses_refloat_settling_seed() {
    assert_eq!(
        imu_start_frequency(SampleRate::from_hertz(0.0)),
        SampleRate::from_hertz(620.0),
    );
    assert_eq!(
        imu_start_frequency(SampleRate::from_hertz(833.0)),
        SampleRate::from_hertz(833.0),
    );
}
