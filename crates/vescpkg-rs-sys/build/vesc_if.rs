//! Parse the append-only VESC firmware interface into 32-bit ABI slots.

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SlotDeclaration {
    pub(crate) c_name: String,
    pub(crate) rust_name: String,
    pub(crate) declaration: String,
    pub(crate) index: usize,
    pub(crate) line: usize,
}

pub(crate) fn parse(source: &str) -> Result<Vec<SlotDeclaration>, String> {
    let lines: Vec<_> = source.lines().collect();
    let end = lines
        .iter()
        .position(|line| line.trim().starts_with("} vesc_c_if"))
        .ok_or_else(|| "missing `} vesc_c_if`".to_owned())?;
    let start = lines[..end]
        .iter()
        .rposition(|line| line.trim() == "typedef struct {")
        .ok_or_else(|| "missing `typedef struct {` before vesc_c_if".to_owned())?;

    let mut slots = Vec::new();
    let mut pending = String::new();
    let mut pending_line = None;

    for (line_index, line) in lines[start + 1..end].iter().enumerate() {
        let Some(fragment) = declaration_fragment(line) else {
            continue;
        };

        if pending.is_empty() {
            pending_line = Some(start + line_index + 2);
        } else {
            pending.push(' ');
        }
        pending.push_str(fragment);

        if fragment.contains(';') {
            let (name, width) = parse_declarator(&pending)
                .ok_or_else(|| format!("unsupported declaration: {}", pending.trim()))?;
            let line = pending_line.expect("pending declaration line");

            for element in 0..width {
                slots.push(SlotDeclaration {
                    c_name: if width == 1 {
                        name.clone()
                    } else {
                        format!("{name}[{element}]")
                    },
                    rust_name: if width == 1 {
                        name.clone()
                    } else {
                        format!("{name}_{element}")
                    },
                    declaration: pending.trim().to_owned(),
                    index: slots.len(),
                    line,
                });
            }

            pending.clear();
            pending_line = None;
        }
    }

    if !pending.is_empty() {
        return Err(format!("unterminated declaration: {}", pending.trim()));
    }

    Ok(slots)
}

fn declaration_fragment(line: &str) -> Option<&str> {
    line.split("//")
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("/*"))
        .filter(|line| !line.starts_with('*'))
}

fn parse_declarator(declaration: &str) -> Option<(String, usize)> {
    let declarator =
        c_function_pointer_field(declaration).or_else(|| c_scalar_field(declaration))?;

    if is_c_identifier(&declarator) {
        return Some((declarator, 1));
    }

    let declarator = declarator.strip_suffix(']')?;
    let (name, width) = declarator.rsplit_once('[')?;
    let width = width.parse::<usize>().ok().filter(|width| *width > 0)?;
    is_c_identifier(name).then(|| (name.to_owned(), width))
}

fn c_function_pointer_field(declaration: &str) -> Option<String> {
    declaration.find("(*").and_then(|start| {
        let rest = &declaration[start + 2..];
        rest.find(')').map(|end| rest[..end].trim().to_owned())
    })
}

fn c_scalar_field(declaration: &str) -> Option<String> {
    declaration
        .trim()
        .strip_suffix(';')
        .map(str::trim)
        .and_then(|declaration| declaration.split_whitespace().last())
        .map(|token| token.trim_matches('*').to_owned())
}

fn is_c_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_scalar_function_pointer_and_multiline_declarations() {
        let source = r#"
typedef struct {
    load_extension_fptr lbm_add_extension;
    void (*send_packet)(
        unsigned char *data,
        unsigned int len
    );
} vesc_c_if;
"#;

        let slots = parse(source).expect("valid VESC_IF fixture");

        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].c_name, "lbm_add_extension");
        assert_eq!(
            slots[0].declaration,
            "load_extension_fptr lbm_add_extension;"
        );
        assert_eq!(slots[1].c_name, "send_packet");
        assert_eq!(
            slots[1].declaration,
            "void (*send_packet)( unsigned char *data, unsigned int len );"
        );
        assert_eq!(slots[1].index, 1);
    }

    #[test]
    fn fixed_size_arrays_contribute_named_vesc32_slots() {
        let source = r#"
typedef struct {
    void (*before)(void);
    void *reserved[3];
    void (*after)(void);
} vesc_c_if;
"#;

        let slots = parse(source).expect("valid VESC_IF fixture");

        assert_eq!(slots.len(), 5);
        assert_eq!(slots[1].c_name, "reserved[0]");
        assert_eq!(slots[1].rust_name, "reserved_0");
        assert_eq!(slots.last().map(|slot| slot.index), Some(4));
    }

    #[test]
    fn unsupported_declarations_fail_instead_of_shifting_offsets() {
        let source = r#"
typedef struct {
    void (*before)(void);
    unsigned flags : 1;
    void (*after)(void);
} vesc_c_if;
"#;

        let error = parse(source).expect_err("bitfields are not ABI slots");

        assert!(error.contains("unsigned flags : 1;"));
    }
}
