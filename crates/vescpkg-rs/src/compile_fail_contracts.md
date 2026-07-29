
# Compile-time contracts

These examples protect API boundaries that ordinary runtime tests cannot
exercise. Each block checks one invariant so one remaining compiler error
cannot hide a regression in another.

Callback state must implement `PackageRuntimeState`:

```compile_fail
struct State;
struct Callback;

impl vescpkg_rs::AppDataHandler for Callback {
    type State = State;

    fn handle(
        _context: &mut vescpkg_rs::StatefulCallbackContext<'_, Self::State>,
        _packet: vescpkg_rs::AppDataPacket<'_>,
        _reply: &mut vescpkg_rs::AppDataReply<'_>,
    ) {
    }
}
```

Package-state references cannot escape a callback state phase:

```compile_fail
use vescpkg_rs::{
    PackageRuntimeState, PackageStateStore, StatefulCallbackContext,
};

struct State;

static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

fn escape_state<'a>(
    context: &'a mut StatefulCallbackContext<'_, State>,
) -> &'a mut State {
    context.with_state(|state| state)
}
```

Only SDK callback trampolines can begin a callback context:

```compile_fail
use vescpkg_rs::{
    PackageRuntimeState, PackageStateAccess, PackageStateStore, StatefulCallbackContext,
};

struct State;

static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

let access = PackageStateAccess::runtime(&STATE);
let _ = StatefulCallbackContext::begin(access);
```

Custom EEPROM access requires an SDK-issued firmware-effect phase:

```compile_fail
use vescpkg_rs::{CustomEeprom, CustomEepromAddress};

let address = CustomEepromAddress::from_index(0).unwrap();
let _ = CustomEeprom::new().read(address);
```

Firmware-setting mutation requires an SDK-issued firmware-effect phase:

```compile_fail
use vescpkg_rs::{FirmwareFloatSetting, FirmwareSettings};

let _ = FirmwareSettings.set_float(FirmwareFloatSetting::MotorCurrentMax, 1.0);
```

Firmware log transport requires an SDK-issued firmware-effect phase:

```compile_fail
let log = vescpkg_rs::FirmwareLog::<8>::new();
let _ = log.flush();
```

Firmware-effect permission cannot be constructed by package code:

```compile_fail
let _ = vescpkg_rs::FirmwareEffects { _private: () };
```

An effect phase cannot be nested inside a callback state phase:

```compile_fail
use vescpkg_rs::{
    AppDataHandler, AppDataPacket, AppDataReply, PackageRuntimeState,
    PackageStateStore, StatefulCallbackContext,
};

struct State;
static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

struct Callback;

impl AppDataHandler for Callback {
    type State = State;

    fn handle(
        context: &mut StatefulCallbackContext<'_, State>,
        _packet: AppDataPacket<'_>,
        _reply: &mut AppDataReply<'_>,
    ) {
        context.with_state(|_| context.with_effects(|_| ()));
    }
}
```

A callback state phase cannot be nested inside an effect phase:

```compile_fail
use vescpkg_rs::{
    AppDataHandler, AppDataPacket, AppDataReply, PackageRuntimeState,
    PackageStateStore, StatefulCallbackContext,
};

struct State;
static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

struct Callback;

impl AppDataHandler for Callback {
    type State = State;

    fn handle(
        context: &mut StatefulCallbackContext<'_, State>,
        _packet: AppDataPacket<'_>,
        _reply: &mut AppDataReply<'_>,
    ) {
        context.with_effects(|_| context.with_state(|_| ()));
    }
}
```

A thread effect phase cannot be nested inside a mutable state phase:

```compile_fail
use vescpkg_rs::{
    FirmwareThread, PackageRuntimeState, PackageStateStore, ThreadContext,
};

struct State;
static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

struct Thread;

impl FirmwareThread for Thread {
    type State = State;

    fn run(mut context: ThreadContext<State>) {
        let _ = context.with_state_mut(|_| context.with_effects(|_| ()));
    }
}
```

A thread mutable state phase cannot be nested inside an effect phase:

```compile_fail
use vescpkg_rs::{
    FirmwareThread, PackageRuntimeState, PackageStateStore, ThreadContext,
};

struct State;
static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

struct Thread;

impl FirmwareThread for Thread {
    type State = State;

    fn run(mut context: ThreadContext<State>) {
        context.with_effects(|_| {
            let _ = context.with_state_mut(|_| ());
        });
    }
}
```

A startup effect phase cannot be nested inside a mutable state phase:

```compile_fail
use vescpkg_rs::{
    PackageRuntimeState, PackageStart, PackageStateStore,
};

struct State;
static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

fn nested(start: &mut PackageStart<'_>) {
    let _ = start.with_runtime_state::<State, _>(|_| {
        start.with_effects(|_| ());
    });
}
```

A startup mutable state phase cannot be nested inside an effect phase:

```compile_fail
use vescpkg_rs::{
    PackageRuntimeState, PackageStart, PackageStateStore,
};

struct State;
static STATE: PackageStateStore<State> = PackageStateStore::new();

impl PackageRuntimeState for State {
    fn runtime_store() -> &'static PackageStateStore<Self> {
        &STATE
    }
}

fn nested(start: &mut PackageStart<'_>) {
    start.with_effects(|_| {
        let _ = start.with_runtime_state::<State, _>(|_| ());
    });
}
```

Macro implementation traits are not available at the package-author root:

```compile_fail
use vescpkg_rs::PackageAppDataCallback;
```

Direct app-data transport is not available to package code:

```compile_fail
use vescpkg_rs::FirmwareAppData;
```

```compile_fail
use vescpkg_rs::AppDataSendError;
```

```compile_fail
let _ = vescpkg_rs::Firmware::new().app_data();
```

App-data reply storage is runtime-owned:

```compile_fail
let _ = vescpkg_rs::AppDataResponse::new();
```

```compile_fail
use vescpkg_rs::AppDataReplyBuffer;
```

```compile_fail
let _ = vescpkg_rs::AppDataReply::new();
```

```compile_fail
use vescpkg_rs::PackageCustomConfigCallback;
```

```compile_fail
use vescpkg_rs::PackageImuReadCallback;
```

Macro implementation traits require an unsafe implementation:

```compile_fail
use vescpkg_rs::__macro_support::PackageAppDataCallback;

struct Callback;

impl PackageAppDataCallback for Callback {
    fn image_address() -> usize {
        0
    }
}
```

Firmware fallback state access remains internal:

```compile_fail
use core::ptr::NonNull;
use vescpkg_rs::{PackageStateAccess, PackageStateStore};

struct State;

unsafe fn firmware_state() -> Option<NonNull<State>> {
    None
}

let runtime = PackageStateStore::new();
let _ = unsafe { PackageStateAccess::with_firmware_fallback(&runtime, firmware_state) };
```

IMU acceleration and angular-rate vectors are not interchangeable:

```compile_fail
use vescpkg_rs::{
    AccelerationG, ImuAcceleration, ImuAccelerationX, ImuAccelerationY, ImuAccelerationZ,
    ImuAngularRate,
};

fn update_gyro(_: ImuAngularRate) {}

let accel = ImuAcceleration::from_axes(
    ImuAccelerationX::new(AccelerationG::from_g(0.0)),
    ImuAccelerationY::new(AccelerationG::from_g(0.0)),
    ImuAccelerationZ::new(AccelerationG::from_g(1.0)),
);
update_gyro(accel);
```

Legacy implementation details remain private:

```compile_fail
use vescpkg_rs::app_data_packet;
```

```compile_fail
use vescpkg_rs::bindings;
```

```compile_fail
use vescpkg_rs::encode_integer;
```

```compile_fail
use vescpkg_rs::ffi;
```

```compile_fail
use vescpkg_rs::PackageProgramAddress;
```

```compile_fail
use vescpkg_rs::ProtocolFrame;
```

```compile_fail
use vescpkg_rs::FirmwareThreadHandle;
```

```compile_fail
use vescpkg_rs::FirmwareThreadPair;
```

```compile_fail
use vescpkg_rs::FirmwareThreadPairSpec;
```

```compile_fail
use vescpkg_rs::FirmwareThreadSpec;
```

```compile_fail
use vescpkg_rs::ThreadGroup;
```

```compile_fail
use vescpkg_rs::ThreadHandle;
```

```compile_fail
use vescpkg_rs::WireCommand;
```

```compile_fail
use vescpkg_rs::WireVersion;
```

```compile_fail
use vescpkg_rs::LoaderInfo;
```

```compile_fail
let info: *mut vescpkg_rs::__macro_support::LoaderInfo = core::ptr::null_mut();
let _ = unsafe { vescpkg_rs::PackageStart::from_raw(info) };
```

Distinct motor semantics are not interchangeable:

```compile_fail
use vescpkg_rs::{Current, MotorCurrent, TotalMotorCurrent};

fn set_total_motor_current(_: TotalMotorCurrent) {}

set_total_motor_current(MotorCurrent::new(Current::from_amps(1.0)));
```

```compile_fail
use vescpkg_rs::{Current, InputCurrent, MotorCurrent};

fn set_input_current(_: InputCurrent) {}

set_input_current(MotorCurrent::new(Current::from_amps(1.0)));
```

```compile_fail
use vescpkg_rs::{Current, DCurrent, MotorCurrent};

fn set_d_current(_: DCurrent) {}

set_d_current(MotorCurrent::new(Current::from_amps(1.0)));
```

```compile_fail
use vescpkg_rs::{DutyCycle, SignedRatio};

fn set_duty_cycle(_: DutyCycle) {}

set_duty_cycle(SignedRatio::from_ratio_const(0.5));
```

```compile_fail
use vescpkg_rs::{BrakeCurrent, Current, MotorCurrent};

fn set_brake_current(_: BrakeCurrent) {}

set_brake_current(MotorCurrent::new(Current::from_amps(1.0)));
```

Package runtime state must be `Send`:

```compile_fail
use std::rc::Rc;

static STATE: vescpkg_rs::PackageStateStore<Rc<()>> = vescpkg_rs::PackageStateStore::new();
```

Non-zero firmware tokens do not implement `Default`:

```compile_fail
use vescpkg_rs::BaudRate;

fn requires_default<T: Default>() {}

requires_default::<BaudRate>();
```

```compile_fail
use vescpkg_rs::PacketLength;

fn requires_default<T: Default>() {}

requires_default::<PacketLength>();
```

Internal module layout is not public API:

```compile_fail
use vescpkg_rs::types;
```

```compile_fail
use vescpkg_rs::units;
```

Controller duty cycle and generic PWM are not interchangeable:

```compile_fail
use vescpkg_rs::{DutyCycle, Pwm, SignedRatio};

fn set_pwm(_: Pwm) {}

let duty = DutyCycle::new(SignedRatio::from_ratio_const(-0.25));
set_pwm(duty);
```

Raw package-state borrowing remains macro-internal:

```compile_fail
let _ = unsafe { vescpkg_rs::firmware_package_state_mut!(u32, main) };
```

Raw package startup remains macro-internal:

```compile_fail
let _ = vescpkg_rs::__macro_support::__package_start_from_raw(core::ptr::null_mut());
```

Mechanical and electrical speed are not interchangeable:

```compile_fail
use vescpkg_rs::{ElectricalSpeed, MechanicalSpeed, Rpm};

fn set_electrical_speed(_: ElectricalSpeed) {}

let mechanical = MechanicalSpeed::new(Rpm::from_revolutions_per_minute(3000.0));
set_electrical_speed(mechanical);
```

GPIO reads and writes require an exclusive lease:

```compile_fail
use vescpkg_rs::{AnalogPin, DigitalOutputLevel, DigitalPin, Firmware};

let firmware = Firmware::new();
let _ = firmware.gpio().read_analog(AnalogPin::ADC1);
let _ = firmware.gpio().configure_output(DigitalPin::HW_1);
let _ = firmware
    .gpio()
    .write(DigitalPin::HW_1, DigitalOutputLevel::High);
```
