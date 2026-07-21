use super::*;
use vescpkg_rs::prelude::AngleDegrees;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefloatTuneNibble(u8);

impl RefloatTuneNibble {
    const fn low(byte: u8) -> Self {
        Self(byte & 0x0F)
    }

    const fn high(byte: u8) -> Self {
        Self(byte >> 4)
    }

    fn angle_from(self, base: AngleDegrees) -> AngleDegrees {
        base + match self.0 {
            0 => AngleDegrees::from_degrees(0.0),
            1 => AngleDegrees::from_degrees(1.0),
            2 => AngleDegrees::from_degrees(2.0),
            3 => AngleDegrees::from_degrees(3.0),
            4 => AngleDegrees::from_degrees(4.0),
            5 => AngleDegrees::from_degrees(5.0),
            6 => AngleDegrees::from_degrees(6.0),
            7 => AngleDegrees::from_degrees(7.0),
            8 => AngleDegrees::from_degrees(8.0),
            9 => AngleDegrees::from_degrees(9.0),
            10 => AngleDegrees::from_degrees(10.0),
            11 => AngleDegrees::from_degrees(11.0),
            12 => AngleDegrees::from_degrees(12.0),
            13 => AngleDegrees::from_degrees(13.0),
            14 => AngleDegrees::from_degrees(14.0),
            15 => AngleDegrees::from_degrees(15.0),
            _ => unreachable!(),
        }
    }

    fn booster_current(self) -> MotorCurrent {
        MotorCurrent::new(match self.0 {
            0 => Current::ZERO,
            1 => Current::from_amps(10.0),
            2 => Current::from_amps(12.0),
            3 => Current::from_amps(14.0),
            4 => Current::from_amps(16.0),
            5 => Current::from_amps(18.0),
            6 => Current::from_amps(20.0),
            7 => Current::from_amps(22.0),
            8 => Current::from_amps(24.0),
            9 => Current::from_amps(26.0),
            10 => Current::from_amps(28.0),
            11 => Current::from_amps(30.0),
            12 => Current::from_amps(32.0),
            13 => Current::from_amps(34.0),
            14 => Current::from_amps(36.0),
            15 => Current::from_amps(38.0),
            _ => unreachable!(),
        })
    }
}

pub(super) fn handle_booster_packet(state: &mut RefloatPackageState, bytes: &[u8]) -> bool {
    let Some(
        [
            booster,
            booster_current,
            brake_booster,
            brake_booster_current,
        ],
    ) = refloat_command_payload(bytes, RefloatAppDataCommand::Booster)
    else {
        return false;
    };

    // C map: `cmd_booster` splits four bytes into low/high nibbles at
    // `third_party/refloat/src/main.c:1448-1481`; only the low nibble of each
    // current byte is used.
    let mut config = state.serialized_config.editor();
    let updated = [
        config.set_booster_angle(
            RefloatTuneNibble::low(*booster).angle_from(AngleDegrees::from_degrees(5.0)),
        ),
        config.set_booster_ramp(
            RefloatTuneNibble::high(*booster).angle_from(AngleDegrees::from_degrees(2.0)),
        ),
        config.set_booster_current(RefloatTuneNibble::low(*booster_current).booster_current()),
        config.set_brake_booster_angle(
            RefloatTuneNibble::low(*brake_booster).angle_from(AngleDegrees::from_degrees(5.0)),
        ),
        config.set_brake_booster_ramp(
            RefloatTuneNibble::high(*brake_booster).angle_from(AngleDegrees::from_degrees(2.0)),
        ),
        config.set_brake_booster_current(
            RefloatTuneNibble::low(*brake_booster_current).booster_current(),
        ),
    ]
    .into_iter()
    .all(core::convert::identity);
    debug_assert!(updated);
    state.alert_beeper(RefloatBeeperAlert::Short(RefloatBeeperCount::ONE));
    true
}

#[cfg(test)]
mod tests {
    use super::RefloatTuneNibble;
    use vescpkg_rs::prelude::{AngleDegrees, Current, MotorCurrent};

    #[test]
    fn tune_nibble_keeps_exact_endpoints_without_primitive_conversions() {
        assert_eq!(RefloatTuneNibble::low(0xF0), RefloatTuneNibble(0));
        assert_eq!(RefloatTuneNibble::high(0xF0), RefloatTuneNibble(15));
        assert_eq!(
            RefloatTuneNibble(0).angle_from(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(5.0),
        );
        assert_eq!(
            RefloatTuneNibble(15).angle_from(AngleDegrees::from_degrees(5.0)),
            AngleDegrees::from_degrees(20.0),
        );
        assert_eq!(
            RefloatTuneNibble(0).booster_current(),
            MotorCurrent::new(Current::ZERO),
        );
        assert_eq!(
            RefloatTuneNibble(15).booster_current(),
            MotorCurrent::new(Current::from_amps(38.0)),
        );
    }
}
