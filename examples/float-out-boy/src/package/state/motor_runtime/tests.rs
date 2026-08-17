use super::*;

#[test]
fn disabled_current_filter_returns_directional_current_like_float_out_boy() {
    let mut filter = FloatOutBoyMotorCurrentFilter::default();
    filter.configure(Frequency::ZERO, SampleRate::from_hertz(832.0));
    let directional = DirectionalMotorCurrent::new(Current::from_amps(-6.75));

    assert_eq!(filter.process(directional).current(), directional);
}

#[test]
fn current_filter_runtime_reset_clears_biquad_history_like_refloat_engage() {
    let mut filter = FloatOutBoyMotorCurrentFilter::default();
    filter.configure(Frequency::from_hertz(10.0), SampleRate::from_hertz(500.0));
    let _ = filter.process(DirectionalMotorCurrent::new(Current::from_amps(20.0)));

    filter.reset_runtime();

    assert_eq!(
        filter
            .process(DirectionalMotorCurrent::new(Current::ZERO))
            .current(),
        DirectionalMotorCurrent::new(Current::ZERO)
    );
}
