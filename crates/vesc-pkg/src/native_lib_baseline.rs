use std::hash::Hasher;
use std::path::PathBuf;

/// Repository-relative files that feed the native-lib baseline audit.
pub const NATIVE_LIB_BASELINE_INPUTS: [&str; 7] = [
    "src/vesc_c_if.h",
    "src/rules.mk",
    "src/link.ld",
    "scripts/conv.py",
    "package/code.lisp",
    "package/pkgdesc.qml",
    "package/README.md",
];

/// Repository-relative native-lib baseline outputs that should be generated.
pub const NATIVE_LIB_BASELINE_OUTPUTS: [&str; 4] = [
    "target/native-lib-baseline/native_lib.elf",
    "target/native-lib-baseline/native_lib.bin",
    "target/native-lib-baseline/package_lib.bin",
    "target/vescpkg/native-lib-baseline/native-lib-baseline.vescpkg",
];

/// Flash block size budget used by VESC package native payload checks.
pub const VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES: u64 = 128 * 1024;
/// Conservative native payload budget reserved within the package flash block.
pub const VESC_PACKAGE_FLASH_BUDGET_BYTES: u64 = VESC_PACKAGE_FLASH_BLOCK_LIMIT_BYTES / 8;
/// Expected fingerprint for the vendored VESC C interface header.
pub const EXPECTED_VESC_C_IF_HEADER_FINGERPRINT: &str = "f0097b82dd4adc19";

/// Package-specific inputs included in native-lib baseline fixture checks.
pub const NATIVE_LIB_BASELINE_PACKAGE_INPUTS: [&str; 3] = [
    "package/code.lisp",
    "package/pkgdesc.qml",
    "package/README.md",
];

/// Baseline fixture root used to resolve native-lib audit inputs and outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLibBaselinePath {
    /// Root directory that contains the baseline fixture tree.
    pub root: PathBuf,
}

impl NativeLibBaselinePath {
    /// Creates a baseline path helper rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Iterates over required baseline input paths.
    pub fn input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        baseline_input_paths().map(|path| self.root.join(path))
    }

    /// Iterates over required package baseline input paths.
    pub fn package_input_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        NATIVE_LIB_BASELINE_PACKAGE_INPUTS
            .iter()
            .map(|path| self.root.join(path))
    }
}

/// Returns the default baseline fixture path helper.

pub fn native_lib_baseline_root() -> NativeLibBaselinePath {
    NativeLibBaselinePath::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/native-lib-baseline"),
    )
}

/// Returns the path to the vendored VESC C interface header.

pub fn vesc_c_if_header_path() -> PathBuf {
    native_lib_baseline_root().root.join("src/vesc_c_if.h")
}

/// Computes the fingerprint of the vendored VESC C interface header.

pub fn vesc_c_if_header_fingerprint() -> String {
    let header = std::fs::read(vesc_c_if_header_path()).expect("vesc_c_if.h contents");
    fingerprint_bytes(&header)
}

/// Computes the stable FNV-1a fingerprint used by fixture pinning checks.

pub fn fingerprint_bytes(bytes: &[u8]) -> String {
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

/// Iterates repository-relative baseline input paths.

pub fn baseline_input_paths() -> impl Iterator<Item = &'static str> {
    NATIVE_LIB_BASELINE_INPUTS.iter().copied()
}

/// Iterates repository-relative baseline output paths.

pub fn baseline_output_paths() -> impl Iterator<Item = &'static str> {
    NATIVE_LIB_BASELINE_OUTPUTS.iter().copied()
}

/// Asserts that required native-lib baseline fixture inputs and outputs are present.

pub fn audit_baseline_fixture_layout() {
    use std::fs;

    assert_eq!(
        baseline_input_paths().collect::<Vec<_>>(),
        NATIVE_LIB_BASELINE_INPUTS
    );
    assert_eq!(
        baseline_output_paths().collect::<Vec<_>>(),
        NATIVE_LIB_BASELINE_OUTPUTS
    );

    let root = native_lib_baseline_root();
    let missing = root
        .input_paths()
        .filter(|path| !path.exists())
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing native-lib baseline files: {missing:?}"
    );

    let inputs = root.input_paths().collect::<Vec<_>>();
    assert!(inputs.iter().any(|path| path.ends_with("src/vesc_c_if.h")));
    assert!(inputs.iter().any(|path| path.ends_with("src/link.ld")));
    assert!(inputs.iter().any(|path| path.ends_with("scripts/conv.py")));
    assert!(
        inputs
            .iter()
            .all(|path| !path.ends_with("src/package_lib.c")),
        "package-specific C shim source must not be a baseline input: {inputs:?}"
    );
    assert!(
        inputs
            .iter()
            .filter(|path| path.extension().is_some_and(|extension| extension == "c"))
            .collect::<Vec<_>>()
            .is_empty(),
        "package-specific C sources must not be native-lib inputs"
    );

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

    let loader = root
        .input_paths()
        .find(|path| path.ends_with("package/code.lisp"))
        .expect("expected code.lisp in the native-lib baseline fixture");
    let source = fs::read_to_string(&loader).expect("code.lisp contents");
    assert!(source.contains("(import \"src/package_lib.bin\" 'package-lib)"));
    assert!(source.contains("(load-native-lib package-lib)"));
    assert!(!source.contains("(loopwhile t") && !source.contains("(sleep 1.0)"));
    assert!(!source.contains("ext-rust-add"));

    let readme = root.root.join("package/README.md");
    let readme_source = fs::read_to_string(&readme).expect("package README");
    assert!(readme_source.contains("BLE loopback test package"));

    let descriptor = fs::read_to_string(root.root.join("package/pkgdesc.qml"))
        .expect("package descriptor contents");
    assert!(descriptor.contains("pkgName: \"Rust BLE loopback test package\""));
    assert!(descriptor.contains("pkgOutput: \"native-lib-baseline.vescpkg\""));
    assert!(!descriptor.contains("packageName"));
    assert!(source.contains("BLE loopback test package"));
}

/// Asserts that the vendored C interface header still matches pinned ABI expectations.

pub fn audit_vesc_c_if_abi_pins() {
    use std::fs;

    assert_eq!(
        vesc_c_if_header_fingerprint(),
        EXPECTED_VESC_C_IF_HEADER_FINGERPRINT,
        "refresh the pinned header fingerprint only after reviewing the ABI diff for fixtures/native-lib-baseline/src/vesc_c_if.h"
    );

    let header = fs::read_to_string(vesc_c_if_header_path()).expect("vesc_c_if.h contents");
    let slots = parse_vesc_c_if_slots(&header);
    let expected = [
        (
            0,
            "lbm_add_extension",
            "load_extension_fptr lbm_add_extension;",
        ),
        (16, "lbm_enc_i", "lbm_value (*lbm_enc_i)(lbm_int x);"),
        (
            25,
            "lbm_dec_as_i32",
            "int32_t (*lbm_dec_as_i32)(lbm_value val);",
        ),
        (31, "lbm_is_number", "bool (*lbm_is_number)(lbm_value x);"),
        (37, "lbm_enc_sym_eerror", "lbm_uint lbm_enc_sym_eerror;"),
        (
            148,
            "send_app_data",
            "void (*send_app_data)(unsigned char *data, unsigned int len);",
        ),
        (
            149,
            "set_app_data_handler",
            "bool (*set_app_data_handler)(app_data_handler_fun handler);",
        ),
        (
            238,
            "system_time_ticks",
            "systime_t (*system_time_ticks)(void);",
        ),
    ]
    .map(|(slot, name, signature)| (slot, name, signature.to_owned()));
    assert_eq!(
        slots, expected,
        "fixture vesc_c_if.h must preserve the upstream vesc_pkg_lib slot order for every Rust-modeled VESC_IF entry"
    );
}

fn parse_vesc_c_if_slots(header: &str) -> Vec<(usize, &'static str, String)> {
    let struct_start = header
        .find("typedef struct {\n    load_extension_fptr lbm_add_extension;")
        .expect("vesc_c_if struct start");
    let struct_end = header[struct_start..]
        .find("} vesc_c_if;")
        .map(|offset| struct_start + offset)
        .expect("vesc_c_if struct end");
    let body = &header[struct_start..struct_end];
    let mut slot = 0usize;
    let mut used = Vec::new();

    for raw_line in body.lines().skip(1) {
        let line = raw_line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let width = if let Some(reserved) = line.strip_prefix("uintptr_t _reserved_after_lbm[") {
            reserved
                .split(']')
                .next()
                .expect("reserved width")
                .parse::<usize>()
                .expect("reserved width integer")
        } else if let Some(reserved) = line.strip_prefix("uintptr_t _reserved_after_app_data[") {
            reserved
                .split(']')
                .next()
                .expect("reserved width")
                .parse::<usize>()
                .expect("reserved width integer")
        } else {
            1
        };

        match line {
            "load_extension_fptr lbm_add_extension;" => {
                used.push((slot, "lbm_add_extension", line.to_owned()));
            }
            "lbm_value (*lbm_enc_i)(lbm_int x);" => {
                used.push((slot, "lbm_enc_i", line.to_owned()));
            }
            "int32_t (*lbm_dec_as_i32)(lbm_value val);" => {
                used.push((slot, "lbm_dec_as_i32", line.to_owned()));
            }
            "bool (*lbm_is_number)(lbm_value x);" => {
                used.push((slot, "lbm_is_number", line.to_owned()));
            }
            "lbm_uint lbm_enc_sym_eerror;" => {
                used.push((slot, "lbm_enc_sym_eerror", line.to_owned()));
            }
            "void (*send_app_data)(unsigned char *data, unsigned int len);" => {
                used.push((slot, "send_app_data", line.to_owned()));
            }
            "bool (*set_app_data_handler)(app_data_handler_fun handler);" => {
                used.push((slot, "set_app_data_handler", line.to_owned()));
            }
            "systime_t (*system_time_ticks)(void);" => {
                used.push((slot, "system_time_ticks", line.to_owned()));
            }
            _ => {}
        }

        slot += width;
    }

    used
}
