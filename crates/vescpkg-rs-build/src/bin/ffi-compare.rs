//! Command-line ABI comparison helper for Rust and VESC C interface surfaces.

use std::path::Path;
use std::process::ExitCode;

use vescpkg_rs_build::ffi_compare::{
    ALL_PINNED_USED_SLOTS, LOOPBACK_USED_SLOTS, compare_full_table, compare_used_slots_from_paths,
    default_header_path, default_rust_table_path,
};

fn main() -> ExitCode {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode_used_slots = args.iter().any(|arg| arg == "--used-slots");
    let mode_full = args.iter().any(|arg| arg == "--full");
    let positional: Vec<_> = args
        .into_iter()
        .filter(|arg| arg != "--used-slots" && arg != "--full")
        .collect();

    let c_header = positional
        .first()
        .map(|path| Path::new(path).to_path_buf())
        .unwrap_or_else(|| default_header_path(manifest));
    let rust_source = positional
        .get(1)
        .map(|path| Path::new(path).to_path_buf())
        .unwrap_or_else(|| default_rust_table_path(manifest));

    if mode_full {
        return run_full_compare(&c_header, &rust_source);
    }

    if mode_used_slots {
        return match compare_used_slots_from_paths(&c_header, &rust_source, ALL_PINNED_USED_SLOTS) {
            Ok(()) => {
                println!(
                    "pinned used-slot order matches for {} slot(s)",
                    ALL_PINNED_USED_SLOTS.len()
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("ffi compare failed: {error}");
                ExitCode::from(1)
            }
        };
    }

    match vescpkg_rs_build::ffi_compare::compare_used_slots_from_paths(
        &c_header,
        &rust_source,
        LOOPBACK_USED_SLOTS,
    ) {
        Ok(()) => {
            println!("loopback used-slot order matches");
            ExitCode::SUCCESS
        }
        Err(vescpkg_rs_build::ffi_compare::CompareError::SlotOrderMismatch { .. }) => {
            match vescpkg_rs_build::ffi_compare::slots_present(
                &std::fs::read_to_string(&c_header).expect("header"),
                LOOPBACK_USED_SLOTS,
            ) {
                Ok(()) => {
                    println!(
                        "loopback slots present in {}; full order compare requires --used-slots with a full vesc_c_if.h",
                        c_header.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("ffi compare failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("ffi compare failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_full_compare(c_header: &Path, rust_source: &Path) -> ExitCode {
    let c_source = match std::fs::read_to_string(c_header) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", c_header.display());
            return ExitCode::from(2);
        }
    };
    let rust_source_text = match std::fs::read_to_string(rust_source) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", rust_source.display());
            return ExitCode::from(2);
        }
    };

    let mismatches = compare_full_table(&c_source, &rust_source_text);
    if mismatches.is_empty() {
        println!("full ffi table slot order matches");
        ExitCode::SUCCESS
    } else {
        let count = mismatches.len();
        for (index, c_field, rust_field) in mismatches {
            match rust_field {
                Some(rust_field) => println!(
                    "slot {index:03} offset {:#05x}: C {} (line {}) != Rust {} (line {})",
                    index * 4,
                    c_field.name,
                    c_field.line,
                    rust_field.name,
                    rust_field.line
                ),
                None => println!(
                    "slot {index:03} offset {:#05x}: missing in Rust, C has {} (line {})",
                    index * 4,
                    c_field.name,
                    c_field.line
                ),
            }
        }
        println!("{count} full-table slot mismatch(es)");
        ExitCode::from(1)
    }
}
