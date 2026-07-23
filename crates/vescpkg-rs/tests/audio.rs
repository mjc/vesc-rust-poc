#![cfg(feature = "test-support")]
//! Integration coverage for the optional FOC audio subsystem.

use vescpkg_rs::{
    AudioChannel, AudioDuration, AudioFrequency, AudioSampleRate, AudioVoltage,
    FocAudioSampleTable, FocAudioStopMode, Frequency, PackageRuntimeState, PackageStateStore,
    SampleRate, Voltage, test_support::FirmwareTest,
};

struct PackageState {
    _table: Option<FocAudioSampleTable<'static>>,
}

static PACKAGE_STATE: PackageStateStore<PackageState> = PackageStateStore::new();

impl PackageRuntimeState for PackageState {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &PACKAGE_STATE
    }
}

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
    audio.stop(FocAudioStopMode::Reset).unwrap();
}

#[test]
fn firmware_audio_rejects_invalid_values_before_ffi() {
    let firmware = FirmwareTest::new();
    let audio = firmware.audio();
    let channel = AudioChannel::try_new(0).unwrap();
    let voltage = AudioVoltage::new(Voltage::from_volts(0.2));

    assert!(
        audio
            .beep(
                AudioFrequency::new(Frequency::from_hertz(f32::NAN)),
                AudioDuration::new(vescpkg_rs::VescSeconds::from_seconds(0.1)),
                voltage,
            )
            .is_err()
    );
    assert!(
        audio
            .play_samples(
                &[],
                AudioSampleRate::new(SampleRate::from_hertz(22_050.0)),
                voltage,
            )
            .is_err()
    );
    assert!(audio.set_sample_table(channel, &[f32::INFINITY]).is_err());
}

#[test]
fn firmware_audio_reports_absent_optional_slots() {
    let firmware = FirmwareTest::new();
    firmware.set_audio_available(false);
    let audio = firmware.audio();
    let channel = AudioChannel::try_new(1).unwrap();
    let voltage = AudioVoltage::new(Voltage::from_volts(0.2));

    assert_eq!(
        audio.beep(
            AudioFrequency::new(Frequency::from_hertz(440.0)),
            AudioDuration::new(vescpkg_rs::VescSeconds::from_seconds(0.1)),
            voltage,
        ),
        Err(vescpkg_rs::FocAudioError::Unavailable)
    );
    assert_eq!(
        audio.play_tone(
            channel,
            AudioFrequency::new(Frequency::from_hertz(880.0)),
            voltage,
        ),
        Err(vescpkg_rs::FocAudioError::Unavailable)
    );
    assert_eq!(
        audio.play_samples(
            &[1, -2, 3],
            AudioSampleRate::new(SampleRate::from_hertz(22_050.0)),
            voltage,
        ),
        Err(vescpkg_rs::FocAudioError::Unavailable)
    );
    assert!(matches!(
        audio.set_sample_table(channel, &[0.1, 0.2]),
        Err(vescpkg_rs::FocAudioError::Unavailable)
    ));
    assert_eq!(
        audio.stop(FocAudioStopMode::Reset),
        Err(vescpkg_rs::FocAudioError::Unavailable)
    );
    assert!(unsafe { audio.sample_table_ptr(channel) }.is_none());
}

#[test]
fn package_stop_releases_audio_table_before_next_registration() {
    static SAMPLES: [f32; 3] = [0.1, 0.2, 0.3];
    let firmware = FirmwareTest::new();
    let channel = AudioChannel::try_new(1).unwrap();
    let table = firmware
        .audio()
        .set_sample_table(channel, &SAMPLES)
        .expect("audio sample table");
    let mut info = vescpkg_rs::test_support::LoaderInfo::new();
    let mut start = vescpkg_rs::test_support::package_start(&mut info);
    start
        .install_runtime_state(PackageState {
            _table: Some(table),
        })
        .expect("package state");
    assert!(start.finish_start(true));
    assert!(vescpkg_rs::test_support::stop_package(&mut info));

    firmware
        .audio()
        .set_sample_table(channel, &SAMPLES)
        .expect("stop released audio table");
}
