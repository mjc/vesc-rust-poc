#![cfg(feature = "test-support")]
//! Integration coverage for the optional FOC audio subsystem.

use vescpkg_rs::{
    AudioChannel, AudioDuration, AudioFrequency, AudioSampleRate, AudioVoltage, Frequency,
    SampleRate, Voltage, test_support::FirmwareTest,
};

#[test]
fn firmware_audio_forwards_checked_commands_and_owns_sample_table_borrow() {
    let firmware = FirmwareTest::new();
    let audio = firmware.audio();
    let channel = AudioChannel::try_new(1).unwrap();
    audio
        .beep(
            AudioFrequency::new(Frequency::from_hertz(440.0)),
            AudioDuration::new(vescpkg_rs::VescSeconds::from_seconds(0.1)),
            AudioVoltage::new(Voltage::from_volts(0.5)),
        )
        .unwrap();
    audio
        .play_tone(
            channel,
            AudioFrequency::new(Frequency::from_hertz(880.0)),
            AudioVoltage::new(Voltage::from_volts(0.25)),
        )
        .unwrap();
    audio
        .play_samples(
            &[1, -2, 3],
            AudioSampleRate::new(SampleRate::from_hertz(22_050.0)),
            AudioVoltage::new(Voltage::from_volts(0.2)),
        )
        .unwrap();
    let samples = [0.1, 0.2, 0.3];
    let table = audio.set_sample_table(channel, &samples).unwrap();
    assert!(unsafe { audio.sample_table_ptr(channel) }.is_some());
    drop(table);
    audio.stop(true).unwrap();
}
