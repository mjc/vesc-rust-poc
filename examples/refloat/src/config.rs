//! Refloat generated custom-config bytes and field layout.
//!
//! This module owns the byte-level config image until the port grows a typed
//! `RefloatConfig` parser/editor for every upstream field.

/// Refloat v1.2.1 generated custom-config XML blob.
///
/// Upstream generates this from `third_party/refloat/src/conf/settings.xml` via `third_party/refloat/src/Makefile:28-31`
/// and exposes `data_refloatconfig_` through `get_cfg_xml` at
/// `third_party/refloat/src/main.c:2388-2396`.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_config_xml")
)]
#[used]
pub(crate) static REFLOAT_CONFIG_XML: [u8; 25_723] = *include_bytes!("conf/refloatconfig.dat");

/// Refloat v1.2.1 generated serialized default custom config.
///
/// Upstream `get_cfg(..., is_default=true)` allocates `RefloatConfig`, fills
/// defaults, serializes it, then frees it at `third_party/refloat/src/main.c:2335-2356`.
/// `third_party/refloat/src/Makefile:28-31` generates the format from `third_party/refloat/src/conf/settings.xml`;
/// generated `conf/confparser.h:11-12` defines signature `2427955642` and
/// serialized length `276`, while generated `conf/confparser.c:8-178` and
/// `conf/confparser.c:363-531` serialize the default values.
#[cfg_attr(
    all(not(test), target_arch = "arm"),
    unsafe(link_section = ".text.refloat_default_config")
)]
#[used]
pub(crate) static REFLOAT_DEFAULT_CONFIG: [u8; 276] = *include_bytes!("conf/default_config.dat");
pub(crate) const REFLOAT_CONFIG_SIGNATURE_BYTES: [u8; 4] = [0x90, 0xb7, 0xa9, 0xba];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct RefloatConfigOffset(usize);

impl RefloatConfigOffset {
    pub(crate) const fn new(offset: usize) -> Self {
        Self(offset)
    }

    pub(crate) const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub(crate) struct RefloatConfigScale(f32);

impl RefloatConfigScale {
    pub(crate) const fn new(scale: f32) -> Self {
        Self(scale)
    }

    pub(crate) const fn get(self) -> f32 {
        self.0
    }
}

/// Typed view of one generated Refloat float16 config field.
///
/// Source map: upstream generated serialization order and scales come from
/// `third_party/refloat/src/conf/settings.xml:3916-3923`; generated C reads
/// these fields through `third_party/refloat/src/conf/confparser.c:363-531`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RefloatScaledConfigField {
    pub(crate) offset: RefloatConfigOffset,
    pub(crate) scale: RefloatConfigScale,
}

impl RefloatScaledConfigField {
    pub(crate) const fn new(offset: usize, scale: f32) -> Self {
        Self {
            offset: RefloatConfigOffset::new(offset),
            scale: RefloatConfigScale::new(scale),
        }
    }
}

// Upstream serializes `kp` as the first float16 config value after the
// signature; `third_party/refloat/src/conf/settings.xml:28-54` uses scale 10.
pub(crate) const REFLOAT_CONFIG_KP_OFFSET: usize = 4;
pub(crate) const REFLOAT_CONFIG_KP_FIELD: RefloatScaledConfigField =
    RefloatScaledConfigField::new(REFLOAT_CONFIG_KP_OFFSET, 10.0);
// Upstream serializes `kp2` immediately after `kp` in
// `third_party/refloat/src/conf/settings.xml:3916-3918`; `third_party/refloat/src/conf/settings.xml:55-84` uses scale 100.
pub(crate) const REFLOAT_CONFIG_KP2_OFFSET: usize = 6;
pub(crate) const REFLOAT_CONFIG_KP2_FIELD: RefloatScaledConfigField =
    RefloatScaledConfigField::new(REFLOAT_CONFIG_KP2_OFFSET, 100.0);
// Upstream serializes `ki` after `kp` and `kp2` in
// `third_party/refloat/src/conf/settings.xml:3916-3919`; `third_party/refloat/src/conf/settings.xml:85-111` uses
// scale 100000.
pub(crate) const REFLOAT_CONFIG_KI_OFFSET: usize = 8;
pub(crate) const REFLOAT_CONFIG_KI_FIELD: RefloatScaledConfigField =
    RefloatScaledConfigField::new(REFLOAT_CONFIG_KI_OFFSET, 100_000.0);
// Upstream serializes Mahony pitch/roll KP after `ki` at
// `third_party/refloat/src/conf/settings.xml:3916-3921`; both use scale 10000 and feed
// `balance_filter_configure` at `third_party/refloat/src/balance_filter.c:64-70`.
pub(crate) const REFLOAT_CONFIG_MAHONY_KP_OFFSET: usize = 10;
pub(crate) const REFLOAT_CONFIG_MAHONY_KP_ROLL_OFFSET: usize = 12;
// Upstream serializes `kp_brake` and `kp2_brake` after the two Mahony tuning
// values at `third_party/refloat/src/conf/settings.xml:3916-3923`; both use scale 100.
pub(crate) const REFLOAT_CONFIG_KP_BRAKE_OFFSET: usize = 14;
pub(crate) const REFLOAT_CONFIG_KP_BRAKE_FIELD: RefloatScaledConfigField =
    RefloatScaledConfigField::new(REFLOAT_CONFIG_KP_BRAKE_OFFSET, 100.0);
pub(crate) const REFLOAT_CONFIG_KP2_BRAKE_OFFSET: usize = 16;
pub(crate) const REFLOAT_CONFIG_KP2_BRAKE_FIELD: RefloatScaledConfigField =
    RefloatScaledConfigField::new(REFLOAT_CONFIG_KP2_BRAKE_OFFSET, 100.0);
// Upstream defines `hertz` in `third_party/refloat/src/conf/settings.xml:223-246`, serializes it
// after the first seven `SerOrder` float16 entries at
// `third_party/refloat/src/conf/settings.xml:3916-3923`, and reads it as a big-endian uint16 via
// `third_party/refloat/src/conf/buffer.c:188-191`.
pub(crate) const REFLOAT_CONFIG_HERTZ_OFFSET: usize = 18;
pub(crate) const REFLOAT_CONFIG_FAULT_PITCH_OFFSET: usize = 20;
pub(crate) const REFLOAT_CONFIG_FAULT_ROLL_OFFSET: usize = 22;
// Upstream serializes `fault_adc1` and `fault_adc2` immediately after
// `fault_roll` at `third_party/refloat/src/conf/settings.xml:3927-3928`;
// both values use `<vTxDoubleScale>1000</vTxDoubleScale>`.
#[cfg(any(test, target_arch = "arm"))]
pub(crate) const REFLOAT_CONFIG_FAULT_ADC1_OFFSET: usize = 24;
#[cfg(any(test, target_arch = "arm"))]
pub(crate) const REFLOAT_CONFIG_FAULT_ADC2_OFFSET: usize = 26;
// Upstream defines `fault_is_dual_switch` in `third_party/refloat/src/conf/settings.xml:454-467`;
// its `<ser>fault_is_dual_switch</ser>` entry at
// `third_party/refloat/src/conf/settings.xml:3935` follows the first seven float16 values,
// hertz, four fault float16 values, footbeep, five uint16 fault fields, and
// lands at byte 39 in the 276-byte generated config image.
// `fault_delay_pitch` and `fault_delay_roll` are the uint16 fields at
// `third_party/refloat/src/conf/settings.xml:3930-3931`.
pub(crate) const REFLOAT_CONFIG_FAULT_DELAY_PITCH_OFFSET: usize = 29;
pub(crate) const REFLOAT_CONFIG_FAULT_DELAY_ROLL_OFFSET: usize = 31;
// `fault_delay_switch_half`, `fault_delay_switch_full`, and
// `fault_adc_half_erpm` are the preceding uint16 fields at
// `third_party/refloat/src/conf/settings.xml:3932-3934`.
pub(crate) const REFLOAT_CONFIG_FAULT_DELAY_SWITCH_HALF_OFFSET: usize = 33;
pub(crate) const REFLOAT_CONFIG_FAULT_DELAY_SWITCH_FULL_OFFSET: usize = 35;
pub(crate) const REFLOAT_CONFIG_FAULT_ADC_HALF_ERPM_OFFSET: usize = 37;
pub(crate) const REFLOAT_CONFIG_FAULT_IS_DUAL_SWITCH_OFFSET: usize = 39;
pub(crate) const REFLOAT_CONFIG_FAULT_MOVING_FAULT_DISABLED_OFFSET: usize = 40;
// Upstream defines `enable_quickstop` in `third_party/refloat/src/conf/settings.xml:482-493`;
// its `<ser>enable_quickstop</ser>` entry at `third_party/refloat/src/conf/settings.xml:3937`
// lands two bools after `fault_is_dual_switch`.
pub(crate) const REFLOAT_CONFIG_ENABLE_QUICKSTOP_OFFSET: usize = 41;
// Upstream defines `fault_darkride_enabled` in
// `third_party/refloat/src/conf/settings.xml:494-507`; its
// `<ser>fault_darkride_enabled</ser>` entry at
// `third_party/refloat/src/conf/settings.xml:3938` lands after
// `enable_quickstop`.
pub(crate) const REFLOAT_CONFIG_FAULT_DARKRIDE_ENABLED_OFFSET: usize = 42;
// Upstream defines `fault_reversestop_enabled` immediately after
// `fault_darkride_enabled` at `third_party/refloat/src/conf/settings.xml:3939`.
pub(crate) const REFLOAT_CONFIG_FAULT_REVERSESTOP_ENABLED_OFFSET: usize = 43;
// Upstream serializes remote throttle fields immediately before startup
// tolerances at `third_party/refloat/src/conf/settings.xml:3962-3965`.
pub(crate) const REFLOAT_CONFIG_INPUTTILT_INVERT_THROTTLE_OFFSET: usize = 84;
pub(crate) const REFLOAT_CONFIG_REMOTE_THROTTLE_CURRENT_MAX_OFFSET: usize = 87;
pub(crate) const REFLOAT_CONFIG_REMOTE_THROTTLE_GRACE_PERIOD_OFFSET: usize = 89;
// Upstream defines `ki_limit` in
// `third_party/refloat/src/conf/settings.xml:1679-1707`; its
// `<ser>ki_limit</ser>` entry at
// `third_party/refloat/src/conf/settings.xml:3975` lands at byte 104 in the
// generated default config image and uses scale 10.
// Upstream serializes startup pitch/roll tolerances and startup speed at
// `third_party/refloat/src/conf/settings.xml:3966-3968`; all use scale 100.
pub(crate) const REFLOAT_CONFIG_STARTUP_PITCH_TOLERANCE_OFFSET: usize = 91;
pub(crate) const REFLOAT_CONFIG_STARTUP_ROLL_TOLERANCE_OFFSET: usize = 93;
pub(crate) const REFLOAT_CONFIG_STARTUP_SPEED_OFFSET: usize = 95;
// Upstream serializes `startup_simplestart_enabled` at
// `third_party/refloat/src/conf/settings.xml:3970` immediately before
// push-start.
pub(crate) const REFLOAT_CONFIG_STARTUP_SIMPLESTART_ENABLED_OFFSET: usize = 99;
// Upstream serializes `startup_pushstart_enabled` at
// `third_party/refloat/src/conf/settings.xml:3971` after the startup
// speed/click fields.
pub(crate) const REFLOAT_CONFIG_STARTUP_PUSHSTART_ENABLED_OFFSET: usize = 100;
// Upstream serializes default parking brake mode and brake current at
// `third_party/refloat/src/conf/settings.xml:3973-3974`; the generated default
// config stores `PARKING_BRAKE_IDLE` at byte 101 and 6.00A at bytes 102..104.
pub(crate) const REFLOAT_CONFIG_PARKING_BRAKE_MODE_OFFSET: usize = 101;
pub(crate) const REFLOAT_CONFIG_BRAKE_CURRENT_OFFSET: usize = 102;
pub(crate) const REFLOAT_CONFIG_KI_LIMIT_OFFSET: usize = 104;
// Upstream serializes booster angle, ramp, and current immediately after
// `ki_limit` at `third_party/refloat/src/conf/settings.xml:3975-3978`; all
// three use scale 100.
pub(crate) const REFLOAT_CONFIG_BOOSTER_ANGLE_OFFSET: usize = 106;
pub(crate) const REFLOAT_CONFIG_BOOSTER_RAMP_OFFSET: usize = 108;
pub(crate) const REFLOAT_CONFIG_BOOSTER_CURRENT_OFFSET: usize = 110;
pub(crate) const REFLOAT_CONFIG_BRKBOOSTER_ANGLE_OFFSET: usize = 112;
pub(crate) const REFLOAT_CONFIG_BRKBOOSTER_RAMP_OFFSET: usize = 114;
pub(crate) const REFLOAT_CONFIG_BRKBOOSTER_CURRENT_OFFSET: usize = 116;
// Upstream serializes tune modifiers in
// `third_party/refloat/src/conf/settings.xml:3952-3996`; `cmd_handtest` zeros
// these fields at `third_party/refloat/src/main.c:1437-1443`.
pub(crate) const REFLOAT_CONFIG_TILTBACK_CONSTANT_OFFSET: usize = 67;
pub(crate) const REFLOAT_CONFIG_TILTBACK_VARIABLE_OFFSET: usize = 71;
pub(crate) const REFLOAT_CONFIG_TORQUETILT_STRENGTH_OFFSET: usize = 126;
pub(crate) const REFLOAT_CONFIG_TORQUETILT_STRENGTH_REGEN_OFFSET: usize = 128;
pub(crate) const REFLOAT_CONFIG_TURNTILT_STRENGTH_OFFSET: usize = 130;
pub(crate) const REFLOAT_CONFIG_ATR_STRENGTH_UP_OFFSET: usize = 145;
pub(crate) const REFLOAT_CONFIG_ATR_STRENGTH_DOWN_OFFSET: usize = 147;

// Upstream defines `disabled` in `third_party/refloat/src/conf/settings.xml:3890-3902`; its
// `<ser>disabled</ser>` entry at `third_party/refloat/src/conf/settings.xml:4064` lands at byte
// 243 in the 276-byte generated config image.
pub(crate) const REFLOAT_CONFIG_DISABLED_OFFSET: usize = 243;
// Upstream defines `meta.is_default` in `third_party/refloat/src/conf/settings.xml:3903-3914`; its
// `<ser>meta.is_default</ser>` entry at `third_party/refloat/src/conf/settings.xml:4083` lands at
// the final byte in the generated config image.
pub(crate) const REFLOAT_CONFIG_META_IS_DEFAULT_OFFSET: usize = 275;
pub(crate) fn refloat_read_scaled_i16(bytes: [u8; 2], scale: f32) -> f32 {
    i16::from_be_bytes(bytes) as f32 / scale
}
