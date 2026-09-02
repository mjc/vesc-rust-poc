use super::EmaAlpha;
use vescpkg_rs::prelude::{Frequency, SampleRate, VescSeconds};

#[test]
fn ema_alpha_constructors_share_the_refloat_approximation_and_bound() {
    let cutoff = Frequency::from_hertz(25.0);
    let sample_rate = SampleRate::from_hertz(500.0);
    let elapsed = VescSeconds::from_seconds(1.0 / 500.0);

    let from_rate = EmaAlpha::from_sample_rate(cutoff, sample_rate);
    let from_elapsed = EmaAlpha::from_elapsed(cutoff, elapsed);
    assert_eq!(from_rate, from_elapsed);
    assert_f32_eq!(from_rate.factor(), 0.264_811_25);

    let capped = EmaAlpha::from_time_constant(VescSeconds::from_seconds(0.000_1), sample_rate);
    assert_f32_eq!(capped.factor(), 0.375);
    assert_f32_eq!(capped.retained(), 0.625);
    assert_f32_eq!(capped.scaled(3.0).factor(), 1.0);
}
