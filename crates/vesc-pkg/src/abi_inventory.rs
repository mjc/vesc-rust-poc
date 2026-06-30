/// Classifies the role of an ABI requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiRequirementKind {
    /// Native entrypoint symbol.
    EntryPoint,
    /// Loader-provided header data.
    LoaderHeader,
    /// ABI type name that must be present.
    Type,
    /// Callable function symbol.
    Function,
    /// Special error symbol exported by the firmware.
    ErrorSymbol,
}

/// One required ABI symbol or type in the minimal package surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiRequirement {
    /// Symbol or type name to check.
    pub name: &'static str,
    /// Kind of ABI requirement.
    pub kind: AbiRequirementKind,
    /// Human-readable explanation of why the symbol matters.
    pub caller: &'static str,
}

impl AbiRequirement {
    /// Construct one ABI requirement entry.
    pub const fn new(name: &'static str, kind: AbiRequirementKind, caller: &'static str) -> Self {
        Self { name, kind, caller }
    }
}

/// ABI requirements that define the minimal loopback test package surface.
pub const MINIMAL_TEST_PACKAGE_ABI: [AbiRequirement; 12] = [
    AbiRequirement::new(
        "prog_ptr",
        AbiRequirementKind::LoaderHeader,
        "Rust package exports the VESC program pointer header word",
    ),
    AbiRequirement::new(
        "init",
        AbiRequirementKind::EntryPoint,
        "package loader calls the Rust-owned native init trampoline",
    ),
    AbiRequirement::new(
        "lib_info",
        AbiRequirementKind::Type,
        "entrypoint receives package metadata",
    ),
    AbiRequirement::new(
        "lbm_add_extension",
        AbiRequirementKind::Function,
        "Rust package registers LispBM extensions through VESC_IF",
    ),
    AbiRequirement::new(
        "lbm_value",
        AbiRequirementKind::Type,
        "extension ABI passes LispBM values",
    ),
    AbiRequirement::new(
        "lbm_uint",
        AbiRequirementKind::Type,
        "extension ABI carries argument counts",
    ),
    AbiRequirement::new(
        "lbm_dec_as_i32",
        AbiRequirementKind::Function,
        "Rust package decodes LispBM integer arguments",
    ),
    AbiRequirement::new(
        "lbm_enc_i",
        AbiRequirementKind::Function,
        "Rust package encodes integer results",
    ),
    AbiRequirement::new(
        "VESC_IF.lbm_enc_sym_eerror",
        AbiRequirementKind::ErrorSymbol,
        "Rust package returns the firmware eval-error symbol on bad arguments",
    ),
    AbiRequirement::new(
        "VESC_IF.set_app_data_handler",
        AbiRequirementKind::Function,
        "Rust package registers the BLE loopback app-data callback",
    ),
    AbiRequirement::new(
        "VESC_IF.send_app_data",
        AbiRequirementKind::Function,
        "Rust package sends BLE loopback replies through firmware app data",
    ),
    AbiRequirement::new(
        "VESC_IF.system_time_ticks",
        AbiRequirementKind::Function,
        "Rust package reads firmware ticks for loopback status replies",
    ),
];

/// Return the pinned ABI requirements for the loopback test package.
pub fn minimal_test_package_abi() -> &'static [AbiRequirement] {
    &MINIMAL_TEST_PACKAGE_ABI
}

#[cfg(test)]
mod tests {
    use super::{AbiRequirementKind, minimal_test_package_abi};

    #[test]
    fn inventories_the_minimal_rust_abi_for_the_test_package() {
        let abi = minimal_test_package_abi();

        let names = abi.iter().map(|item| item.name).collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                "prog_ptr",
                "init",
                "lib_info",
                "lbm_add_extension",
                "lbm_value",
                "lbm_uint",
                "lbm_dec_as_i32",
                "lbm_enc_i",
                "VESC_IF.lbm_enc_sym_eerror",
                "VESC_IF.set_app_data_handler",
                "VESC_IF.send_app_data",
                "VESC_IF.system_time_ticks",
            ]
        );
    }

    #[test]
    fn minimal_package_function_requirements_are_pinned_for_ffi_compare() {
        use crate::ffi_compare::LOOPBACK_USED_SLOTS;

        for requirement in minimal_test_package_abi() {
            let slot = match requirement.kind {
                AbiRequirementKind::Function | AbiRequirementKind::ErrorSymbol => requirement
                    .name
                    .strip_prefix("VESC_IF.")
                    .unwrap_or(requirement.name),
                _ => continue,
            };
            assert!(
                LOOPBACK_USED_SLOTS.contains(&slot),
                "missing pinned slot for {}",
                requirement.name
            );
        }
    }

    #[test]
    fn groups_requirements_by_abi_role() {
        let abi = minimal_test_package_abi();

        assert!(
            abi.iter()
                .any(|item| item.kind == AbiRequirementKind::EntryPoint)
        );
        assert!(
            abi.iter()
                .any(|item| item.kind == AbiRequirementKind::LoaderHeader)
        );
        assert!(abi.iter().any(|item| item.kind == AbiRequirementKind::Type));
        assert!(
            abi.iter()
                .any(|item| item.kind == AbiRequirementKind::Function)
        );
        assert!(
            abi.iter()
                .any(|item| item.kind == AbiRequirementKind::ErrorSymbol)
        );
    }
}
