#![cfg_attr(test, allow(dead_code))]

use crate::{AppDataHandler, ExtensionHandler, LbmValue, VescIfAbi, VescIfPresence};
use core::ffi::{c_char, c_int, c_uchar, c_uint, c_void};

/// Bindgen-generated `lbm_flat_value_t` layout.
pub type LbmFlatValue = crate::bindgen::lbm_flat_value_t;
/// Bindgen-generated `lbm_array_header_t` layout.
pub type LbmArrayHeader = crate::bindgen::lbm_array_header_t;
/// Bindgen-generated `eeprom_var` layout.
pub type EepromVar = crate::bindgen::eeprom_var;
/// Bindgen-generated `can_status_msg` layout.
pub type CanStatusMsg = crate::bindgen::can_status_msg;
/// Bindgen-generated `can_status_msg_2` layout.
pub type CanStatusMsg2 = crate::bindgen::can_status_msg_2;
/// Bindgen-generated `can_status_msg_3` layout.
pub type CanStatusMsg3 = crate::bindgen::can_status_msg_3;
/// Bindgen-generated `can_status_msg_4` layout.
pub type CanStatusMsg4 = crate::bindgen::can_status_msg_4;
/// Bindgen-generated `can_status_msg_5` layout.
pub type CanStatusMsg5 = crate::bindgen::can_status_msg_5;
/// Bindgen-generated `can_status_msg_6` layout.
pub type CanStatusMsg6 = crate::bindgen::can_status_msg_6;
/// Bindgen-generated `gnss_data` layout.
pub type GnssData = crate::bindgen::gnss_data;
/// Bindgen-generated `ATTITUDE_INFO` layout.
pub type AttitudeInfo = crate::bindgen::ATTITUDE_INFO;
/// Bindgen-generated `remote_state` layout.
pub type RemoteState = crate::bindgen::remote_state;
/// Bindgen-generated `PACKET_STATE_t` layout.
pub type PacketState = crate::bindgen::PACKET_STATE_t;

/// VESC CAN receive callback ABI.
pub type CanReceiverCallback = unsafe extern "C" fn(u32, *mut u8, u8) -> bool;
type CanRxCallback = CanReceiverCallback;

type LibThread = crate::bindgen::lib_thread;
type LibMutex = crate::bindgen::lib_mutex;
type LibSemaphore = crate::bindgen::lib_semaphore;
/// Float Out Boy/VESC Tool custom-config serializer callback.
///
/// Float Out Boy `v1.2.1` passes `get_cfg` to `conf_custom_add_config` in
/// `src/main.c:2456`; the callback is declared in
/// `vesc_pkg_lib/vesc_c_if.h:549-550`.
pub type CustomConfigGet = unsafe extern "C" fn(*mut u8, bool) -> c_int;
/// Float Out Boy/VESC Tool custom-config deserializer callback.
///
/// Float Out Boy `v1.2.1` passes `set_cfg` to `conf_custom_add_config` in
/// `src/main.c:2456`; the callback is declared in
/// `vesc_pkg_lib/vesc_c_if.h:551`.
pub type CustomConfigSet = unsafe extern "C" fn(*mut u8) -> bool;
/// Float Out Boy/VESC Tool custom-config XML callback.
///
/// Float Out Boy `v1.2.1` passes `get_cfg_xml` to `conf_custom_add_config` in
/// `src/main.c:2456`; the callback is declared in
/// `vesc_pkg_lib/vesc_c_if.h:552`.
pub type CustomConfigXml = unsafe extern "C" fn(*mut *mut u8) -> c_int;
/// Float Out Boy/VESC IMU read callback.
///
/// Float Out Boy `v1.2.1` registers `imu_ref_callback` with this slot at
/// `src/main.c:2455`; the callback itself updates the balance filter at
/// `src/main.c:760-764`.
pub type ImuReadCallback = unsafe extern "C" fn(*mut f32, *mut f32, *mut f32, f32);

/// Raw firmware function table generated from the pinned VESC package header.
///
/// This alias deliberately keeps bindgen's names private to the sys crate while
/// making the C header the single source of field order and signatures.
pub type VescIf = crate::bindgen::vesc_c_if;

impl VescIf {
    /// Inspect this host-side table without exposing its raw fields.
    pub fn presence(&self) -> VescIfPresence {
        crate::c_vesc_if::presence(self)
    }
}

#[cfg(target_pointer_width = "32")]
const _: () = {
    assert!(core::mem::size_of::<VescIf>() == VescIfAbi::FIELD_COUNT * 4);
    assert!(
        core::mem::offset_of!(VescIf, lbm_enc_sym_nil)
            == VescIfAbi::LBM_ENC_SYM_NIL.vesc32_byte_offset()
    );
    assert!(
        core::mem::offset_of!(VescIf, shutdown_disable)
            == VescIfAbi::SHUTDOWN_DISABLE.vesc32_byte_offset()
    );
};

/// Inspect a target table while bounding reads to the caller-provided table width.
///
/// Forward a NUL-terminated message through the firmware's `%s` formatter.
///
/// The wrapper never treats message bytes as a format string.
///
/// # Safety
///
/// `message` must point to a readable, NUL-terminated C string for the duration
/// of the firmware call.
pub unsafe fn printf_data(message: *const c_char) -> bool {
    let Some(printf) = (unsafe { slots::printf() }) else {
        return false;
    };
    static FORMAT: &[u8] = b"%s\0";
    unsafe {
        printf(FORMAT.as_ptr().cast(), message);
    }
    true
}

/// # Safety
///
/// `base` must point to at least `available_slots` contiguous pointer-sized ABI words.
pub unsafe fn vesc_if_presence_from(base: usize, available_slots: usize) -> VescIfPresence {
    let slot_count = core::cmp::min(available_slots, VescIfAbi::FIELD_COUNT);
    let words = unsafe { core::slice::from_raw_parts(base as *const usize, slot_count) };
    VescIfPresence::from_words(words)
}

#[cfg(not(all(target_arch = "arm", not(test))))]
#[inline(always)]
unsafe fn vesc_if() -> *const VescIf {
    #[cfg(test)]
    if let Some(table) = crate::test_support::current_table() {
        return table;
    }
    VescIfAbi::BASE_ADDR.0 as *const VescIf
}

#[cfg(all(target_arch = "arm", not(test)))]
#[inline(always)]
unsafe fn load_vesc_if_word_from<const OFFSET: usize>(vesc_if: usize) -> usize {
    let word: usize;
    unsafe {
        core::arch::asm!(
            "ldr {word}, [{vesc_if}, #{offset}]",
            vesc_if = in(reg) vesc_if,
            word = out(reg) word,
            offset = const OFFSET,
            options(nostack, preserves_flags),
        );
    }
    word
}

#[cfg(all(target_arch = "arm", not(test)))]
macro_rules! vesc_slot_word_from {
    ($vesc_if:expr, $name:ident) => {
        crate::raw::load_vesc_if_word_from::<{ core::mem::offset_of!(crate::raw::VescIf, $name) }>(
            $vesc_if as usize,
        )
    };
}

mod slots {
    use super::{
        AppDataHandler, CanRxCallback, CanStatusMsg, CanStatusMsg2, CanStatusMsg3, CanStatusMsg4, CanStatusMsg5,
        CanStatusMsg6, CustomConfigGet, CustomConfigSet, CustomConfigXml, EepromVar,
        ExtensionHandler, GnssData, HwType, ImuReadCallback, LbmFlatValue, LbmValue, LibMutex, LibSemaphore,
        LibThread, RemoteState, VescIfAbi, c_char, c_int, c_uchar, c_uint, c_void,
    };
    #[cfg(not(all(target_arch = "arm", not(test))))]
    use super::{VescIf, vesc_if};

    macro_rules! word_slot {
        ($name:ident) => {
            #[inline(always)]
            pub(super) unsafe fn $name() -> usize {
                #[cfg(all(target_arch = "arm", not(test)))]
                unsafe {
                    vesc_slot_word_from!(VescIfAbi::BASE_ADDR.0, $name)
                }

                #[cfg(not(all(target_arch = "arm", not(test))))]
                unsafe {
                    (*vesc_if()).$name as usize
                }
            }
        };
    }

    macro_rules! fn_slot {
        ($name:ident as $fn_ty:ty) => {
            #[inline(always)]
            pub(super) unsafe fn $name() -> Option<$fn_ty> {
                #[cfg(all(target_arch = "arm", not(test)))]
                unsafe {
                    let address = vesc_slot_word_from!(VescIfAbi::BASE_ADDR.0, $name);
                    if address == 0 {
                        None
                    } else {
                        Some(core::mem::transmute::<usize, $fn_ty>(address))
                    }
                }

                #[cfg(not(all(target_arch = "arm", not(test))))]
                unsafe {
                    (*vesc_if()).$name
                }
            }
        };
    }

    #[cfg(all(target_arch = "arm", not(test)))]
    #[inline(always)]
    pub(super) unsafe fn lbm_add_extension_from(
        vesc_if_base: usize,
    ) -> Option<unsafe extern "C" fn(*mut c_char, crate::bindgen::extension_fptr) -> bool> {
        let address = unsafe { vesc_slot_word_from!(vesc_if_base, lbm_add_extension) };
        if address == 0 {
            None
        } else {
            Some(unsafe {
                core::mem::transmute::<
                    usize,
                    unsafe extern "C" fn(*mut c_char, crate::bindgen::extension_fptr) -> bool,
                >(address)
            })
        }
    }

    #[cfg(not(all(target_arch = "arm", not(test))))]
    #[inline(always)]
    pub(super) unsafe fn lbm_add_extension_from(
        vesc_if_base: usize,
    ) -> Option<unsafe extern "C" fn(*mut c_char, crate::bindgen::extension_fptr) -> bool> {
        let table = if vesc_if_base == VescIfAbi::BASE_ADDR.0 {
            unsafe { vesc_if() }
        } else {
            vesc_if_base as *const VescIf
        };
        unsafe { (*table).lbm_add_extension }
    }

    fn_slot!(lbm_dec_as_float as unsafe extern "C" fn(u32) -> f32);
    fn_slot!(lbm_dec_as_u32 as unsafe extern "C" fn(u32) -> u32);
    fn_slot!(lbm_dec_as_i32 as unsafe extern "C" fn(u32) -> i32);
    fn_slot!(lbm_dec_char as unsafe extern "C" fn(u32) -> u8);
    fn_slot!(lbm_enc_i as unsafe extern "C" fn(i32) -> u32);
    fn_slot!(lbm_enc_char as unsafe extern "C" fn(u8) -> u32);
    fn_slot!(lbm_enc_float as unsafe extern "C" fn(f32) -> u32);
    fn_slot!(lbm_enc_u32 as unsafe extern "C" fn(u32) -> u32);
    fn_slot!(lbm_dec_str as unsafe extern "C" fn(u32) -> *mut c_char);
    fn_slot!(lbm_is_number as unsafe extern "C" fn(u32) -> bool);
    fn_slot!(lbm_is_char as unsafe extern "C" fn(u32) -> bool);
    fn_slot!(lbm_is_symbol as unsafe extern "C" fn(u32) -> bool);
    fn_slot!(lbm_is_cons as unsafe extern "C" fn(u32) -> bool);
    fn_slot!(lbm_is_byte_array as unsafe extern "C" fn(u32) -> bool);
    fn_slot!(lbm_cons as unsafe extern "C" fn(u32, u32) -> u32);
    fn_slot!(lbm_car as unsafe extern "C" fn(u32) -> u32);
    fn_slot!(lbm_cdr as unsafe extern "C" fn(u32) -> u32);
    fn_slot!(lbm_list_destructive_reverse as unsafe extern "C" fn(u32) -> u32);
    fn_slot!(lbm_create_byte_array as unsafe extern "C" fn(*mut LbmValue, u32) -> bool);
    fn_slot!(lbm_enc_sym as unsafe extern "C" fn(u32) -> LbmValue);
    fn_slot!(lbm_dec_sym as unsafe extern "C" fn(LbmValue) -> u32);
    fn_slot!(lbm_send_message as unsafe extern "C" fn(u32, LbmValue) -> c_int);
    fn_slot!(lbm_get_current_cid as unsafe extern "C" fn() -> u32);
    fn_slot!(lbm_block_ctx_from_extension as unsafe extern "C" fn());
    optional_fn_slot!(lbm_unblock_ctx as unsafe extern "C" fn(u32, *mut LbmFlatValue) -> bool);
    optional_fn_slot!(lbm_unblock_ctx_unboxed as unsafe extern "C" fn(u32, LbmValue) -> bool);
    optional_fn_slot!(lbm_start_flatten as unsafe extern "C" fn(*mut LbmFlatValue, usize) -> bool);
    optional_fn_slot!(lbm_finish_flatten as unsafe extern "C" fn(*mut LbmFlatValue) -> bool);
    optional_fn_slot!(f_cons as unsafe extern "C" fn(*mut LbmFlatValue) -> bool);
    optional_fn_slot!(f_sym as unsafe extern "C" fn(*mut LbmFlatValue, u32) -> bool);
    optional_fn_slot!(f_b as unsafe extern "C" fn(*mut LbmFlatValue, u8) -> bool);
    optional_fn_slot!(f_i32 as unsafe extern "C" fn(*mut LbmFlatValue, i32) -> bool);
    optional_fn_slot!(f_u32 as unsafe extern "C" fn(*mut LbmFlatValue, u32) -> bool);
    optional_fn_slot!(f_float as unsafe extern "C" fn(*mut LbmFlatValue, f32) -> bool);
    optional_fn_slot!(f_i64 as unsafe extern "C" fn(*mut LbmFlatValue, i64) -> bool);
    optional_fn_slot!(f_u64 as unsafe extern "C" fn(*mut LbmFlatValue, u64) -> bool);
    optional_fn_slot!(f_lbm_array as unsafe extern "C" fn(*mut LbmFlatValue, u32, *mut u8) -> bool);
    fn_slot!(set_app_data_handler as unsafe extern "C" fn(Option<AppDataHandler>) -> bool);
    fn_slot!(imu_set_read_callback as unsafe extern "C" fn(Option<ImuReadCallback>));
    fn_slot!(read_eeprom_var as unsafe extern "C" fn(*mut EepromVar, c_int) -> bool);
    fn_slot!(store_eeprom_var as unsafe extern "C" fn(*mut EepromVar, c_int) -> bool);

    word_slot!(lbm_enc_sym_nil);
    word_slot!(lbm_enc_sym_true);
    word_slot!(lbm_enc_sym_eerror);

    fn_slot!(
        conf_custom_add_config
            as unsafe extern "C" fn(
                Option<CustomConfigGet>,
                Option<CustomConfigSet>,
                Option<CustomConfigXml>,
            )
    );
    fn_slot!(conf_custom_clear_configs as unsafe extern "C" fn());
    fn_slot!(mutex_create as unsafe extern "C" fn() -> LibMutex);
    fn_slot!(mutex_lock as unsafe extern "C" fn(LibMutex));
    fn_slot!(mutex_unlock as unsafe extern "C" fn(LibMutex));
    fn_slot!(sem_create as unsafe extern "C" fn() -> LibSemaphore);
    fn_slot!(sem_wait as unsafe extern "C" fn(LibSemaphore));
    fn_slot!(sem_signal as unsafe extern "C" fn(LibSemaphore));
    fn_slot!(sem_wait_to as unsafe extern "C" fn(LibSemaphore, u32) -> bool);
    fn_slot!(sem_reset as unsafe extern "C" fn(LibSemaphore));
    fn_slot!(malloc as unsafe extern "C" fn(usize) -> *mut c_void);
    fn_slot!(free as unsafe extern "C" fn(*mut c_void));
    fn_slot!(sleep_us as unsafe extern "C" fn(u32));
    fn_slot!(
        spawn
            as unsafe extern "C" fn(
                Option<unsafe extern "C" fn(*mut c_void)>,
                usize,
                *const c_char,
                *mut c_void,
            ) -> LibThread
    );
    fn_slot!(request_terminate as unsafe extern "C" fn(LibThread));
    fn_slot!(should_terminate as unsafe extern "C" fn() -> bool);
    fn_slot!(get_arg as unsafe extern "C" fn(u32) -> *mut *mut c_void);
    optional_fn_slot!(can_get_status_msg_index as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg);
    optional_fn_slot!(can_get_status_msg_id as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg);
    optional_fn_slot!(can_transmit_sid as unsafe extern "C" fn(u32, *const u8, u8));
    optional_fn_slot!(can_transmit_eid as unsafe extern "C" fn(u32, *const u8, u8));
    optional_fn_slot!(can_set_sid_cb as unsafe extern "C" fn(Option<CanRxCallback>));
    optional_fn_slot!(can_set_eid_cb as unsafe extern "C" fn(Option<CanRxCallback>));
    optional_fn_slot!(can_set_duty as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_set_current as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_set_current_off_delay as unsafe extern "C" fn(u8, f32, f32));
    optional_fn_slot!(can_set_current_brake as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_set_current_rel as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_set_current_rel_off_delay as unsafe extern "C" fn(u8, f32, f32));
    optional_fn_slot!(can_set_current_brake_rel as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_set_rpm as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_set_pos as unsafe extern "C" fn(u8, f32));
    optional_fn_slot!(can_ping as unsafe extern "C" fn(u8, *mut HwType) -> bool);
    optional_fn_slot!(
        can_get_status_msg_2_index as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg2
    );
    optional_fn_slot!(can_get_status_msg_2_id as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg2);
    optional_fn_slot!(
        can_get_status_msg_3_index as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg3
    );
    optional_fn_slot!(can_get_status_msg_3_id as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg3);
    optional_fn_slot!(
        can_get_status_msg_4_index as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg4
    );
    optional_fn_slot!(can_get_status_msg_4_id as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg4);
    optional_fn_slot!(
        can_get_status_msg_5_index as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg5
    );
    optional_fn_slot!(can_get_status_msg_5_id as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg5);
    optional_fn_slot!(
        can_get_status_msg_6_index as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg6
    );
    optional_fn_slot!(can_get_status_msg_6_id as unsafe extern "C" fn(c_int) -> *mut CanStatusMsg6);
    optional_fn_slot!(mc_gnss as unsafe extern "C" fn() -> *mut GnssData);
    optional_fn_slot!(store_backup_data as unsafe extern "C" fn() -> bool);
    optional_fn_slot!(get_ppm as unsafe extern "C" fn() -> f32);
    optional_fn_slot!(get_ppm_age as unsafe extern "C" fn() -> f32);
    optional_fn_slot!(app_is_output_disabled as unsafe extern "C" fn() -> bool);
    optional_fn_slot!(read_nvm as unsafe extern "C" fn(*mut u8, c_uint, c_uint) -> bool);
    optional_fn_slot!(write_nvm as unsafe extern "C" fn(*mut u8, c_uint, c_uint) -> bool);
    optional_fn_slot!(wipe_nvm as unsafe extern "C" fn() -> bool);
    fn_slot!(get_remote_state as unsafe extern "C" fn() -> RemoteState);
    fn_slot!(get_ppm as unsafe extern "C" fn() -> f32);
    fn_slot!(get_ppm_age as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_fault as unsafe extern "C" fn() -> c_uint);
    fn_slot!(mc_fault_to_string as unsafe extern "C" fn(c_uint) -> *const c_char);
    fn_slot!(mc_get_rpm as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_speed as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_directional_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_directional as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_in_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tot_current_in as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_duty_cycle_now as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_sampling_frequency_now as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_tachometer_value as unsafe extern "C" fn(bool) -> c_int);
    fn_slot!(mc_get_tachometer_abs_value as unsafe extern "C" fn(bool) -> c_int);
    fn_slot!(mc_stat_power_avg as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_stat_power_max as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_stat_reset as unsafe extern "C" fn());
    fn_slot!(mc_get_input_voltage_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_amp_hours as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_amp_hours_charged as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_watt_hours as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_watt_hours_charged as unsafe extern "C" fn(bool) -> f32);
    fn_slot!(mc_get_battery_level as unsafe extern "C" fn(*mut f32) -> f32);
    fn_slot!(mc_get_distance_abs as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_get_odometer as unsafe extern "C" fn() -> u64);
    fn_slot!(get_cfg_float as unsafe extern "C" fn(c_uint) -> f32);
    fn_slot!(get_cfg_int as unsafe extern "C" fn(c_uint) -> c_int);
    fn_slot!(mc_set_duty as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_current as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_current_off_delay as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_brake_current as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_handbrake as unsafe extern "C" fn(f32));
    fn_slot!(mc_set_handbrake_rel as unsafe extern "C" fn(f32));
    fn_slot!(mc_release_motor as unsafe extern "C" fn());
    fn_slot!(mc_wait_for_motor_release as unsafe extern "C" fn(f32) -> bool);
    fn_slot!(timeout_reset as unsafe extern "C" fn());
    fn_slot!(timeout_has_timeout as unsafe extern "C" fn() -> bool);
    fn_slot!(timeout_secs_since_update as unsafe extern "C" fn() -> f32);
    // Refloat capability-probes this pre-6.05 slot because not every motor
    // implementation populates the FOC-specific function.
    fn_slot!(foc_get_id as unsafe extern "C" fn() -> f32);
    fn_slot!(foc_play_tone as unsafe extern "C" fn(c_int, f32, f32) -> bool);
    fn_slot!(mc_temp_fet_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(mc_temp_motor_filtered as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_startup_done as unsafe extern "C" fn() -> bool);
    fn_slot!(imu_get_roll as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_get_pitch as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_get_yaw as unsafe extern "C" fn() -> f32);
    fn_slot!(imu_get_gyro as unsafe extern "C" fn(*mut f32));
    fn_slot!(imu_get_quaternions as unsafe extern "C" fn(*mut f32));
    fn_slot!(send_app_data as unsafe extern "C" fn(*mut c_uchar, u32));
    fn_slot!(system_time as unsafe extern "C" fn() -> f32);
    fn_slot!(ts_to_age_s as unsafe extern "C" fn(u32) -> f32);
    optional_fn_slot!(printf as unsafe extern "C" fn(*const c_char, ...) -> c_int);
    fn_slot!(timer_time_now as unsafe extern "C" fn() -> u32);
    fn_slot!(timer_seconds_elapsed_since as unsafe extern "C" fn(u32) -> f32);
    // Appended in firmware 6.05; older tables fall back to `system_time`.
    fn_slot!(system_time_ticks as unsafe extern "C" fn() -> u32);
    // Appended in firmware 6.06; callers treat absence as an unsupported hint.
    fn_slot!(thread_set_priority as unsafe extern "C" fn(c_int));
    fn_slot!(io_set_mode as unsafe extern "C" fn(c_uint, c_uint) -> bool);
    fn_slot!(io_write as unsafe extern "C" fn(c_uint, c_int) -> bool);
    fn_slot!(io_read as unsafe extern "C" fn(c_uint) -> bool);
    fn_slot!(io_read_analog as unsafe extern "C" fn(c_uint) -> f32);
}

#[track_caller]
fn required_slot<T>(slot: Option<T>) -> T {
    slot.expect("required VESC_IF slot is unavailable")
}

macro_rules! required_slot {
    ($name:ident) => {
        required_slot(slots::$name())
    };
}

/// # Safety
///
/// `name` must point to a valid, NUL-terminated extension name and
/// `handler` must use the firmware LispBM extension ABI.
pub unsafe fn lbm_add_extension(name: *const c_char, handler: ExtensionHandler) -> bool {
    unsafe { lbm_add_extension_with_table_base(VescIfAbi::BASE_ADDR.0 as u32, name, handler) }
}

/// # Safety
///
/// `vesc_if_base` must be the firmware VESC function table address and
/// `name`/`handler` must satisfy the same requirements as
/// [`lbm_add_extension`].
#[inline(always)]
pub unsafe fn lbm_add_extension_with_table_base(
    vesc_if_base: u32,
    name: *const c_char,
    handler: ExtensionHandler,
) -> bool {
    let slot = unsafe { slots::lbm_add_extension_from(vesc_if_base as usize) };
    let lbm_add_extension = required_slot(slot);
    unsafe { lbm_add_extension(name as *mut c_char, Some(handler)) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_dec_as_float(value: LbmValue) -> f32 {
    unsafe { required_slot!(lbm_dec_as_float)(value.0) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_float(value: f32) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_enc_float)(value)) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_dec_as_u32(value: LbmValue) -> u32 {
    unsafe { required_slot!(lbm_dec_as_u32)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
    unsafe { required_slot!(lbm_dec_as_i32)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM character value supplied by the firmware.
pub unsafe fn lbm_dec_char(value: LbmValue) -> u8 {
    unsafe { required_slot!(lbm_dec_char)(value.0) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_i(value: i32) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_enc_i)(value)) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_char(value: u8) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_enc_char)(value)) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_u32(value: u32) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_enc_u32)(value)) }
}

/// # Safety
///
/// `value` must be a LispBM string/byte-array value whose firmware-owned
/// storage remains valid for the duration of the call.
pub unsafe fn lbm_dec_str(value: LbmValue) -> *mut c_char {
    unsafe { required_slot!(lbm_dec_str)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_is_number(value: LbmValue) -> bool {
    unsafe { required_slot!(lbm_is_number)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_is_char(value: LbmValue) -> bool {
    unsafe { required_slot!(lbm_is_char)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_is_symbol(value: LbmValue) -> bool {
    unsafe { required_slot!(lbm_is_symbol)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_is_cons(value: LbmValue) -> bool {
    unsafe { required_slot!(lbm_is_cons)(value.0) }
}

/// # Safety
///
/// `value` must be a LispBM value supplied by the firmware.
pub unsafe fn lbm_is_byte_array(value: LbmValue) -> bool {
    unsafe { required_slot!(lbm_is_byte_array)(value.0) }
}

/// Allocate a LispBM byte array through the firmware allocator.
///
/// # Safety
///
/// `value` must be valid for one writable [`LbmValue`], and the firmware
/// function table must remain valid for the duration of the call.
pub unsafe fn lbm_create_byte_array(value: *mut LbmValue, len: u32) -> bool {
    unsafe { required_slot!(lbm_create_byte_array)(value.cast(), len) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_cons(car: LbmValue, cdr: LbmValue) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_cons)(car.0, cdr.0)) }
}

/// # Safety
///
/// `value` must be a LispBM cons cell supplied by the firmware.
pub unsafe fn lbm_car(value: LbmValue) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_car)(value.0)) }
}

/// # Safety
///
/// `value` must be a LispBM cons cell supplied by the firmware.
pub unsafe fn lbm_cdr(value: LbmValue) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_cdr)(value.0)) }
}

/// # Safety
///
/// `value` must be a mutable LispBM list owned by the current evaluation.
pub unsafe fn lbm_list_destructive_reverse(value: LbmValue) -> LbmValue {
    unsafe { LbmValue(required_slot!(lbm_list_destructive_reverse)(value.0)) }
}

/// # Safety
///
/// `value` must be valid for one firmware-written LispBM value.
pub unsafe fn lbm_create_byte_array(value: *mut LbmValue, len: u32) -> bool {
    unsafe { slots::lbm_create_byte_array()(value, len) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym(symbol: u32) -> LbmValue {
    unsafe { slots::lbm_enc_sym()(symbol) }
}

/// # Safety
///
/// `value` must be a LispBM symbol supplied by the firmware.
pub unsafe fn lbm_dec_sym(value: LbmValue) -> u32 {
    unsafe { slots::lbm_dec_sym()(value) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_send_message(context: u32, message: LbmValue) -> c_int {
    unsafe { slots::lbm_send_message()(context, message) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_get_current_cid() -> u32 {
    unsafe { slots::lbm_get_current_cid()() }
}

/// # Safety
///
/// The current callback must be running in a blockable LispBM extension context.
pub unsafe fn lbm_block_ctx_from_extension() {
    unsafe { slots::lbm_block_ctx_from_extension()() }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must remain valid.
pub unsafe fn lbm_unblock_ctx_unboxed(context: u32, value: LbmValue) -> Option<bool> {
    optional_bool_call(
        unsafe { slots::lbm_unblock_ctx_unboxed() },
        |unblock| unsafe { unblock(context, value) },
    )
}

/// Start an optional firmware 6.05 flat-value buffer.
///
/// # Safety
///
/// `value` must point to writable storage for one [`LbmFlatValue`].
pub unsafe fn lbm_start_flatten(value: *mut LbmFlatValue, buffer_size: usize) -> Option<bool> {
    optional_bool_call(unsafe { slots::lbm_start_flatten() }, |start| unsafe {
        start(value, buffer_size)
    })
}

/// Finish an optional firmware 6.05 flat-value buffer.
///
/// # Safety
///
/// `value` must be a buffer previously initialized by `lbm_start_flatten`.
pub unsafe fn lbm_finish_flatten(value: *mut LbmFlatValue) -> Option<bool> {
    optional_bool_call(unsafe { slots::lbm_finish_flatten() }, |finish| unsafe {
        finish(value)
    })
}

macro_rules! flat_value_bool_call {
    ($name:ident, $slot:ident, $($arg:ident : $ty:ty),* $(,)?) => {
        /// Call an optional firmware 6.05 flat-value constructor.
        ///
        /// # Safety
        ///
        /// `value` must be a live buffer initialized by `lbm_start_flatten` and
        /// each argument must remain valid for the duration of the call.
        pub unsafe fn $name(value: *mut LbmFlatValue, $($arg: $ty),*) -> Option<bool> {
            optional_bool_call(unsafe { slots::$slot() }, |build| unsafe {
                build(value, $($arg),*)
            })
        }
    };
}

flat_value_bool_call!(f_i64, f_i64, number: i64);
flat_value_bool_call!(f_u64, f_u64, number: u64);
flat_value_bool_call!(f_lbm_array, f_lbm_array, count: u32, data: *mut u8);
flat_value_bool_call!(f_sym, f_sym, symbol: u32);
flat_value_bool_call!(f_b, f_b, byte: u8);
flat_value_bool_call!(f_i32, f_i32, number: i32);
flat_value_bool_call!(f_u32, f_u32, number: u32);
flat_value_bool_call!(f_float, f_float, number: f32);

/// Append a cons marker to an optional firmware 6.05 flat value.
///
/// # Safety
///
/// `value` must be a live buffer initialized by `lbm_start_flatten`.
pub unsafe fn f_cons(value: *mut LbmFlatValue) -> Option<bool> {
    optional_bool_call(unsafe { slots::f_cons() }, |build| unsafe { build(value) })
}

/// Unblock a context with an optional firmware 6.05 flat value.
///
/// # Safety
///
/// `value` must be a finished buffer initialized by `lbm_start_flatten`.
pub unsafe fn lbm_unblock_ctx(context: u32, value: *mut LbmFlatValue) -> Option<bool> {
    optional_bool_call(unsafe { slots::lbm_unblock_ctx() }, |unblock| unsafe {
        unblock(context, value)
    })
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym_nil() -> LbmValue {
    unsafe { LbmValue(slots::lbm_enc_sym_nil() as u32) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym_true() -> LbmValue {
    unsafe { LbmValue(slots::lbm_enc_sym_true() as u32) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn lbm_enc_sym_eerror() -> LbmValue {
    unsafe { LbmValue(slots::lbm_enc_sym_eerror() as u32) }
}

/// Read one native-endian word from the package custom-EEPROM range.
///
/// # Safety
///
/// `word` must be valid for one `u32` write and the firmware function table
/// must remain valid for the duration of the call.
pub unsafe fn read_eeprom_word(word: *mut u32, address: c_int) -> bool {
    unsafe { required_slot!(read_eeprom_var)(word.cast(), address) }
}

/// Write one native-endian word to the package custom-EEPROM range.
///
/// # Safety
///
/// `word` must be valid for one `u32` read and the firmware function table
/// must remain valid for the duration of the call.
pub unsafe fn store_eeprom_word(word: *mut u32, address: c_int) -> bool {
    unsafe { required_slot!(store_eeprom_var)(word.cast(), address) }
}

/// Read a byte range from firmware NVM when the optional slot is present.
///
/// The outer `Option` reports whether the firmware exposes NVM; the inner
/// `bool` is the firmware's read result.
///
/// # Safety
///
/// `buffer` must be valid for `len` writable bytes, and the firmware function
/// table must remain valid for the duration of the call.
pub unsafe fn read_nvm(buffer: *mut u8, len: c_uint, address: c_uint) -> Option<bool> {
    optional_bool_call(unsafe { slots::read_nvm() }, |read| unsafe {
        read(buffer, len, address)
    })
}

/// Write a byte range to firmware NVM when the optional slot is present.
///
/// The outer `Option` reports whether the firmware exposes NVM; the inner
/// `bool` is the firmware's write result.
///
/// # Safety
///
/// `buffer` must be valid for `len` readable bytes, and the firmware function
/// table must remain valid for the duration of the call.
pub unsafe fn write_nvm(buffer: *mut u8, len: c_uint, address: c_uint) -> Option<bool> {
    optional_bool_call(unsafe { slots::write_nvm() }, |write| unsafe {
        write(buffer, len, address)
    })
}

/// Wipe firmware NVM when the optional slot is present.
///
/// The outer `Option` reports whether the firmware exposes NVM; the inner
/// `bool` is the firmware's wipe result.
///
/// # Safety
///
/// The firmware function table must remain valid for the duration of the call.
pub unsafe fn wipe_nvm() -> Option<bool> {
    optional_bool_call(unsafe { slots::wipe_nvm() }, |wipe| unsafe { wipe() })
}

fn optional_bool_call<F>(loader: Option<F>, invoke: impl FnOnce(F) -> bool) -> Option<bool> {
    loader.map(invoke)
}

/// Register the firmware app-data callback using the float-out-boy/C ABI.
///
/// # Safety
///
/// `handler` must remain valid until replaced or cleared by a later firmware call.
pub unsafe fn vesc_set_app_data_handler(handler: AppDataHandler) -> bool {
    unsafe { vesc_set_app_data_handler_slot(Some(handler)) }
}

unsafe fn vesc_set_app_data_handler_slot(handler: Option<AppDataHandler>) -> bool {
    unsafe { required_slot!(set_app_data_handler)(handler) }
}

/// Clear the firmware app-data callback.
///
/// # Safety
///
/// Must only be called when the firmware `VESC_IF` table is valid, same as
/// [`vesc_set_app_data_handler`].
pub unsafe fn vesc_clear_app_data_handler() {
    let _ = unsafe { vesc_set_app_data_handler_slot(None) };
}

/// Register the firmware IMU read callback.
///
/// Float Out Boy registers `imu_ref_callback` at `src/main.c:2455`; that callback
/// updates the balance filter at `src/main.c:760-764`. The VESC slot is
/// declared in `lispBM/c_libs/vesc_c_if.h:586`.
///
/// # Safety
///
/// `handler` must remain valid until replaced or cleared by a later firmware call.
pub unsafe fn vesc_set_imu_read_callback(handler: ImuReadCallback) {
    unsafe { vesc_set_imu_read_callback_slot(Some(handler)) }
}

unsafe fn vesc_set_imu_read_callback_slot(handler: Option<ImuReadCallback>) {
    unsafe { required_slot!(imu_set_read_callback)(handler) }
}

/// Clear the firmware IMU read callback.
///
/// Float Out Boy clears package callbacks during stop at `src/main.c:2401-2403`;
/// the VESC callback slot is declared in `lispBM/c_libs/vesc_c_if.h:586`.
///
/// # Safety
///
/// Must only be called when the firmware `VESC_IF` table is valid, same as
/// [`vesc_set_imu_read_callback`].
pub unsafe fn vesc_clear_imu_read_callback() {
    unsafe { vesc_set_imu_read_callback_slot(None) }
}

/// Read firmware IMU quaternions.
///
/// Float Out Boy initializes its balance filter from firmware quaternions at
/// `src/balance_filter.c:53-61`; the VESC slot is declared in
/// `lispBM/c_libs/vesc_c_if.h:521`.
///
/// # Safety
///
/// `quaternions` must point to four writable `f32` values.
pub unsafe fn vesc_imu_get_quaternions(quaternions: *mut f32) {
    unsafe { required_slot!(imu_get_quaternions)(quaternions) }
}

/// Register firmware custom-config callbacks using the Float Out Boy/VESC ABI.
///
/// Float Out Boy `v1.2.1` registers `get_cfg`, `set_cfg`, and `get_cfg_xml` through
/// this slot in `src/main.c:2456`. The VESC function-table slot is declared in
/// `vesc_pkg_lib/vesc_c_if.h:549-552`.
///
/// # Safety
///
/// The callbacks must remain valid until package stop clears them or the
/// firmware replaces them.
pub unsafe fn conf_custom_add_config(
    get_cfg: CustomConfigGet,
    set_cfg: CustomConfigSet,
    get_cfg_xml: CustomConfigXml,
) {
    unsafe {
        required_slot!(conf_custom_add_config)(Some(get_cfg), Some(set_cfg), Some(get_cfg_xml))
    }
}

/// Clear firmware custom-config callbacks.
///
/// Float Out Boy `v1.2.1` calls this during stop in `src/main.c:2403`. The VESC
/// function-table slot is declared in `vesc_pkg_lib/vesc_c_if.h:553`.
///
/// # Safety
///
/// Must only be called while the firmware `VESC_IF` table is valid.
pub unsafe fn conf_custom_clear_configs() {
    unsafe { required_slot!(conf_custom_clear_configs)() }
}

/// Allocate and initialize a firmware mutex.
///
/// Returns null when the slot is unavailable. A non-null mutex belongs to the
/// firmware reserve heap and must eventually be released with [`vesc_free`].
///
/// # Safety
///
/// The firmware `VESC_IF` table must be valid.
pub unsafe fn vesc_mutex_create() -> LibMutex {
    unsafe { slots::mutex_create() }.map_or(core::ptr::null_mut(), |create| unsafe { create() })
}

/// Lock a firmware mutex, blocking the current firmware thread.
///
/// # Safety
///
/// `mutex` must be a live handle returned by [`vesc_mutex_create`]. The mutex
/// is non-recursive and must not already be owned by the current thread.
pub unsafe fn vesc_mutex_lock(mutex: LibMutex) {
    unsafe { required_slot!(mutex_lock)(mutex) };
}

/// Unlock a firmware mutex owned by the current firmware thread.
///
/// # Safety
///
/// `mutex` must be a live handle returned by [`vesc_mutex_create`] and locked
/// by the current thread.
pub unsafe fn vesc_mutex_unlock(mutex: LibMutex) {
    unsafe { required_slot!(mutex_unlock)(mutex) };
}

/// Allocate and initialize a firmware semaphore, returning null when the slot
/// is unavailable.
///
/// # Safety
///
/// The firmware function table must be valid; the returned handle must be
/// released with [`vesc_free`].
pub unsafe fn vesc_sem_create() -> *mut c_void {
    unsafe { slots::sem_create() }.map_or(core::ptr::null_mut(), |create| unsafe { create() })
}

/// Block until a firmware semaphore is signaled.
///
/// # Safety
///
/// `semaphore` must be a live handle returned by [`vesc_sem_create`].
pub unsafe fn vesc_sem_wait(semaphore: *mut c_void) {
    unsafe { required_slot!(sem_wait)(semaphore) };
}

/// Signal a firmware semaphore.
///
/// # Safety
///
/// `semaphore` must be a live handle returned by [`vesc_sem_create`].
pub unsafe fn vesc_sem_signal(semaphore: *mut c_void) {
    unsafe { required_slot!(sem_signal)(semaphore) };
}

/// Wait for a firmware semaphore for at most `ticks` system ticks.
///
/// # Safety
///
/// `semaphore` must be a live handle returned by [`vesc_sem_create`].
pub unsafe fn vesc_sem_wait_to(semaphore: *mut c_void, ticks: u32) -> bool {
    unsafe { required_slot!(sem_wait_to)(semaphore, ticks) }
}

/// Reset a firmware semaphore to its unsignaled state.
///
/// # Safety
///
/// `semaphore` must be a live handle returned by [`vesc_sem_create`].
pub unsafe fn vesc_sem_reset(semaphore: *mut c_void) {
    unsafe { required_slot!(sem_reset)(semaphore) };
}

/// Allocate memory from the firmware LispBM reserve heap.
///
/// # Safety
///
/// The caller must check for null. A non-null returned pointer belongs to the
/// firmware/LispBM reserve heap and must be freed with [`vesc_free`] when no
/// longer used.
pub unsafe fn vesc_malloc(bytes: usize) -> *mut c_void {
    unsafe { required_slot!(malloc)(bytes) }
}

/// Free memory previously allocated by [`vesc_malloc`].
///
/// # Safety
///
/// `ptr` must be null or a pointer returned by the firmware allocator, and it
/// must not already have been freed.
pub unsafe fn vesc_free(ptr: *mut c_void) {
    unsafe {
        required_slot!(free)(ptr);
    }
}

/// Spawn a firmware package thread.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:382` and starts its main/auxiliary threads at
/// `src/main.c:2438-2448`.
///
/// # Safety
///
/// `entry` and `name` must remain valid for the firmware call, and `arg` must
/// point to state that lives until the spawned thread terminates.
pub unsafe fn vesc_spawn(
    entry: unsafe extern "C" fn(*mut c_void),
    stack_bytes: usize,
    name: *const c_char,
    arg: *mut c_void,
) -> LibThread {
    unsafe { required_slot!(spawn)(Some(entry), stack_bytes, name, arg) }
}

/// Sleep the current firmware package thread for a number of microseconds.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:376` and sleeps the main loop at
/// `src/main.c:1080`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_sleep_us(micros: u32) {
    unsafe { required_slot!(sleep_us)(micros) };
}

/// Set the current firmware package thread priority when the slot is present.
///
/// Float Out Boy v1.2.1 checks optional `thread_set_priority` before lowering
/// `aux_thd` priority at `src/main.c:1133-1135`; the VESC ABI slot is declared
/// at `vesc_pkg_lib/vesc_c_if.h:670`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_thread_set_priority(priority: c_int) -> bool {
    unsafe { slots::thread_set_priority() }
        .map(|func| unsafe { func(priority) })
        .is_some()
}
/// Ask a firmware package thread to terminate.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:383` and requests thread termination during stop
/// at `src/main.c:2404-2408`.
///
/// # Safety
///
/// `thread` must be null or a thread handle returned by [`vesc_spawn`].
pub unsafe fn vesc_request_terminate(thread: LibThread) {
    unsafe {
        required_slot!(request_terminate)(thread);
    }
}

/// Return whether the current firmware package thread should terminate.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:384` and loops on it in `float_out_boy_thd` and
/// `aux_thd` at `src/main.c:771` and `src/main.c:1138`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_should_terminate() -> bool {
    unsafe { required_slot!(should_terminate)() }
}

/// Return the firmware-owned mutable `lib_info.arg` slot for a loaded native library.
///
/// # Safety
///
/// `prog_addr` must be the native library base address passed by the VESC loader.
pub unsafe fn vesc_get_arg(prog_addr: u32) -> *mut *mut c_void {
    unsafe { required_slot!(get_arg)(prog_addr) }
}

/// Transmit one bounded standard CAN frame when the optional slot is present.
///
/// # Safety
///
/// `data` must point to `len` readable bytes for the duration of the call.
pub unsafe fn can_transmit_sid(id: u32, data: *const u8, len: u8) -> Option<()> {
    unsafe { slots::can_transmit_sid() }.map(|transmit| unsafe { transmit(id, data, len) })
}

/// Install or clear the standard-ID CAN receiver callback.
pub unsafe fn can_set_sid_callback(callback: Option<CanReceiverCallback>) -> Option<()> {
    unsafe { slots::can_set_sid_cb() }.map(|set| unsafe { set(callback) })
}

/// Install or clear the extended-ID CAN receiver callback.
pub unsafe fn can_set_eid_callback(callback: Option<CanReceiverCallback>) -> Option<()> {
    unsafe { slots::can_set_eid_cb() }.map(|set| unsafe { set(callback) })
}

/// Transmit one bounded extended CAN frame when the optional slot is present.
///
/// # Safety
///
/// `data` must point to `len` readable bytes for the duration of the call.
pub unsafe fn can_transmit_eid(id: u32, data: *const u8, len: u8) -> Option<()> {
    unsafe { slots::can_transmit_eid() }.map(|transmit| unsafe { transmit(id, data, len) })
}

/// Send a remote motor duty command when the optional slot is present.
pub unsafe fn can_set_duty(controller: u8, duty: f32) -> Option<()> {
    unsafe { slots::can_set_duty() }.map(|set| unsafe { set(controller, duty) })
}

/// Ping one remote controller and copy its reported hardware type.
pub unsafe fn can_ping(controller: u8) -> Option<(bool, crate::HardwareType)> {
    let ping = unsafe { slots::can_ping() }?;
    let mut hardware = 0;
    let ok = unsafe { ping(controller, &mut hardware) };
    Some((ok, crate::HardwareType(hardware)))
}

/// Send a remote motor current command when the optional slot is present.
pub unsafe fn can_set_current(controller: u8, current: f32) -> Option<()> {
    unsafe { slots::can_set_current() }.map(|set| unsafe { set(controller, current) })
}

/// Send a remote motor relative-current command when the optional slot is present.
pub unsafe fn can_set_current_rel(controller: u8, current: f32) -> Option<()> {
    unsafe { slots::can_set_current_rel() }.map(|set| unsafe { set(controller, current) })
}

/// Send a remote motor relative-current command with an off-delay when the optional slot is present.
pub unsafe fn can_set_current_rel_off_delay(
    controller: u8,
    current: f32,
    delay_seconds: f32,
) -> Option<()> {
    unsafe { slots::can_set_current_rel_off_delay() }
        .map(|set| unsafe { set(controller, current, delay_seconds) })
}

/// Send a remote motor brake-current command when the optional slot is present.
pub unsafe fn can_set_current_brake(controller: u8, current: f32) -> Option<()> {
    unsafe { slots::can_set_current_brake() }.map(|set| unsafe { set(controller, current) })
}

/// Send a remote motor relative brake-current command when the optional slot is present.
pub unsafe fn can_set_current_brake_rel(controller: u8, current: f32) -> Option<()> {
    unsafe { slots::can_set_current_brake_rel() }.map(|set| unsafe { set(controller, current) })
}

/// Send a remote motor current command with an off-delay when the optional slot is present.
pub unsafe fn can_set_current_off_delay(
    controller: u8,
    current: f32,
    delay_seconds: f32,
) -> Option<()> {
    unsafe { slots::can_set_current_off_delay() }
        .map(|set| unsafe { set(controller, current, delay_seconds) })
}

/// Send a remote motor electrical-speed command when the optional slot is present.
pub unsafe fn can_set_rpm(controller: u8, rpm: f32) -> Option<()> {
    unsafe { slots::can_set_rpm() }.map(|set| unsafe { set(controller, rpm) })
}

/// Send a remote motor position command when the optional slot is present.
pub unsafe fn can_set_pos(controller: u8, position: f32) -> Option<()> {
    unsafe { slots::can_set_pos() }.map(|set| unsafe { set(controller, position) })
}

/// Copy a firmware-owned record without allowing a null pointer to cross the
/// raw boundary. The firmware records are `Copy` snapshots, so the returned
/// value is independent of the firmware's storage.
unsafe fn copy_firmware_record<T: Copy>(record: *const T) -> Option<T> {
    unsafe { record.as_ref().copied() }
}

macro_rules! copy_optional_status {
    ($wrapper:ident, $slot:ident, $status:ty) => {
        /// Copy one firmware-owned CAN status record, returning `None` when the
        /// slot or indexed record is unavailable.
        ///
        /// # Safety
        ///
        /// The VESC function table and the record returned by firmware must be
        /// valid for the duration of this call.
        pub unsafe fn $wrapper(index: c_int) -> Option<$status> {
            let loader = unsafe { slots::$slot()? };
            unsafe { copy_firmware_record(loader(index)) }
        }
    };
}

copy_optional_status!(can_status_msg_index, can_get_status_msg_index, CanStatusMsg);
copy_optional_status!(can_status_msg_id, can_get_status_msg_id, CanStatusMsg);
copy_optional_status!(
    can_status_msg_2_index,
    can_get_status_msg_2_index,
    CanStatusMsg2
);
copy_optional_status!(can_status_msg_2_id, can_get_status_msg_2_id, CanStatusMsg2);
copy_optional_status!(
    can_status_msg_3_index,
    can_get_status_msg_3_index,
    CanStatusMsg3
);
copy_optional_status!(can_status_msg_3_id, can_get_status_msg_3_id, CanStatusMsg3);
copy_optional_status!(
    can_status_msg_4_index,
    can_get_status_msg_4_index,
    CanStatusMsg4
);
copy_optional_status!(can_status_msg_4_id, can_get_status_msg_4_id, CanStatusMsg4);
copy_optional_status!(
    can_status_msg_5_index,
    can_get_status_msg_5_index,
    CanStatusMsg5
);
copy_optional_status!(can_status_msg_5_id, can_get_status_msg_5_id, CanStatusMsg5);
copy_optional_status!(
    can_status_msg_6_index,
    can_get_status_msg_6_index,
    CanStatusMsg6
);
copy_optional_status!(can_status_msg_6_id, can_get_status_msg_6_id, CanStatusMsg6);

/// Copy the firmware-owned GNSS record, returning `None` when the slot or
/// current GNSS record is unavailable.
///
/// # Safety
///
/// The VESC function table and its GNSS record pointer must be valid for the
/// duration of this call.
pub unsafe fn gnss_snapshot() -> Option<GnssData> {
    let loader = unsafe { slots::mc_gnss()? };
    unsafe { copy_firmware_record(loader()) }
}

/// Copy the current remote-control state when firmware exposes the slot.
///
/// # Safety
///
/// The VESC function table must be valid for the duration of this call.
pub unsafe fn remote_state() -> Option<RemoteState> {
    unsafe { slots::get_remote_state() }.map(|func| unsafe { func() })
}

/// Read the decoded PPM input when the optional slot is present.
pub unsafe fn get_ppm() -> Option<f32> {
    unsafe { slots::get_ppm() }.map(|read| unsafe { read() })
}

/// Read the age of the latest decoded PPM input when supported.
pub unsafe fn get_ppm_age() -> Option<f32> {
    unsafe { slots::get_ppm_age() }.map(|read| unsafe { read() })
}

/// Read whether firmware currently asks applications to disable output.
pub unsafe fn app_is_output_disabled() -> Option<bool> {
    unsafe { slots::app_is_output_disabled() }.map(|read| unsafe { read() })
}

/// Persist firmware backup data when the optional capability is present.
pub unsafe fn store_backup_data() -> Option<bool> {
    unsafe { slots::store_backup_data() }.map(|store| unsafe { store() })
}

/// Return the active motor fault code, or zero for no fault.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_fault() -> c_int {
    unsafe { required_slot!(mc_get_fault)() as c_int }
}

/// Return the latest decoded PPM input.
///
/// # Safety
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn get_ppm() -> Option<f32> {
    unsafe { slots::get_ppm() }.map(|func| unsafe { func() })
}

/// Return the age of the latest decoded PPM input in seconds.
///
/// # Safety
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn get_ppm_age() -> Option<f32> {
    unsafe { slots::get_ppm_age() }.map(|func| unsafe { func() })
}

/// Return the current motor electrical RPM.
///
/// Float Out Boy v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:108`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:450`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_rpm() -> f32 {
    unsafe { required_slot!(mc_get_rpm)() }
}

/// Return firmware-calculated vehicle speed in meters per second.
///
/// Float Out Boy v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:118`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:470`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_speed() -> f32 {
    unsafe { required_slot!(mc_get_speed)() }
}

/// Return filtered total motor current.
///
/// Float Out Boy v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:120`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:456`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_tot_current_filtered() -> f32 {
    unsafe { required_slot!(mc_get_tot_current_filtered)() }
}

/// Return the instantaneous total motor current.
pub unsafe fn mc_get_tot_current() -> f32 {
    unsafe { slots::mc_get_tot_current()() }
}

/// Return direction-adjusted filtered motor current.
///
/// Float Out Boy v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:121`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:458`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_tot_current_directional_filtered() -> f32 {
    unsafe { required_slot!(mc_get_tot_current_directional_filtered)() }
}

/// Return the instantaneous total motor current with motor direction applied.
pub unsafe fn mc_get_tot_current_directional() -> f32 {
    unsafe { slots::mc_get_tot_current_directional()() }
}

/// Return filtered input/battery current.
///
/// Float Out Boy v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:140`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:460`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_tot_current_in_filtered() -> f32 {
    unsafe { required_slot!(mc_get_tot_current_in_filtered)() }
}

/// Return the instantaneous battery/input current.
pub unsafe fn mc_get_tot_current_in() -> f32 {
    unsafe { slots::mc_get_tot_current_in()() }
}

/// Read a firmware motor configuration float by `CFG_PARAM_*` id.
///
/// Float Out Boy v1.2.1 reads `CFG_PARAM_l_current_max` and
/// `CFG_PARAM_l_current_min` in `src/motor_data.c:90-91`; the VESC ABI slot is
/// declared at `vesc_pkg_lib/vesc_c_if.h:588`.
///
/// # Safety
///
/// The firmware VESC function table must be valid and `param` must be a valid
/// firmware configuration parameter id for a float-valued setting.
pub unsafe fn get_cfg_float(param: c_int) -> f32 {
    unsafe { required_slot!(get_cfg_float)(param as c_uint) }
}

/// Read a firmware motor configuration integer by `CFG_PARAM_*` id.
///
/// Float Out Boy v1.2.1 reads `CFG_PARAM_si_battery_cells` in
/// `src/motor_data.c:76`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:590`.
///
/// # Safety
///
/// The firmware VESC function table must be valid and `param` must be a valid
/// firmware configuration parameter id for an integer-valued setting.
pub unsafe fn get_cfg_int(param: c_int) -> c_int {
    unsafe { required_slot!(get_cfg_int)(param as c_uint) }
}

/// Reset the firmware motor-command safety timeout.
///
/// Float Out Boy v1.2.1 calls this before every motor-control apply branch in
/// `third_party/float-out-boy/src/motor_control.c:92-93`; the VESC ABI slot is declared at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:538`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn timeout_reset() {
    unsafe { required_slot!(timeout_reset)() }
}

/// Return whether the firmware command timeout is active.
pub unsafe fn timeout_has_timeout() -> bool {
    unsafe { slots::timeout_has_timeout()() }
}

/// Return seconds since the firmware command timeout was last refreshed.
pub unsafe fn timeout_secs_since_update() -> f32 {
    unsafe { slots::timeout_secs_since_update()() }
}

/// Keep current control enabled after a current command.
///
/// Float Out Boy v1.2.1 calls this before `mc_set_current` in
/// `third_party/float-out-boy/src/motor_control.c:96-99`; the VESC ABI slot is declared at
/// `third_party/vesc_pkg_lib/vesc_c_if.h:476`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_current_off_delay(seconds: f32) {
    unsafe { required_slot!(mc_set_current_off_delay)(seconds) }
}

/// Set the motor current command in amps.
///
/// Float Out Boy v1.2.1 sends requested current in
/// `third_party/float-out-boy/src/motor_control.c:96-99`; the VESC ABI slot is
/// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:440`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_current(amps: f32) {
    unsafe { required_slot!(mc_set_current)(amps) }
}

/// Set the motor duty-cycle command.
///
/// Float Out Boy v1.2.1 applies parking brake duty zero in
/// `third_party/float-out-boy/src/motor_control.c:112-114`; the VESC ABI slot is
/// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:436`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_duty(duty_cycle: f32) {
    unsafe { required_slot!(mc_set_duty)(duty_cycle) }
}

/// Set the motor brake current command in amps.
///
/// Float Out Boy v1.2.1 applies idle brake current in
/// `third_party/float-out-boy/src/motor_control.c:115-117`; the VESC ABI slot is
/// declared at `third_party/vesc_pkg_lib/vesc_c_if.h:441`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_set_brake_current(amps: f32) {
    unsafe { required_slot!(mc_set_brake_current)(amps) }
}

/// Set the motor handbrake current.
pub unsafe fn mc_set_handbrake(amps: f32) {
    unsafe { slots::mc_set_handbrake()(amps) }
}

/// Set the motor relative handbrake command.
pub unsafe fn mc_set_handbrake_rel(ratio: f32) {
    unsafe { slots::mc_set_handbrake_rel()(ratio) }
}

/// Read the relative motor tachometer, optionally resetting it.
pub unsafe fn mc_get_tachometer_value(reset: bool) -> c_int {
    unsafe { slots::mc_get_tachometer_value()(reset) }
}

/// Read the absolute motor tachometer, optionally resetting it.
pub unsafe fn mc_get_tachometer_abs_value(reset: bool) -> c_int {
    unsafe { slots::mc_get_tachometer_abs_value()(reset) }
}

/// Read the average motor power statistic.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid and contain
/// the motor-statistics slot.
pub unsafe fn mc_stat_power_avg() -> f32 {
    unsafe { slots::mc_stat_power_avg()() }
}

/// Read the peak motor power statistic.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid and contain
/// the motor-statistics slot.
pub unsafe fn mc_stat_power_max() -> f32 {
    unsafe { slots::mc_stat_power_max()() }
}

/// Reset motor power and related statistics.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid and contain
/// the motor-statistics slot.
pub unsafe fn mc_stat_reset() {
    unsafe { slots::mc_stat_reset()() }
}

/// Read the motor control sampling frequency.
pub unsafe fn mc_get_sampling_frequency_now() -> f32 {
    unsafe { slots::mc_get_sampling_frequency_now()() }
}

/// Release motor control ownership.
pub unsafe fn mc_release_motor() {
    unsafe { slots::mc_release_motor()() }
}

/// Wait until motor control ownership has been released.
pub unsafe fn mc_wait_for_motor_release(timeout: f32) -> bool {
    unsafe { slots::mc_wait_for_motor_release()(timeout) }
}

/// Return the current duty cycle.
///
/// Float Out Boy v1.2.1 reads this in `motor_data_update` at
/// `src/motor_data.c:124`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:448`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_duty_cycle_now() -> f32 {
    unsafe { required_slot!(mc_get_duty_cycle_now)() }
}

/// Return the firmware-owned name for a motor fault code when the slot exists.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid. A non-null
/// returned pointer is firmware-owned and must point to a NUL-terminated string.
pub unsafe fn mc_fault_to_string(code: c_uint) -> Option<*const c_char> {
    let pointer = unsafe { required_slot!(mc_fault_to_string)(code) };
    (!pointer.is_null()).then_some(pointer)
}

/// Return FOC d-axis Id current when the firmware slot is present.
///
/// Float Out Boy v1.2.1 reads optional `foc_get_id` while encoding compact all-data
/// at `src/main.c:1364-1368`; the VESC ABI slot is declared at
/// `vesc_pkg_lib/vesc_c_if.h:616`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn foc_get_id() -> Option<f32> {
    unsafe { slots::foc_get_id() }.map(|func| unsafe { func() })
}

/// Play one FOC tone when the motor firmware exposes audio support.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn foc_play_tone(channel: c_int, frequency: f32, voltage: f32) -> Option<bool> {
    unsafe { slots::foc_play_tone() }.map(|func| unsafe { func(channel, frequency, voltage) })
}
/// Return the filtered input/battery voltage.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_input_voltage_filtered() -> f32 {
    unsafe { required_slot!(mc_get_input_voltage_filtered)() }
}

/// Return the discharged amp-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_amp_hours(reset: bool) -> f32 {
    unsafe { required_slot!(mc_get_amp_hours)(reset) }
}

/// Return the charged amp-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_amp_hours_charged(reset: bool) -> f32 {
    unsafe { required_slot!(mc_get_amp_hours_charged)(reset) }
}

/// Return the discharged watt-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_watt_hours(reset: bool) -> f32 {
    unsafe { required_slot!(mc_get_watt_hours)(reset) }
}

/// Return the charged watt-hours counter.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_watt_hours_charged(reset: bool) -> f32 {
    unsafe { required_slot!(mc_get_watt_hours_charged)(reset) }
}

/// Return the estimated battery level as a ratio.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid. If
/// `wh_left` is not null, it must be valid for firmware to write one `f32`.
pub unsafe fn mc_get_battery_level(wh_left: *mut f32) -> f32 {
    unsafe { required_slot!(mc_get_battery_level)(wh_left) }
}

/// Return the absolute motor distance in meters.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_distance_abs() -> f32 {
    unsafe { required_slot!(mc_get_distance_abs)() }
}

/// Return the odometer distance in meters.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_get_odometer() -> u64 {
    unsafe { required_slot!(mc_get_odometer)() }
}

/// Return the filtered MOSFET/FET temperature in degrees Celsius.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_temp_fet_filtered() -> f32 {
    unsafe { required_slot!(mc_temp_fet_filtered)() }
}

/// Return the filtered motor temperature in degrees Celsius.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn mc_temp_motor_filtered() -> f32 {
    unsafe { required_slot!(mc_temp_motor_filtered)() }
}

/// Return whether firmware IMU startup has completed.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:510` and gates startup readiness at
/// `src/main.c:834-838`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_startup_done() -> bool {
    unsafe { required_slot!(imu_startup_done)() }
}

/// Return firmware IMU roll in radians.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:511` and reads it in `src/imu.c:35-40`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_roll() -> f32 {
    unsafe { required_slot!(imu_get_roll)() }
}

/// Return firmware IMU pitch in radians.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:512` and reads it in `src/imu.c:37-38`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_pitch() -> f32 {
    unsafe { required_slot!(imu_get_pitch)() }
}

/// Return firmware IMU yaw in radians.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:513` and reads it in `src/imu.c:39-40`.
///
/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_yaw() -> f32 {
    unsafe { required_slot!(imu_get_yaw)() }
}

/// Write firmware IMU gyro axes in degrees/sec into `xyz`.
///
/// Float Out Boy v1.2.1 mirrors this VESC ABI slot from
/// `vesc_pkg_lib/vesc_c_if.h:516` and reads it in `src/imu.c:45-53`.
///
/// # Safety
///
/// `xyz` must point to three writable `f32` values, and the VESC function
/// table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn imu_get_gyro(xyz: *mut f32) {
    unsafe { required_slot!(imu_get_gyro)(xyz) }
}

/// # Safety
///
/// `data` must point to at least `len` bytes that remain valid for the
/// duration of the firmware call.
pub unsafe fn vesc_send_app_data(data: *const u8, len: u32) {
    unsafe {
        required_slot!(send_app_data)(data as *mut c_uchar, len);
    }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn vesc_system_time_ticks() -> u32 {
    unsafe {
        if let Some(system_time_ticks) = slots::system_time_ticks() {
            system_time_ticks()
        } else {
            // Legacy VESC tables expose seconds only. The firmware system tick
            // is 100 microseconds (10 kHz), matching chVTGetSystemTimeX().
            (required_slot!(system_time)() * 10_000.0) as u32
        }
    }
}

/// Return firmware uptime in its native floating-point seconds domain.
///
/// # Safety
///
/// The VESC function table at [`VescIfAbi::BASE_ADDR`] must be valid.
pub unsafe fn vesc_system_time_seconds() -> f32 {
    unsafe { required_slot!(system_time)() }
}

/// Return the age of a firmware system timestamp in floating-point seconds.
///
/// # Safety
///
/// The VESC function table at [`VescIfAbi::BASE_ADDR`] must be valid.
pub unsafe fn vesc_timestamp_age_seconds(timestamp: u32) -> f32 {
    unsafe { required_slot!(ts_to_age_s)(timestamp) }
}

/// Return the current high-resolution firmware timer instant.
///
/// # Safety
///
/// The VESC function table at [`VescIfAbi::BASE_ADDR`] must be valid.
pub unsafe fn vesc_timer_time_now() -> u32 {
    unsafe { required_slot!(timer_time_now)() }
}

/// Return high-resolution elapsed time in floating-point seconds.
///
/// # Safety
///
/// The VESC function table at [`VescIfAbi::BASE_ADDR`] must be valid.
pub unsafe fn vesc_timer_seconds_elapsed_since(timestamp: u32) -> f32 {
    unsafe { required_slot!(timer_seconds_elapsed_since)(timestamp) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_set_mode(pin: crate::VescPin, mode: crate::VescPinMode) -> bool {
    unsafe { required_slot!(io_set_mode)(pin.0 as c_uint, mode.0 as c_uint) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_write(pin: crate::VescPin, level: i32) -> bool {
    unsafe { required_slot!(io_write)(pin.0 as c_uint, level) }
}

/// # Safety
///
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
pub unsafe fn io_read(pin: crate::VescPin) -> bool {
    unsafe { required_slot!(io_read)(pin.0 as c_uint) }
}

/// # Safety
///
/// The VESC slot is declared in `lispBM/c_libs/vesc_c_if.h:396`.
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
#[inline(always)]
pub unsafe fn io_read_analog(pin: crate::VescPin) -> f32 {
    unsafe { required_slot!(io_read_analog)(pin.0 as c_uint) }
}

/// # Safety
///
/// The VESC slot is declared in `lispBM/c_libs/vesc_c_if.h:396`.
/// The VESC function table at `VescIfAbi::BASE_ADDR` must be valid.
#[inline(always)]
pub unsafe fn io_read_analog_pair(first: crate::VescPin, second: crate::VescPin) -> (f32, f32) {
    let read = unsafe { required_slot!(io_read_analog) };
    (unsafe { read(first.0 as c_uint) }, unsafe {
        read(second.0 as c_uint)
    })
}

#[cfg(test)]
mod dispatch_tests;
