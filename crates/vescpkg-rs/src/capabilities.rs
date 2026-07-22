//! Capability-aware safe subsystem constructors.

use crate::{CanBus, FocAudio, Nvm, NvmCapacity, Uart};
use vescpkg_rs_sys::{AbiError, Stm32AbiRevision, VescIfCapabilities, VescIfPresence};

/// Observed firmware capabilities used to construct safe subsystem handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareCapabilities {
    inner: VescIfCapabilities,
}

/// Checked settings capability marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareSettings;

impl FirmwareCapabilities {
    /// Construct capabilities from one bounded table-presence snapshot.
    pub const fn new(presence: VescIfPresence) -> Self {
        Self {
            inner: VescIfCapabilities::new(presence),
        }
    }

    /// Return the observed slot presence used by this value.
    pub const fn presence(self) -> VescIfPresence {
        self.inner.presence()
    }

    /// Return the descriptive revision inferred from observed pointers.
    pub fn revision(self) -> Stm32AbiRevision {
        self.inner.revision()
    }

    /// Construct a CAN handle only when its observed transmit entry exists.
    pub fn can_bus(self) -> Result<CanBus, AbiError> {
        self.inner.can().map(|_| CanBus::new())
    }

    /// Construct an NVM handle only when its observed read entry exists.
    pub fn nvm(self) -> Result<Nvm, AbiError> {
        self.inner.nvm().map(|_| Nvm::new())
    }

    /// Construct NVM with a separately discovered byte capacity.
    pub fn nvm_with_capacity(self, capacity: NvmCapacity) -> Result<Nvm, AbiError> {
        self.inner.nvm().map(|_| Nvm::with_capacity(capacity))
    }

    /// Construct an FOC audio handle only when its observed beep entry exists.
    pub fn audio(self) -> Result<FocAudio, AbiError> {
        self.inner.audio().map(|_| FocAudio::new())
    }

    /// Construct a UART handle only when its observed start entry exists.
    pub fn uart(self) -> Result<Uart, AbiError> {
        self.inner.uart().map(|_| Uart::new())
    }

    /// Construct a settings marker only when its observed getter exists.
    pub fn settings(self) -> Result<FirmwareSettings, AbiError> {
        self.inner.settings().map(|_| FirmwareSettings)
    }

    /// Require CAN for a constructor that cannot operate without it.
    pub fn require_can(self) -> Result<CanBus, AbiError> {
        self.inner.require_can().map(|_| CanBus::new())
    }

    /// Require settings for a constructor that cannot operate without it.
    pub fn require_settings(self) -> Result<FirmwareSettings, AbiError> {
        self.inner.require_settings().map(|_| FirmwareSettings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vescpkg_rs_sys::VescIfAbi;

    #[test]
    fn safe_capability_constructors_follow_observed_presence() {
        let mut words = [0_usize; VescIfAbi::FIELD_COUNT];
        words[VescIfAbi::CAN_TRANSMIT_SID.slot_index()] = 1;
        words[VescIfAbi::READ_NVM.slot_index()] = 1;
        let capabilities = FirmwareCapabilities::new(VescIfPresence::from_words(&words));

        assert!(capabilities.can_bus().is_ok());
        assert!(capabilities.nvm().is_ok());
        assert_eq!(
            capabilities
                .nvm_with_capacity(NvmCapacity::new(32).unwrap())
                .unwrap()
                .capacity()
                .unwrap()
                .get(),
            32
        );
        assert_eq!(capabilities.audio().unwrap_err().capability(), "FOC audio");
        assert_eq!(capabilities.uart().unwrap_err().capability(), "UART");
        assert_eq!(capabilities.settings().unwrap_err().capability(), "settings");
    }

    #[test]
    fn safe_required_constructor_preserves_missing_slot_diagnostics() {
        let capabilities = FirmwareCapabilities::new(VescIfPresence::empty());

        let error = match capabilities.require_can() {
            Err(error) => error,
            Ok(_) => panic!("empty presence must reject required CAN"),
        };
        assert_eq!(error.capability(), "CAN");
        assert_eq!(error.slot(), VescIfAbi::CAN_TRANSMIT_SID);
        assert_eq!(capabilities.revision(), Stm32AbiRevision::UnknownCompatible);
        assert_eq!(capabilities.require_settings().unwrap_err().capability(), "settings");
    }
}
