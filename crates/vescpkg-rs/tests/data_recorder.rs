//! Recorder-firmware descriptor contract tests.

use vescpkg_rs::{FirmwareDataRecorderDescriptor, FirmwareDataRecorderDescriptorError};

#[test]
fn recorder_descriptor_accepts_the_exact_refloat_firmware_region() {
    let descriptor =
        FirmwareDataRecorderDescriptor::try_from_words([0xcafe_1111, 0x1000_0000, 0x0000_f800])
            .expect("exact Refloat recorder firmware descriptor");

    assert_eq!(descriptor.start_address(), 0x1000_0000);
    assert_eq!(descriptor.len(), 0x0000_f800);
}

#[test]
fn recorder_descriptor_rejects_every_untrusted_boundary_failure() {
    for (words, expected) in [
        (
            [0, 0x1000_0000, 0x0000_f800],
            FirmwareDataRecorderDescriptorError::BadMagic,
        ),
        (
            [0xcafe_1111, 0x1000_0001, 0x0000_f7ff],
            FirmwareDataRecorderDescriptorError::Misaligned,
        ),
        (
            [0xcafe_1111, 0x1000_0000, 0],
            FirmwareDataRecorderDescriptorError::Undersized,
        ),
        (
            [0xcafe_1111, 0xffff_fffc, 8],
            FirmwareDataRecorderDescriptorError::AddressOverflow,
        ),
        (
            [0xcafe_1111, 0x0fff_fffc, 4],
            FirmwareDataRecorderDescriptorError::OutsideRecorderRam,
        ),
        (
            [0xcafe_1111, 0x1000_f7fc, 8],
            FirmwareDataRecorderDescriptorError::OutsideRecorderRam,
        ),
    ] {
        assert_eq!(
            FirmwareDataRecorderDescriptor::try_from_words(words),
            Err(expected),
            "descriptor {words:08x?}"
        );
    }
}
