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

pub const MINIMAL_TEST_PACKAGE_ABI: [AbiRequirement; 10] = [
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
        "ENC_SYM_EERROR",
        AbiRequirementKind::ErrorSymbol,
        "C shim returns a LispBM error on bad arity",
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
                "ENC_SYM_EERROR",
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
