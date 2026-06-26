#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiRequirementKind {
    EntryPoint,
    LoaderHeader,
    Type,
    Function,
    ErrorSymbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiRequirement {
    pub name: &'static str,
    pub kind: AbiRequirementKind,
    pub caller: &'static str,
}

impl AbiRequirement {
    pub const fn new(name: &'static str, kind: AbiRequirementKind, caller: &'static str) -> Self {
        Self { name, kind, caller }
    }
}

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

pub fn minimal_test_package_abi() -> &'static [AbiRequirement] {
    &MINIMAL_TEST_PACKAGE_ABI
}

#[cfg(test)]
mod tests {
    use super::{minimal_test_package_abi, AbiRequirementKind};

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
    fn groups_requirements_by_abi_role() {
        let abi = minimal_test_package_abi();

        assert!(abi
            .iter()
            .any(|item| item.kind == AbiRequirementKind::EntryPoint));
        assert!(abi
            .iter()
            .any(|item| item.kind == AbiRequirementKind::LoaderHeader));
        assert!(abi.iter().any(|item| item.kind == AbiRequirementKind::Type));
        assert!(abi
            .iter()
            .any(|item| item.kind == AbiRequirementKind::Function));
        assert!(abi
            .iter()
            .any(|item| item.kind == AbiRequirementKind::ErrorSymbol));
    }
}
