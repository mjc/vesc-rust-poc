use super::{
    FloatOutBoyHardwareLedsConfig, FloatOutBoyInternalLedLayoutError, FloatOutBoyLedMode,
    FloatOutBoyLedStripRole,
};
use crate::leds::{
    FloatOutBoyLedColorOrder, FloatOutBoyLedPin, FloatOutBoyLedPinConfig,
    FloatOutBoyLedStripConfig, FloatOutBoyLedStripOrder,
};

#[test]
fn float_out_boy_led_mode_matches_upstream_flag_ids() {
    // C map: Float Out Boy v1.2.1 treats LED mode as flags at
    // `third_party/float-out-boy/src/leds.c:795-830` and external-LCM mode details at
    // `third_party/float-out-boy/src/lcm.c:27-28`; the typed mode IDs mirror
    // `third_party/float-out-boy/src/conf/datatypes.h:36-60`.
    let disabled = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Off);
    let internal = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal);
    let external = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::External);
    let both = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Both);

    assert_eq!(FloatOutBoyLedMode::Off.id(), 0);
    assert_eq!(FloatOutBoyLedMode::Internal.id(), 0x1);
    assert_eq!(FloatOutBoyLedMode::External.id(), 0x2);
    assert_eq!(FloatOutBoyLedMode::Both.id(), 0x3);
    assert!(!disabled.uses_internal_leds());
    assert!(!disabled.uses_external_leds());
    assert!(internal.uses_internal_leds());
    assert!(!internal.uses_external_leds());
    assert!(!external.uses_internal_leds());
    assert!(external.uses_external_leds());
    assert!(both.uses_internal_leds());
    assert!(both.uses_external_leds());
}

#[test]
fn float_out_boy_hardware_leds_default_and_overrides_match_upstream_shape() {
    // C map: Float Out Boy's default hardware LED settings come from
    // `third_party/float-out-boy/src/conf/settings.xml:3560-3863`; the mode/pin/pin-config
    // wiring follows the same flags behavior as `third_party/float-out-boy/src/leds.c:795-830`
    // and `third_party/float-out-boy/src/lcm.c:27-28`.
    let defaults = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Off);

    assert_eq!(defaults.pin(), FloatOutBoyLedPin::B7);
    assert_eq!(defaults.pin_config(), FloatOutBoyLedPinConfig::PullupTo5v);
    assert_eq!(
        defaults.status_strip().order(),
        FloatOutBoyLedStripOrder::First
    );
    assert_eq!(defaults.status_strip().count(), 10);
    assert_eq!(
        defaults.front_strip().order(),
        FloatOutBoyLedStripOrder::Second
    );
    assert_eq!(defaults.front_strip().count(), 20);
    assert_eq!(
        defaults.rear_strip().order(),
        FloatOutBoyLedStripOrder::Third
    );
    assert_eq!(defaults.rear_strip().count(), 20);

    let status_strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        8,
        FloatOutBoyLedColorOrder::Grbw,
    );
    let front_strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Second,
        24,
        FloatOutBoyLedColorOrder::Rgb,
    );
    let rear_strip = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Third,
        24,
        FloatOutBoyLedColorOrder::Grb,
    )
    .reversed();

    let hardware_leds = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Both)
        .with_pin(FloatOutBoyLedPin::C9)
        .with_pin_config(FloatOutBoyLedPinConfig::NoPullup)
        .with_status_strip(status_strip)
        .with_front_strip(front_strip)
        .with_rear_strip(rear_strip);

    assert_eq!(hardware_leds.mode(), FloatOutBoyLedMode::Both);
    assert_eq!(hardware_leds.pin(), FloatOutBoyLedPin::C9);
    assert_eq!(
        hardware_leds.pin_config(),
        FloatOutBoyLedPinConfig::NoPullup
    );
    assert_eq!(
        hardware_leds.status_strip().color_order(),
        FloatOutBoyLedColorOrder::Grbw
    );
    assert_eq!(
        hardware_leds.front_strip().color_order(),
        FloatOutBoyLedColorOrder::Rgb
    );
    assert!(hardware_leds.rear_strip().is_reversed());
}

#[test]
fn internal_layout_orders_nonempty_strips_with_refloat_priority() {
    let first = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        2,
        FloatOutBoyLedColorOrder::Grb,
    );
    let second = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Second,
        3,
        FloatOutBoyLedColorOrder::Rgb,
    );
    let duplicate_first = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        4,
        FloatOutBoyLedColorOrder::Grbw,
    );
    let config = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(first)
        .with_front_strip(duplicate_first)
        .with_rear_strip(second);

    let layout = config.internal_layout().expect("valid internal layout");

    assert_eq!(
        layout.roles(),
        &[
            FloatOutBoyLedStripRole::Status,
            FloatOutBoyLedStripRole::Rear
        ]
    );
    assert_eq!(layout.offset(FloatOutBoyLedStripRole::Status), Some(0));
    assert_eq!(layout.offset(FloatOutBoyLedStripRole::Rear), Some(2));
    assert_eq!(layout.offset(FloatOutBoyLedStripRole::Front), None);
    assert_eq!(layout.pixel_count(), 5);
}

#[test]
fn internal_layout_matches_refloat_priority_for_every_order_assignment() {
    let orders = [
        FloatOutBoyLedStripOrder::None,
        FloatOutBoyLedStripOrder::First,
        FloatOutBoyLedStripOrder::Second,
        FloatOutBoyLedStripOrder::Third,
    ];

    for status_order in orders {
        for front_order in orders {
            for rear_order in orders {
                let config = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
                    .with_status_strip(FloatOutBoyLedStripConfig::new(
                        status_order,
                        2,
                        FloatOutBoyLedColorOrder::Grb,
                    ))
                    .with_front_strip(FloatOutBoyLedStripConfig::new(
                        front_order,
                        3,
                        FloatOutBoyLedColorOrder::Grb,
                    ))
                    .with_rear_strip(FloatOutBoyLedStripConfig::new(
                        rear_order,
                        5,
                        FloatOutBoyLedColorOrder::Grb,
                    ));
                let layout = config.internal_layout().expect("small layout is valid");
                let candidates = [
                    (FloatOutBoyLedStripRole::Status, status_order, 2_usize),
                    (FloatOutBoyLedStripRole::Front, front_order, 3_usize),
                    (FloatOutBoyLedStripRole::Rear, rear_order, 5_usize),
                ];
                let mut expected_roles = std::vec::Vec::new();
                let mut expected_offsets = [None; 3];
                let mut expected_count = 0;

                for order in [
                    FloatOutBoyLedStripOrder::First,
                    FloatOutBoyLedStripOrder::Second,
                    FloatOutBoyLedStripOrder::Third,
                ] {
                    if let Some((role, _, count)) = candidates
                        .iter()
                        .find(|(_, candidate, _)| *candidate == order)
                    {
                        expected_roles.push(*role);
                        let index = match role {
                            FloatOutBoyLedStripRole::Status => 0,
                            FloatOutBoyLedStripRole::Front => 1,
                            FloatOutBoyLedStripRole::Rear => 2,
                        };
                        expected_offsets[index] = Some(expected_count);
                        expected_count += count;
                    }
                }

                assert_eq!(layout.roles(), expected_roles);
                assert_eq!(
                    [
                        layout.offset(FloatOutBoyLedStripRole::Status),
                        layout.offset(FloatOutBoyLedStripRole::Front),
                        layout.offset(FloatOutBoyLedStripRole::Rear),
                    ],
                    expected_offsets
                );
                assert_eq!(layout.pixel_count(), expected_count);
            }
        }
    }
}

#[test]
fn internal_layout_rejects_only_selected_front_rear_overflow() {
    let disabled = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::None,
        255,
        FloatOutBoyLedColorOrder::Grb,
    );
    let front = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::First,
        31,
        FloatOutBoyLedColorOrder::Grb,
    );
    let rear = FloatOutBoyLedStripConfig::new(
        FloatOutBoyLedStripOrder::Second,
        30,
        FloatOutBoyLedColorOrder::Grb,
    );
    let config = FloatOutBoyHardwareLedsConfig::new(FloatOutBoyLedMode::Internal)
        .with_status_strip(disabled)
        .with_front_strip(front)
        .with_rear_strip(rear);

    assert_eq!(
        config.internal_layout(),
        Err(FloatOutBoyInternalLedLayoutError::FrontAndRearCountExceedsMaximum)
    );

    let empty = config
        .with_front_strip(disabled)
        .with_rear_strip(disabled)
        .internal_layout()
        .expect("order none omits even nonzero strips");
    assert!(empty.roles().is_empty());
    assert_eq!(empty.pixel_count(), 0);
}
