use std::{env, fs, path::PathBuf, process::ExitCode};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    name: String,
    line: usize,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let c_header = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/mjc/projects/refloat/vesc_pkg_lib/vesc_c_if.h"));
    let rust_source = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("/Users/mjc/projects/vesc-rust-poc/crates/vesc-ffi/src/lib.rs")
    });

    let c_source = match fs::read_to_string(&c_header) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", c_header.display());
            return ExitCode::from(2);
        }
    };
    let rust_source_text = match fs::read_to_string(&rust_source) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", rust_source.display());
            return ExitCode::from(2);
        }
    };

    let c_fields = parse_refloat_vesc_if_fields(&c_source);
    let rust_fields = parse_rust_vesc_if_fields(&rust_source_text);

    println!("refloat vesc_c_if slots: {}", c_fields.len());
    println!("rust raw::VescIf slots: {}", rust_fields.len());

    let mut mismatches = 0;
    let max_len = c_fields.len().max(rust_fields.len());
    for index in 0..max_len {
        match (c_fields.get(index), rust_fields.get(index)) {
            (Some(c), Some(rust)) if c.name == rust.name => {}
            (Some(c), Some(rust)) => {
                mismatches += 1;
                println!(
                    "slot {index:03} offset {:#05x}: C {} (line {}) != Rust {} (line {})",
                    index * 4,
                    c.name,
                    c.line,
                    rust.name,
                    rust.line
                );
            }
            (Some(c), None) => {
                mismatches += 1;
                println!(
                    "slot {index:03} offset {:#05x}: missing in Rust, C has {} (line {})",
                    index * 4,
                    c.name,
                    c.line
                );
            }
            (None, Some(rust)) => {
                mismatches += 1;
                println!(
                    "slot {index:03} offset {:#05x}: extra in Rust {} (line {})",
                    index * 4,
                    rust.name,
                    rust.line
                );
            }
            (None, None) => {}
        }
    }

    for used in [
        "lbm_add_extension",
        "lbm_enc_i",
        "lbm_dec_as_i32",
        "lbm_is_number",
        "lbm_enc_sym_eerror",
        "send_app_data",
        "set_app_data_handler",
        "system_time_ticks",
    ] {
        let c_index = c_fields.iter().position(|field| field.name == used);
        let rust_index = rust_fields.iter().position(|field| field.name == used);
        println!(
            "used slot {used}: C {:?}, Rust {:?}",
            c_index.map(|index| index * 4),
            rust_index.map(|index| index * 4)
        );
    }

    if mismatches == 0 {
        println!("ffi table slot order matches refloat");
        ExitCode::SUCCESS
    } else {
        println!("{mismatches} ffi table slot mismatch(es)");
        ExitCode::from(1)
    }
}

fn parse_refloat_vesc_if_fields(source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut seen_comment = false;
    let mut in_table = false;
    let mut pending_decl = String::new();
    let mut pending_line = 0;

    for (line_index, line) in source.lines().enumerate() {
        if line.contains("Function pointer struct") {
            seen_comment = true;
            continue;
        }
        if seen_comment && line.trim() == "typedef struct {" {
            in_table = true;
            continue;
        }
        if in_table && line.contains("} vesc_c_if;") {
            break;
        }
        if !in_table {
            continue;
        }

        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with("/*") || line.starts_with('*') {
            continue;
        }
        if pending_decl.is_empty() {
            pending_line = line_index + 1;
        }
        pending_decl.push(' ');
        pending_decl.push_str(line);

        if !line.contains(';') {
            continue;
        }

        if let Some(name) = parse_c_field_name(&pending_decl) {
            fields.push(Field {
                name,
                line: pending_line,
            });
        }
        pending_decl.clear();
    }

    fields
}

fn parse_c_field_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(start) = line.find("(*") {
        let rest = &line[start + 2..];
        let end = rest.find(')')?;
        let name = rest[..end].trim();
        return (!name.is_empty()).then(|| name.to_owned());
    }

    if !line.ends_with(';') {
        return None;
    }

    let without_semicolon = line.trim_end_matches(';').trim();
    let name = without_semicolon
        .split_whitespace()
        .last()?
        .trim_matches('*')
        .trim();

    (!name.is_empty()).then(|| name.to_owned())
}

fn parse_rust_vesc_if_fields(source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut in_table = false;

    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "pub struct VescIf {" {
            in_table = true;
            continue;
        }
        if in_table && trimmed == "}" {
            break;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            fields.push(Field {
                name: name.to_owned(),
                line: line_index + 1,
            });
        }
    }

    fields
}
