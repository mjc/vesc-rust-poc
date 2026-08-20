use vescpkg_rs::prelude::Frequency;

// The package schema caps loop frequency at 4 kHz; the 8 Hz source formula
// needs 221 samples there. One spare slot keeps the fixed storage bounded
// without importing the branch's full u8 address space into package state.
const MAX_WINDOW: usize = 222;
const ABS_ERPM_CUTOFF: Frequency = Frequency::from_hertz(10.0);
const ACCELERATION_CUTOFF: Frequency = Frequency::from_hertz(8.0);

pub(super) const MOTOR_KINEMATICS_CONFIG: vescpkg_rs::MotorKinematicsConfig =
    vescpkg_rs::MotorKinematicsConfig::new(ABS_ERPM_CUTOFF, ACCELERATION_CUTOFF);

#[pin_init::pin_init]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MotorKinematicsTracker(pub(super) vescpkg_rs::MotorKinematics<MAX_WINDOW>);

#[cfg(target_arch = "arm")]
const _: [(); 912] = [(); core::mem::size_of::<MotorKinematicsTracker>()];

#[cfg(test)]
impl Default for MotorKinematicsTracker {
    fn default() -> Self {
        let mut tracker = vescpkg_rs::MotorKinematics::default();
        MOTOR_KINEMATICS_CONFIG.configure(
            &mut tracker,
            crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE,
        );
        Self(tracker)
    }
}

impl MotorKinematicsTracker {
    pub(super) fn default_in_place() -> impl pin_init::Init<Self, core::convert::Infallible> {
        pin_init::init_from_closure(|state| {
            let mut state = state.init(pin_init::init_pin!(MotorKinematicsTracker(
                vescpkg_rs::MotorKinematics::default()
            )))?;
            MOTOR_KINEMATICS_CONFIG.configure(
                &mut state.as_mut().get_mut().0,
                crate::config::FLOAT_OUT_BOY_MAIN_THREAD_SAMPLE_RATE,
            );
            Ok(state)
        })
    }
}

#[cfg(test)]
mod tests;
