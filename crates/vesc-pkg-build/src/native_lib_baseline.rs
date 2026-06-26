use std::hash::Hasher;
use std::path::PathBuf;

pub const NATIVE_LIB_BASELINE_INPUTS: [&str; 8] = [
    "src/package_lib.c",
    "src/vesc_c_if.h",
    "src/rules.mk",
    "src/link.ld",
    "scripts/conv.py",
    "package/code.lisp",
    "package/pkgdesc.qml",
    "package/README.md",
];

pub const NATIVE_LIB_BASELINE_OUTPUTS: [&str; 4] = [
    "target/native-lib-baseline/native_lib.elf",
    "target/native-lib-baseline/native_lib.bin",
    "target/native-lib-baseline/package_lib.bin",
    "target/vescpkg/native-lib-baseline/native-lib-baseline.vescpkg",
];

pub const VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES: u64 = 128 * 1024;
pub const VESC_PACKAGE_FLASH_BUDGET_BYTES: u64 = VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES / 8;
pub const EXPECTED_VESC_C_IF_HEADER_FINGERPRINT: &str = "a8980de23614d274";

pub const NATIVE_LIB_BASELINE_PACKAGE_INPUTS: [&str; 3] = [
    "package/code.lisp",
    "package/pkgdesc.qml",
    "package/README.md",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibBaselinePath {
    root: PathBuf,
}

impl NativeLibBaselinePath {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        NATIVE_LIB_BASELINE_INPUTS
            .iter()
            .map(move |relative| self.root.join(relative))
    }

    pub fn package_input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        NATIVE_LIB_BASELINE_PACKAGE_INPUTS
            .iter()
            .map(move |relative| self.root.join(relative))
    }
}

pub fn native_lib_baseline_root() -> NativeLibBaselinePath {
    NativeLibBaselinePath::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/native-lib-baseline"),
    )
}

pub fn vesc_c_if_header_path() -> PathBuf {
    native_lib_baseline_root().root.join("src/vesc_c_if.h")
}

pub fn vesc_c_if_header_fingerprint() -> String {
    let header = std::fs::read(vesc_c_if_header_path()).expect("vesc_c_if.h contents");
    fingerprint_bytes(&header)
}

fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hasher = Fnv1a64::default();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

#[derive(Default)]
struct Fnv1a64(u64);

impl Hasher for Fnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };

        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }

        self.0 = hash;
    }
}

pub fn baseline_input_paths() -> impl Iterator<Item = &'static str> {
    NATIVE_LIB_BASELINE_INPUTS.iter().copied()
}

pub fn baseline_output_paths() -> impl Iterator<Item = &'static str> {
    NATIVE_LIB_BASELINE_OUTPUTS.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::{
        baseline_input_paths, baseline_output_paths, native_lib_baseline_root,
        vesc_c_if_header_fingerprint, EXPECTED_VESC_C_IF_HEADER_FINGERPRINT,
        NATIVE_LIB_BASELINE_INPUTS, NATIVE_LIB_BASELINE_OUTPUTS, VESC_PACKAGE_FLASH_BUDGET_BYTES,
    };
    use std::fs;

    #[test]
    fn lists_expected_baseline_inputs() {
        assert_eq!(
            baseline_input_paths().collect::<Vec<_>>(),
            NATIVE_LIB_BASELINE_INPUTS
        );
    }

    #[test]
    fn lists_expected_baseline_outputs() {
        assert_eq!(
            baseline_output_paths().collect::<Vec<_>>(),
            NATIVE_LIB_BASELINE_OUTPUTS
        );
    }

    #[test]
    fn fixture_contains_the_expected_input_layout() {
        let root = native_lib_baseline_root();

        let missing = root
            .input_paths()
            .filter(|path| !path.exists())
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "missing native-lib baseline files: {missing:?}"
        );
    }

    #[test]
    fn package_payload_stays_well_below_the_vesc_tool_flash_block_limit() {
        let root = native_lib_baseline_root();
        let sizes = root
            .package_input_paths()
            .map(|path| {
                let size = fs::metadata(&path).expect("package input metadata").len();
                (path, size)
            })
            .collect::<Vec<_>>();

        let total_size = sizes.iter().map(|(_, size)| *size).sum::<u64>();
        let biggest = sizes.iter().map(|(_, size)| *size).max().unwrap_or(0);

        assert!(
            biggest < VESC_PACKAGE_FLASH_BUDGET_BYTES,
            "largest package input reaches the package budget: {sizes:?}"
        );
        assert!(
            total_size < VESC_PACKAGE_FLASH_BUDGET_BYTES,
            "package payload is too large for the package budget: {sizes:?}"
        );
    }

    #[test]
    fn vesc_c_if_header_fingerprint_is_pinned() {
        assert_eq!(
            vesc_c_if_header_fingerprint(),
            EXPECTED_VESC_C_IF_HEADER_FINGERPRINT,
            "refresh the pinned header fingerprint only after reviewing the ABI diff for fixtures/native-lib-baseline/src/vesc_c_if.h"
        );
    }

    #[test]
    fn package_lib_c_registers_the_rust_extension_shim() {
        let root = native_lib_baseline_root();
        let package_lib = root
            .input_paths()
            .find(|path| path.ends_with("src/package_lib.c"))
            .expect("expected package_lib.c in the native-lib baseline fixture");
        let source = fs::read_to_string(&package_lib).expect("package_lib.c contents");

        assert!(
            source.contains("INIT_FUN(package_lib_init)"),
            "expected an init hook in the C shim: {package_lib:?}"
        );
        assert!(
            source.contains("ext-rust-add"),
            "expected the shim to register the Rust extension name: {package_lib:?}"
        );
        assert!(
            source.contains("rust_add"),
            "expected the shim to delegate to the Rust function: {package_lib:?}"
        );
        assert!(
            source.contains("argn != 2"),
            "expected the shim to guard against bad arity: {package_lib:?}"
        );
        assert!(
            source.contains("ENC_SYM_EERROR"),
            "expected the shim to return the LispBM arity error: {package_lib:?}"
        );
        assert!(
            source.contains("lbm_dec_as_i32"),
            "expected the shim to decode LispBM integers: {package_lib:?}"
        );
        assert!(
            source.contains("lbm_enc_i"),
            "expected the shim to encode the Rust result back to LispBM: {package_lib:?}"
        );
    }

    #[test]
    fn package_loader_only_loads_the_native_library_for_ble_loopback() {
        let root = native_lib_baseline_root();
        let loader = root
            .input_paths()
            .find(|path| path.ends_with("package/code.lisp"))
            .expect("expected code.lisp in the native-lib baseline fixture");
        let source = fs::read_to_string(&loader).expect("code.lisp contents");

        assert!(
            source.contains("(import \"src/package_lib.bin\" 'package-lib)"),
            "expected the loader to import the native library: {loader:?}"
        );
        assert!(
            source.contains("(load-native-lib package-lib)"),
            "expected the loader to load the imported native library: {loader:?}"
        );
        assert!(
            !source.contains("ext-rust-add"),
            "expected the BLE loopback loader to avoid the old proof extension: {loader:?}"
        );
    }

    #[test]
    fn fixture_package_identity_mentions_the_ble_loopback_test_package() {
        let root = native_lib_baseline_root();
        let readme = fs::read_to_string(root.root.join("package/README.md"))
            .expect("package README contents");
        let descriptor = fs::read_to_string(root.root.join("package/pkgdesc.qml"))
            .expect("package descriptor contents");
        let loader = fs::read_to_string(root.root.join("package/code.lisp"))
            .expect("package loader contents");

        assert!(
            readme.contains("BLE loopback test package"),
            "expected the package README to describe the BLE loopback test package"
        );
        assert!(
            descriptor.contains("Rust BLE loopback test package"),
            "expected the package descriptor to name the BLE loopback test package"
        );
        assert!(
            loader.contains("BLE loopback test package"),
            "expected the package loader comment to name the BLE loopback test package"
        );
    }
}
