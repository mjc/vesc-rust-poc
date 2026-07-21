use core::ffi::{c_char, c_void};
use core::hint::spin_loop;
use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering,
};

use crate::{
    AmpHoursCharged, AmpHoursDischarged, BatteryLevel, DCurrent, DirectionalMotorCurrent,
    DutyCycle, ElectricalSpeed, FirmwareFaultCode, ImuAngularRate, ImuOrientation, ImuPitch,
    ImuRoll, ImuYaw, InputCurrent, InputVoltage, MosfetTemperature, MotorCurrentLimit,
    MotorTemperature, OdometerMeters, TotalMotorCurrent, TripDistance, VehicleSpeed,
    WattHoursCharged, WattHoursDischarged,
};
use vescpkg_rs_sys::LbmValue;

// C map: these host replacements model the motor slots declared at
// `third_party/vesc_pkg_lib/vesc_c_if.h:435-476`. Refloat reads them in
// `third_party/refloat/src/motor_data.c:75-125` and writes motor commands in
// `third_party/refloat/src/motor_control.c:92-117`.
// IMU replacements preserve `third_party/vesc/imu/imu.c:414-443`; the typed
// SDK values feed `third_party/refloat/src/balance_filter.c:54-59`.
// Thread replacements model `third_party/vesc_pkg_lib/vesc_c_if.h:387-403`
// as used by `third_party/refloat/src/main.c:1075-1080,2439-2445`.

static LOCKED: AtomicBool = AtomicBool::new(false);
static KEEP_ALIVE_COUNT: AtomicUsize = AtomicUsize::new(0);
static CURRENT_OFF_DELAY_COUNT: AtomicUsize = AtomicUsize::new(0);
static CURRENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static DUTY_COUNT: AtomicUsize = AtomicUsize::new(0);
static BRAKE_CURRENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static CURRENT_OFF_DELAY: AtomicU32 = AtomicU32::new(0);
static CURRENT: AtomicU32 = AtomicU32::new(0);
static DUTY: AtomicU32 = AtomicU32::new(0);
static BRAKE_CURRENT: AtomicU32 = AtomicU32::new(0);
static ELECTRICAL_SPEED: AtomicU32 = AtomicU32::new(0);
static VEHICLE_SPEED: AtomicU32 = AtomicU32::new(0);
static MOTOR_CURRENT: AtomicU32 = AtomicU32::new(0);
static DIRECTIONAL_MOTOR_CURRENT: AtomicU32 = AtomicU32::new(0);
static MOTOR_CURRENT_MAX: AtomicU32 = AtomicU32::new(0);
static MOTOR_CURRENT_MIN: AtomicU32 = AtomicU32::new(0);
static MOSFET_TEMPERATURE_LIMIT_START: AtomicU32 = AtomicU32::new(0);
static MOTOR_TEMPERATURE_LIMIT_START: AtomicU32 = AtomicU32::new(0);
static DUTY_CYCLE_LIMIT: AtomicU32 = AtomicU32::new(0);
static BATTERY_CELL_COUNT: AtomicI32 = AtomicI32::new(0);
static INPUT_CURRENT: AtomicU32 = AtomicU32::new(0);
static DUTY_CYCLE: AtomicU32 = AtomicU32::new(0);
static FOC_ID_CURRENT: AtomicU32 = AtomicU32::new(0);
static HAS_FOC_ID_CURRENT: AtomicBool = AtomicBool::new(false);
static DISTANCE_ABS: AtomicU32 = AtomicU32::new(0);
static MOSFET_TEMPERATURE: AtomicU32 = AtomicU32::new(0);
static MOTOR_TEMPERATURE: AtomicU32 = AtomicU32::new(0);
static ODOMETER: AtomicU64 = AtomicU64::new(0);
static AMP_HOURS_DISCHARGED: AtomicU32 = AtomicU32::new(0);
static AMP_HOURS_CHARGED: AtomicU32 = AtomicU32::new(0);
static WATT_HOURS_DISCHARGED: AtomicU32 = AtomicU32::new(0);
static WATT_HOURS_CHARGED: AtomicU32 = AtomicU32::new(0);
static BATTERY_LEVEL: AtomicU32 = AtomicU32::new(0);
static FIRMWARE_FAULT: AtomicI32 = AtomicI32::new(0);
static INPUT_VOLTAGE: AtomicU32 = AtomicU32::new(0);
static IMU_STARTUP_DONE: AtomicBool = AtomicBool::new(false);
static IMU_ROLL: AtomicU32 = AtomicU32::new(0);
static IMU_PITCH: AtomicU32 = AtomicU32::new(0);
static IMU_YAW: AtomicU32 = AtomicU32::new(0);
static IMU_GYRO: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];
static IMU_QUATERNION: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
static THREAD_SPAWN_COUNT: AtomicUsize = AtomicUsize::new(0);
static THREAD_SPAWN_STACKS: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
static THREAD_SPAWN_RESULTS: [AtomicUsize; 2] = [AtomicUsize::new(1), AtomicUsize::new(2)];
static THREAD_TERMINATE_COUNT: AtomicUsize = AtomicUsize::new(0);
static THREAD_TERMINATED: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
static THREAD_TERMINATE_CHECKS: AtomicUsize = AtomicUsize::new(0);
static THREAD_TERMINATE_AFTER: AtomicUsize = AtomicUsize::new(usize::MAX);
static THREAD_SLEEP_COUNT: AtomicUsize = AtomicUsize::new(0);
static THREAD_SLEEP_MICROS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static THREAD_PRIORITY_COUNT: AtomicUsize = AtomicUsize::new(0);
static THREAD_PRIORITIES: [AtomicI32; 2] = [const { AtomicI32::new(0) }; 2];
const EEPROM_WORDS: usize = 128;
static EEPROM: [AtomicU32; EEPROM_WORDS] = [const { AtomicU32::new(0) }; EEPROM_WORDS];
static EEPROM_PRESENT: [AtomicBool; EEPROM_WORDS] =
    [const { AtomicBool::new(false) }; EEPROM_WORDS];
static EEPROM_WRITE_FAILURE: AtomicI32 = AtomicI32::new(-1);
const NVM_BYTES: usize = 256;
static NVM: [AtomicU8; NVM_BYTES] = [const { AtomicU8::new(0) }; NVM_BYTES];
static NVM_FAILURE: AtomicBool = AtomicBool::new(false);
static CLOCK_TICKS: AtomicU32 = AtomicU32::new(0);
static TIMER_TICKS: AtomicU32 = AtomicU32::new(0);
static MUTEX_TOKEN: u8 = 0;
static MUTEX_LOCK_COUNT: AtomicUsize = AtomicUsize::new(0);
static MUTEX_UNLOCK_COUNT: AtomicUsize = AtomicUsize::new(0);
static MUTEX_FREE_COUNT: AtomicUsize = AtomicUsize::new(0);
static MUTEX_CREATE_FAILURE: AtomicBool = AtomicBool::new(false);
static SEMAPHORE_TOKEN: u8 = 0;
static SEMAPHORE_CREATE_FAILURE: AtomicBool = AtomicBool::new(false);
static SEMAPHORE_TIMEOUT_FAILURE: AtomicBool = AtomicBool::new(false);
static SEMAPHORE_WAIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static SEMAPHORE_TIMED_WAIT_TICKS: AtomicU32 = AtomicU32::new(u32::MAX);
static SEMAPHORE_SIGNAL_COUNT: AtomicUsize = AtomicUsize::new(0);
static SEMAPHORE_RESET_COUNT: AtomicUsize = AtomicUsize::new(0);
static SEMAPHORE_FREE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct MotorOutputState {
    pub keep_alive_count: usize,
    pub current_off_delay_count: usize,
    pub current_count: usize,
    pub duty_count: usize,
    pub brake_current_count: usize,
    pub current_off_delay: f32,
    pub current: f32,
    pub duty: f32,
    pub brake_current: f32,
}

pub(crate) struct FirmwareLockGuard;

impl Drop for FirmwareLockGuard {
    fn drop(&mut self) {
        LOCKED.store(false, Ordering::Release);
    }
}

pub(crate) fn lock_firmware() -> FirmwareLockGuard {
    while LOCKED
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        spin_loop();
    }
    KEEP_ALIVE_COUNT.store(0, Ordering::Relaxed);
    CURRENT_OFF_DELAY_COUNT.store(0, Ordering::Relaxed);
    CURRENT_COUNT.store(0, Ordering::Relaxed);
    DUTY_COUNT.store(0, Ordering::Relaxed);
    BRAKE_CURRENT_COUNT.store(0, Ordering::Relaxed);
    CURRENT_OFF_DELAY.store(0.0_f32.to_bits(), Ordering::Relaxed);
    CURRENT.store(0.0_f32.to_bits(), Ordering::Relaxed);
    DUTY.store(0.0_f32.to_bits(), Ordering::Relaxed);
    BRAKE_CURRENT.store(0.0_f32.to_bits(), Ordering::Relaxed);
    ELECTRICAL_SPEED.store(0.0_f32.to_bits(), Ordering::Relaxed);
    VEHICLE_SPEED.store(0.0_f32.to_bits(), Ordering::Relaxed);
    MOTOR_CURRENT.store(0.0_f32.to_bits(), Ordering::Relaxed);
    DIRECTIONAL_MOTOR_CURRENT.store(0.0_f32.to_bits(), Ordering::Relaxed);
    MOTOR_CURRENT_MAX.store(100.0_f32.to_bits(), Ordering::Relaxed);
    MOTOR_CURRENT_MIN.store((-100.0_f32).to_bits(), Ordering::Relaxed);
    MOSFET_TEMPERATURE_LIMIT_START.store(85.0_f32.to_bits(), Ordering::Relaxed);
    MOTOR_TEMPERATURE_LIMIT_START.store(85.0_f32.to_bits(), Ordering::Relaxed);
    DUTY_CYCLE_LIMIT.store(0.95_f32.to_bits(), Ordering::Relaxed);
    BATTERY_CELL_COUNT.store(0, Ordering::Relaxed);
    INPUT_CURRENT.store(0.0_f32.to_bits(), Ordering::Relaxed);
    DUTY_CYCLE.store(0.0_f32.to_bits(), Ordering::Relaxed);
    FOC_ID_CURRENT.store(0.0_f32.to_bits(), Ordering::Relaxed);
    HAS_FOC_ID_CURRENT.store(false, Ordering::Relaxed);
    DISTANCE_ABS.store(0.0_f32.to_bits(), Ordering::Relaxed);
    MOSFET_TEMPERATURE.store(0.0_f32.to_bits(), Ordering::Relaxed);
    MOTOR_TEMPERATURE.store(0.0_f32.to_bits(), Ordering::Relaxed);
    ODOMETER.store(0, Ordering::Relaxed);
    AMP_HOURS_DISCHARGED.store(0.0_f32.to_bits(), Ordering::Relaxed);
    AMP_HOURS_CHARGED.store(0.0_f32.to_bits(), Ordering::Relaxed);
    WATT_HOURS_DISCHARGED.store(0.0_f32.to_bits(), Ordering::Relaxed);
    WATT_HOURS_CHARGED.store(0.0_f32.to_bits(), Ordering::Relaxed);
    BATTERY_LEVEL.store(0.0_f32.to_bits(), Ordering::Relaxed);
    FIRMWARE_FAULT.store(0, Ordering::Relaxed);
    INPUT_VOLTAGE.store(0.0_f32.to_bits(), Ordering::Relaxed);
    IMU_STARTUP_DONE.store(false, Ordering::Relaxed);
    store(&IMU_ROLL, 0.0);
    store(&IMU_PITCH, 0.0);
    store(&IMU_YAW, 0.0);
    IMU_GYRO.iter().for_each(|axis| store(axis, 0.0));
    [1.0, 0.0, 0.0, 0.0]
        .into_iter()
        .zip(&IMU_QUATERNION)
        .for_each(|(value, component)| store(component, value));
    THREAD_SPAWN_COUNT.store(0, Ordering::Relaxed);
    THREAD_SPAWN_STACKS
        .iter()
        .for_each(|slot| slot.store(0, Ordering::Relaxed));
    [1, 2]
        .into_iter()
        .zip(&THREAD_SPAWN_RESULTS)
        .for_each(|(value, slot)| slot.store(value, Ordering::Relaxed));
    THREAD_TERMINATE_COUNT.store(0, Ordering::Relaxed);
    THREAD_TERMINATED
        .iter()
        .for_each(|slot| slot.store(0, Ordering::Relaxed));
    THREAD_TERMINATE_CHECKS.store(0, Ordering::Relaxed);
    THREAD_TERMINATE_AFTER.store(usize::MAX, Ordering::Relaxed);
    THREAD_SLEEP_COUNT.store(0, Ordering::Relaxed);
    THREAD_SLEEP_MICROS
        .iter()
        .for_each(|slot| slot.store(0, Ordering::Relaxed));
    THREAD_PRIORITY_COUNT.store(0, Ordering::Relaxed);
    THREAD_PRIORITIES
        .iter()
        .for_each(|slot| slot.store(0, Ordering::Relaxed));
    EEPROM_PRESENT
        .iter()
        .for_each(|slot| slot.store(false, Ordering::Relaxed));
    EEPROM_WRITE_FAILURE.store(-1, Ordering::Relaxed);
    NVM.iter().for_each(|byte| byte.store(0, Ordering::Relaxed));
    NVM_FAILURE.store(false, Ordering::Relaxed);
    CLOCK_TICKS.store(0, Ordering::Relaxed);
    TIMER_TICKS.store(0, Ordering::Relaxed);
    MUTEX_LOCK_COUNT.store(0, Ordering::Relaxed);
    MUTEX_UNLOCK_COUNT.store(0, Ordering::Relaxed);
    MUTEX_FREE_COUNT.store(0, Ordering::Relaxed);
    MUTEX_CREATE_FAILURE.store(false, Ordering::Relaxed);
    SEMAPHORE_WAIT_COUNT.store(0, Ordering::Relaxed);
    SEMAPHORE_TIMED_WAIT_TICKS.store(u32::MAX, Ordering::Relaxed);
    SEMAPHORE_SIGNAL_COUNT.store(0, Ordering::Relaxed);
    SEMAPHORE_RESET_COUNT.store(0, Ordering::Relaxed);
    SEMAPHORE_FREE_COUNT.store(0, Ordering::Relaxed);
    SEMAPHORE_CREATE_FAILURE.store(false, Ordering::Relaxed);
    SEMAPHORE_TIMEOUT_FAILURE.store(false, Ordering::Relaxed);
    FirmwareLockGuard
}

pub unsafe fn read_eeprom_word(word: *mut u32, address: i32) -> bool {
    let Some(index) = usize::try_from(address)
        .ok()
        .filter(|index| *index < EEPROM_WORDS)
    else {
        return false;
    };
    if !EEPROM_PRESENT[index].load(Ordering::Relaxed) {
        return false;
    }
    if let Some(word) = unsafe { word.as_mut() } {
        *word = EEPROM[index].load(Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub unsafe fn store_eeprom_word(word: *mut u32, address: i32) -> bool {
    let Some(index) = usize::try_from(address)
        .ok()
        .filter(|index| *index < EEPROM_WORDS)
    else {
        return false;
    };
    if EEPROM_WRITE_FAILURE.load(Ordering::Relaxed) == address {
        return false;
    }
    let Some(word) = (unsafe { word.as_ref() }) else {
        return false;
    };
    EEPROM[index].store(*word, Ordering::Relaxed);
    EEPROM_PRESENT[index].store(true, Ordering::Relaxed);
    true
}

pub(crate) fn fail_eeprom_write(address: crate::CustomEepromAddress) {
    EEPROM_WRITE_FAILURE.store(address.get(), Ordering::Relaxed);
}

pub unsafe fn read_nvm(buffer: *mut u8, offset: u32, len: u32) -> Option<bool> {
    let Some(end) = offset
        .checked_add(len)
        .and_then(|end| usize::try_from(end).ok())
    else {
        return Some(false);
    };
    let start = usize::try_from(offset).ok()?;
    let Some(buffer) = core::ptr::NonNull::new(buffer) else {
        return Some(false);
    };
    if end > NVM_BYTES || NVM_FAILURE.load(Ordering::Relaxed) {
        return Some(false);
    }
    for index in 0..usize::try_from(len).ok()? {
        unsafe {
            buffer
                .as_ptr()
                .add(index)
                .write(NVM[start + index].load(Ordering::Relaxed));
        }
    }
    Some(true)
}

pub unsafe fn write_nvm(buffer: *mut u8, offset: u32, len: u32) -> Option<bool> {
    let Some(end) = offset
        .checked_add(len)
        .and_then(|end| usize::try_from(end).ok())
    else {
        return Some(false);
    };
    let start = usize::try_from(offset).ok()?;
    let Some(buffer) = core::ptr::NonNull::new(buffer) else {
        return Some(false);
    };
    if end > NVM_BYTES || NVM_FAILURE.load(Ordering::Relaxed) {
        return Some(false);
    }
    for index in 0..usize::try_from(len).ok()? {
        let byte = unsafe { buffer.as_ptr().add(index).read() };
        NVM[start + index].store(byte, Ordering::Relaxed);
    }
    Some(true)
}

pub unsafe fn wipe_nvm() -> Option<bool> {
    if NVM_FAILURE.load(Ordering::Relaxed) {
        return Some(false);
    }
    NVM.iter().for_each(|byte| byte.store(0, Ordering::Relaxed));
    Some(true)
}

pub(crate) fn fail_nvm_operations(fail: bool) {
    NVM_FAILURE.store(fail, Ordering::Relaxed);
}

pub unsafe fn lbm_is_number(value: LbmValue) -> bool {
    value.0 & 0x0f == 0x08
}

pub unsafe fn lbm_dec_as_u32(value: LbmValue) -> u32 {
    (value.0 as i32 >> 4) as u32
}

pub unsafe fn lbm_dec_as_i32(value: LbmValue) -> i32 {
    (value.0 as i32) >> 4
}

pub unsafe fn lbm_enc_i(value: i32) -> LbmValue {
    LbmValue((value << 4) as u32 | 0x08)
}

pub unsafe fn lbm_dec_char(value: LbmValue) -> u8 {
    (value.0 >> 4) as u8
}

pub unsafe fn lbm_enc_char(value: u8) -> LbmValue {
    LbmValue((value as u32) << 4 | 0x04)
}

pub unsafe fn lbm_enc_u32(value: u32) -> LbmValue {
    LbmValue(value << 4 | 0x08)
}

pub unsafe fn lbm_is_char(value: LbmValue) -> bool {
    value.0 & 0x0f == 0x04
}

pub unsafe fn lbm_is_symbol(_value: LbmValue) -> bool {
    false
}

pub unsafe fn lbm_is_cons(_value: LbmValue) -> bool {
    false
}

pub unsafe fn lbm_is_byte_array(_value: LbmValue) -> bool {
    false
}

pub unsafe fn vesc_system_time_ticks() -> u32 {
    CLOCK_TICKS.load(Ordering::Relaxed)
}

pub unsafe fn vesc_system_time_seconds() -> f32 {
    CLOCK_TICKS.load(Ordering::Relaxed) as f32 / 10_000.0
}

pub unsafe fn vesc_timestamp_age_seconds(timestamp: u32) -> f32 {
    CLOCK_TICKS.load(Ordering::Relaxed).wrapping_sub(timestamp) as f32 / 10_000.0
}

pub unsafe fn vesc_timer_time_now() -> u32 {
    TIMER_TICKS.load(Ordering::Relaxed)
}

pub unsafe fn vesc_timer_seconds_elapsed_since(timestamp: u32) -> f32 {
    TIMER_TICKS.load(Ordering::Relaxed).wrapping_sub(timestamp) as f32 / 1_000_000.0
}

pub(crate) fn set_clock_ticks(ticks: u32) {
    CLOCK_TICKS.store(ticks, Ordering::Relaxed);
}

pub(crate) fn set_timer_ticks(ticks: u32) {
    TIMER_TICKS.store(ticks, Ordering::Relaxed);
}

pub unsafe fn vesc_mutex_create() -> *mut c_void {
    if MUTEX_CREATE_FAILURE.load(Ordering::Relaxed) {
        return core::ptr::null_mut();
    }
    core::ptr::addr_of!(MUTEX_TOKEN).cast::<c_void>().cast_mut()
}

pub unsafe fn vesc_mutex_lock(_mutex: *mut c_void) {
    MUTEX_LOCK_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub unsafe fn vesc_mutex_unlock(_mutex: *mut c_void) {
    MUTEX_UNLOCK_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub unsafe fn vesc_sem_create() -> *mut c_void {
    if SEMAPHORE_CREATE_FAILURE.load(Ordering::Relaxed) {
        return core::ptr::null_mut();
    }
    core::ptr::addr_of!(SEMAPHORE_TOKEN)
        .cast::<c_void>()
        .cast_mut()
}

pub unsafe fn vesc_sem_wait(_semaphore: *mut c_void) {
    SEMAPHORE_WAIT_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub unsafe fn vesc_sem_wait_to(_semaphore: *mut c_void, ticks: u32) -> bool {
    SEMAPHORE_TIMED_WAIT_TICKS.store(ticks, Ordering::Relaxed);
    !SEMAPHORE_TIMEOUT_FAILURE.load(Ordering::Relaxed)
}

pub unsafe fn vesc_sem_signal(_semaphore: *mut c_void) {
    SEMAPHORE_SIGNAL_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub unsafe fn vesc_sem_reset(_semaphore: *mut c_void) {
    SEMAPHORE_RESET_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub unsafe fn vesc_free(pointer: *mut c_void) {
    if core::ptr::eq(
        pointer.cast_const(),
        core::ptr::addr_of!(SEMAPHORE_TOKEN).cast::<c_void>(),
    ) {
        SEMAPHORE_FREE_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        MUTEX_FREE_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn mutex_lock_count() -> usize {
    MUTEX_LOCK_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn mutex_unlock_count() -> usize {
    MUTEX_UNLOCK_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn mutex_free_count() -> usize {
    MUTEX_FREE_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn semaphore_wait_count() -> usize {
    SEMAPHORE_WAIT_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn semaphore_timed_wait_ticks() -> Option<u32> {
    match SEMAPHORE_TIMED_WAIT_TICKS.load(Ordering::Relaxed) {
        u32::MAX => None,
        ticks => Some(ticks),
    }
}

pub(crate) fn semaphore_signal_count() -> usize {
    SEMAPHORE_SIGNAL_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn semaphore_reset_count() -> usize {
    SEMAPHORE_RESET_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn semaphore_free_count() -> usize {
    SEMAPHORE_FREE_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn fail_mutex_creation(fail: bool) {
    MUTEX_CREATE_FAILURE.store(fail, Ordering::Relaxed);
}

pub(crate) fn fail_semaphore_creation(fail: bool) {
    SEMAPHORE_CREATE_FAILURE.store(fail, Ordering::Relaxed);
}

pub(crate) fn fail_semaphore_timeout(fail: bool) {
    SEMAPHORE_TIMEOUT_FAILURE.store(fail, Ordering::Relaxed);
}

fn load(value: &AtomicU32) -> f32 {
    f32::from_bits(value.load(Ordering::Relaxed))
}

fn store(value: &AtomicU32, raw: f32) {
    value.store(raw.to_bits(), Ordering::Relaxed);
}

pub(crate) fn set_runtime_motor(
    electrical_speed: ElectricalSpeed,
    vehicle_speed: VehicleSpeed,
    motor_current: TotalMotorCurrent,
    input_current: InputCurrent,
    duty_cycle: DutyCycle,
) {
    store(
        &ELECTRICAL_SPEED,
        electrical_speed.rpm().as_revolutions_per_minute(),
    );
    store(&VEHICLE_SPEED, vehicle_speed.speed().as_meters_per_second());
    store(&MOTOR_CURRENT, motor_current.current().as_amps());
    store(&INPUT_CURRENT, input_current.current().as_amps());
    store(&DUTY_CYCLE, duty_cycle.ratio().as_ratio());
}

pub(crate) fn set_motor_current_limits(max: MotorCurrentLimit, min: MotorCurrentLimit) {
    store(&MOTOR_CURRENT_MAX, max.current().as_amps());
    store(&MOTOR_CURRENT_MIN, -min.current().as_amps());
}

pub(crate) fn set_duty_cycle_limit(limit: crate::DutyCycleLimit) {
    store(&DUTY_CYCLE_LIMIT, limit.ratio().as_ratio());
}

pub(crate) fn set_temperature_limit_starts(
    mosfet: crate::TemperatureLimitStart,
    motor: crate::TemperatureLimitStart,
) {
    store(
        &MOSFET_TEMPERATURE_LIMIT_START,
        mosfet.temperature().as_degrees_celsius(),
    );
    store(
        &MOTOR_TEMPERATURE_LIMIT_START,
        motor.temperature().as_degrees_celsius(),
    );
}

pub(crate) fn set_battery_cell_count(count: crate::BatteryCellCount) {
    BATTERY_CELL_COUNT.store(i32::from(count.as_u16()), Ordering::Relaxed);
}

pub(crate) fn set_directional_motor_current(current: DirectionalMotorCurrent) {
    store(&DIRECTIONAL_MOTOR_CURRENT, current.current().as_amps());
}

pub(crate) fn set_distance_abs(distance: TripDistance) {
    store(&DISTANCE_ABS, distance.distance().as_meters());
}

pub(crate) fn set_temperatures(mosfet: MosfetTemperature, motor: MotorTemperature) {
    store(
        &MOSFET_TEMPERATURE,
        mosfet.temperature().as_degrees_celsius(),
    );
    store(&MOTOR_TEMPERATURE, motor.temperature().as_degrees_celsius());
}

pub(crate) fn set_ride_totals(
    odometer: OdometerMeters,
    amp_hours_discharged: AmpHoursDischarged,
    amp_hours_charged: AmpHoursCharged,
    watt_hours_discharged: WattHoursDischarged,
    watt_hours_charged: WattHoursCharged,
    battery_level: BatteryLevel,
) {
    ODOMETER.store(odometer.as_meters(), Ordering::Relaxed);
    store(
        &AMP_HOURS_DISCHARGED,
        amp_hours_discharged.charge().as_amp_hours(),
    );
    store(
        &AMP_HOURS_CHARGED,
        amp_hours_charged.charge().as_amp_hours(),
    );
    store(
        &WATT_HOURS_DISCHARGED,
        watt_hours_discharged.energy().as_watt_hours(),
    );
    store(
        &WATT_HOURS_CHARGED,
        watt_hours_charged.energy().as_watt_hours(),
    );
    store(&BATTERY_LEVEL, battery_level.as_fraction());
}

pub(crate) fn set_firmware_fault(fault: FirmwareFaultCode) {
    FIRMWARE_FAULT.store(
        crate::FirmwareFaultWireCode::try_from(fault)
            .map_or(0, |fault| i32::from(fault.wire_code())),
        Ordering::Relaxed,
    );
}

pub(crate) fn set_input_voltage(voltage: InputVoltage) {
    store(&INPUT_VOLTAGE, voltage.voltage().as_volts());
}

pub(crate) fn set_foc_id_current(current: Option<DCurrent>) {
    HAS_FOC_ID_CURRENT.store(current.is_some(), Ordering::Relaxed);
    if let Some(value) = current {
        store(&FOC_ID_CURRENT, value.current().as_amps());
    }
}

pub(crate) fn set_imu_startup_done(done: bool) {
    IMU_STARTUP_DONE.store(done, Ordering::Relaxed);
}

pub(crate) fn set_imu_attitude(roll: ImuRoll, pitch: ImuPitch, yaw: ImuYaw) {
    store(&IMU_ROLL, roll.angle().as_radians());
    store(&IMU_PITCH, pitch.angle().as_radians());
    store(&IMU_YAW, yaw.angle().as_radians());
}

pub(crate) fn set_imu_angular_rate(rate: ImuAngularRate) {
    [rate.roll(), rate.pitch(), rate.yaw()]
        .into_iter()
        .zip(&IMU_GYRO)
        .for_each(|(axis, slot)| store(slot, axis.as_degrees_per_second()));
}

pub(crate) fn set_imu_orientation(orientation: ImuOrientation) {
    let quaternion = orientation.quaternion();
    [
        f32::from(quaternion.w()),
        f32::from(quaternion.x()),
        f32::from(quaternion.y()),
        f32::from(quaternion.z()),
    ]
    .into_iter()
    .zip(&IMU_QUATERNION)
    .for_each(|(component, slot)| store(slot, component));
}

pub(crate) fn fail_second_thread_spawn() {
    THREAD_SPAWN_RESULTS[1].store(0, Ordering::Relaxed);
}

pub(crate) fn terminate_threads_after_checks(checks: usize) {
    THREAD_TERMINATE_AFTER.store(checks, Ordering::Relaxed);
}

pub(crate) fn thread_spawn_count() -> usize {
    THREAD_SPAWN_COUNT.load(Ordering::Relaxed)
}
pub(crate) fn thread_spawn_stacks() -> [usize; 2] {
    THREAD_SPAWN_STACKS
        .each_ref()
        .map(|slot| slot.load(Ordering::Relaxed))
}
pub(crate) fn thread_termination_count() -> usize {
    THREAD_TERMINATE_COUNT.load(Ordering::Relaxed)
}
pub(crate) fn thread_terminated() -> [usize; 2] {
    THREAD_TERMINATED
        .each_ref()
        .map(|slot| slot.load(Ordering::Relaxed))
}
pub(crate) fn thread_termination_check_count() -> usize {
    THREAD_TERMINATE_CHECKS.load(Ordering::Relaxed)
}
pub(crate) fn thread_sleep_count() -> usize {
    THREAD_SLEEP_COUNT.load(Ordering::Relaxed)
}
pub(crate) fn thread_sleep_micros() -> [u32; 2] {
    THREAD_SLEEP_MICROS
        .each_ref()
        .map(|slot| slot.load(Ordering::Relaxed))
}
pub(crate) fn thread_priority_count() -> usize {
    THREAD_PRIORITY_COUNT.load(Ordering::Relaxed)
}
pub(crate) fn thread_priorities() -> [i32; 2] {
    THREAD_PRIORITIES
        .each_ref()
        .map(|slot| slot.load(Ordering::Relaxed))
}

pub(crate) fn motor_output() -> MotorOutputState {
    MotorOutputState {
        keep_alive_count: KEEP_ALIVE_COUNT.load(Ordering::Relaxed),
        current_off_delay_count: CURRENT_OFF_DELAY_COUNT.load(Ordering::Relaxed),
        current_count: CURRENT_COUNT.load(Ordering::Relaxed),
        duty_count: DUTY_COUNT.load(Ordering::Relaxed),
        brake_current_count: BRAKE_CURRENT_COUNT.load(Ordering::Relaxed),
        current_off_delay: f32::from_bits(CURRENT_OFF_DELAY.load(Ordering::Relaxed)),
        current: f32::from_bits(CURRENT.load(Ordering::Relaxed)),
        duty: f32::from_bits(DUTY.load(Ordering::Relaxed)),
        brake_current: f32::from_bits(BRAKE_CURRENT.load(Ordering::Relaxed)),
    }
}

pub unsafe fn timeout_reset() {
    KEEP_ALIVE_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub unsafe fn mc_set_current_off_delay(seconds: f32) {
    CURRENT_OFF_DELAY_COUNT.fetch_add(1, Ordering::Relaxed);
    CURRENT_OFF_DELAY.store(seconds.to_bits(), Ordering::Relaxed);
}

pub unsafe fn mc_set_current(amps: f32) {
    CURRENT_COUNT.fetch_add(1, Ordering::Relaxed);
    CURRENT.store(amps.to_bits(), Ordering::Relaxed);
}

pub unsafe fn mc_set_duty(duty: f32) {
    DUTY_COUNT.fetch_add(1, Ordering::Relaxed);
    DUTY.store(duty.to_bits(), Ordering::Relaxed);
}

pub unsafe fn mc_set_brake_current(amps: f32) {
    BRAKE_CURRENT_COUNT.fetch_add(1, Ordering::Relaxed);
    BRAKE_CURRENT.store(amps.to_bits(), Ordering::Relaxed);
}

pub unsafe fn mc_get_rpm() -> f32 {
    load(&ELECTRICAL_SPEED)
}

pub unsafe fn mc_get_speed() -> f32 {
    load(&VEHICLE_SPEED)
}

pub unsafe fn mc_get_tot_current_filtered() -> f32 {
    load(&MOTOR_CURRENT)
}

pub unsafe fn mc_get_tot_current_directional_filtered() -> f32 {
    load(&DIRECTIONAL_MOTOR_CURRENT)
}

pub unsafe fn get_cfg_float(param: i32) -> f32 {
    match param {
        0 => load(&MOTOR_CURRENT_MAX),
        1 => load(&MOTOR_CURRENT_MIN),
        16 => load(&MOSFET_TEMPERATURE_LIMIT_START),
        18 => load(&MOTOR_TEMPERATURE_LIMIT_START),
        22 => load(&DUTY_CYCLE_LIMIT),
        _ => 0.0,
    }
}

pub unsafe fn get_cfg_int(param: i32) -> i32 {
    match param {
        43 => BATTERY_CELL_COUNT.load(Ordering::Relaxed),
        _ => 0,
    }
}

pub unsafe fn mc_get_tot_current_in_filtered() -> f32 {
    load(&INPUT_CURRENT)
}

pub unsafe fn mc_get_duty_cycle_now() -> f32 {
    load(&DUTY_CYCLE)
}

pub unsafe fn foc_get_id() -> Option<f32> {
    HAS_FOC_ID_CURRENT
        .load(Ordering::Relaxed)
        .then(|| load(&FOC_ID_CURRENT))
}

pub unsafe fn mc_get_distance_abs() -> f32 {
    load(&DISTANCE_ABS)
}

pub unsafe fn mc_temp_fet_filtered() -> f32 {
    load(&MOSFET_TEMPERATURE)
}

pub unsafe fn mc_temp_motor_filtered() -> f32 {
    load(&MOTOR_TEMPERATURE)
}

pub unsafe fn mc_get_odometer() -> u64 {
    ODOMETER.load(Ordering::Relaxed)
}

pub unsafe fn mc_get_amp_hours(_reset: bool) -> f32 {
    load(&AMP_HOURS_DISCHARGED)
}

pub unsafe fn mc_get_amp_hours_charged(_reset: bool) -> f32 {
    load(&AMP_HOURS_CHARGED)
}

pub unsafe fn mc_get_watt_hours(_reset: bool) -> f32 {
    load(&WATT_HOURS_DISCHARGED)
}

pub unsafe fn mc_get_watt_hours_charged(_reset: bool) -> f32 {
    load(&WATT_HOURS_CHARGED)
}

pub unsafe fn mc_get_battery_level(_wh_left: *mut f32) -> f32 {
    load(&BATTERY_LEVEL)
}

pub unsafe fn mc_get_fault() -> i32 {
    FIRMWARE_FAULT.load(Ordering::Relaxed)
}

pub unsafe fn mc_get_input_voltage_filtered() -> f32 {
    load(&INPUT_VOLTAGE)
}

pub unsafe fn imu_startup_done() -> bool {
    IMU_STARTUP_DONE.load(Ordering::Relaxed)
}

pub unsafe fn imu_get_roll() -> f32 {
    load(&IMU_ROLL)
}

pub unsafe fn imu_get_pitch() -> f32 {
    load(&IMU_PITCH)
}

pub unsafe fn imu_get_yaw() -> f32 {
    load(&IMU_YAW)
}

pub unsafe fn imu_get_gyro(values: *mut f32) {
    if let Some(values) = unsafe { values.cast::<[f32; 3]>().as_mut() } {
        values
            .iter_mut()
            .zip(&IMU_GYRO)
            .for_each(|(value, axis)| *value = load(axis));
    }
}

pub unsafe fn vesc_imu_get_quaternions(values: *mut f32) {
    if let Some(values) = unsafe { values.cast::<[f32; 4]>().as_mut() } {
        values
            .iter_mut()
            .zip(&IMU_QUATERNION)
            .for_each(|(value, component)| *value = load(component));
    }
}

pub unsafe fn vesc_spawn(
    _entry: unsafe extern "C" fn(*mut c_void),
    stack_bytes: usize,
    _name: *const c_char,
    _arg: *mut c_void,
) -> *mut c_void {
    let call = THREAD_SPAWN_COUNT.fetch_add(1, Ordering::Relaxed);
    let index = call.min(1);
    THREAD_SPAWN_STACKS[index].store(stack_bytes, Ordering::Relaxed);
    THREAD_SPAWN_RESULTS[index].load(Ordering::Relaxed) as *mut c_void
}

pub unsafe fn vesc_request_terminate(thread: *mut c_void) {
    let call = THREAD_TERMINATE_COUNT.fetch_add(1, Ordering::Relaxed);
    THREAD_TERMINATED[call.min(1)].store(thread as usize, Ordering::Relaxed);
}

pub unsafe fn vesc_should_terminate() -> bool {
    THREAD_TERMINATE_CHECKS.fetch_add(1, Ordering::Relaxed) + 1
        >= THREAD_TERMINATE_AFTER.load(Ordering::Relaxed)
}

pub unsafe fn vesc_sleep_us(micros: u32) {
    let call = THREAD_SLEEP_COUNT.fetch_add(1, Ordering::Relaxed);
    THREAD_SLEEP_MICROS[call.min(1)].store(micros, Ordering::Relaxed);
}

pub unsafe fn vesc_thread_set_priority(priority: i32) -> bool {
    let call = THREAD_PRIORITY_COUNT.fetch_add(1, Ordering::Relaxed);
    THREAD_PRIORITIES[call.min(1)].store(priority, Ordering::Relaxed);
    true
}
