#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiRequirementKind {
    EntryPoint,
    Macro,
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

pub const MINIMAL_TEST_PACKAGE_ABI: [AbiRequirement; 14] = [
    AbiRequirement::new(
        "INIT_FUN",
        AbiRequirementKind::EntryPoint,
        "package loader calls the C shim entry point",
    ),
    AbiRequirement::new(
        "INIT_START",
        AbiRequirementKind::Macro,
        "C shim wraps registration setup",
    ),
    AbiRequirement::new(
        "INIT_END",
        AbiRequirementKind::Macro,
        "C shim wraps registration teardown",
    ),
    AbiRequirement::new(
        "lib_info",
        AbiRequirementKind::Type,
        "entrypoint receives package metadata",
    ),
    AbiRequirement::new(
        "lbm_add_extension",
        AbiRequirementKind::Function,
        "C shim registers the LispBM extension",
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
        "C shim decodes LispBM integer arguments",
    ),
    AbiRequirement::new(
        "lbm_enc_i",
        AbiRequirementKind::Function,
        "C shim encodes the integer result",
    ),
    AbiRequirement::new(
        "VESC_IF.lbm_is_number",
        AbiRequirementKind::Function,
        "Rust package validates LispBM extension arguments before decoding",
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
    fn inventories_the_minimal_c_abi_for_the_test_package() {
        let abi = minimal_test_package_abi();

        let names = abi.iter().map(|item| item.name).collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                "INIT_FUN",
                "INIT_START",
                "INIT_END",
                "lib_info",
                "lbm_add_extension",
                "lbm_value",
                "lbm_uint",
                "lbm_dec_as_i32",
                "lbm_enc_i",
                "VESC_IF.lbm_is_number",
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
            .any(|item| item.kind == AbiRequirementKind::Macro));
        assert!(abi.iter().any(|item| item.kind == AbiRequirementKind::Type));
        assert!(abi
            .iter()
            .any(|item| item.kind == AbiRequirementKind::Function));
        assert!(abi
            .iter()
            .any(|item| item.kind == AbiRequirementKind::ErrorSymbol));
    }
}
