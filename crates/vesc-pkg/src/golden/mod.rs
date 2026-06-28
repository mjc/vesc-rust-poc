//! Golden fixture helpers for host-side packaging tests.
//!
//! Tests embed fixture bytes with `include_bytes!` and never rebuild native code.
//! Refresh fixtures with `cargo run -p vesc-pkg --bin write-golden-fixtures`.

pub mod fixtures;
pub mod pack;

pub use fixtures::{
    FINGERPRINTS_TOML, LISP_DATA, NATIVE_LIB_BIN, NATIVE_LIB_ELF, PACKAGE_LIB, VERSION,
    fixture_dir, payload_contains_probe_extension, probe_extension_name,
};
pub use pack::pack_lisp_data;
