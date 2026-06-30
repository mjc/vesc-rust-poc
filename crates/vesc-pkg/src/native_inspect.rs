use object::read::File as ObjectFile;
use object::read::archive::ArchiveFile;
use object::read::elf::FileHeader;
use object::{Object, ObjectSection, ObjectSymbol, SectionFlags, SymbolKind};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Simplified section metadata extracted from an ELF file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionLayout {
    /// Section name as reported by the ELF inspector.
    pub name: String,
    /// Section size in bytes.
    pub size: usize,
    /// Section virtual memory address.
    pub vma: usize,
}

fn parse_elf_bytes<'data>(bytes: &'data [u8], path: &Path) -> ObjectFile<'data> {
    ObjectFile::parse(bytes).unwrap_or_else(|error| panic!("parse ELF {path:?}: {error}"))
}

fn read_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("read object file {path:?}: {error}"))
}

fn push_nm_lines<'data>(lines: &mut Vec<String>, object: ObjectFile<'data>) {
    for symbol in object.symbols() {
        let Ok(name) = symbol.name() else {
            continue;
        };
        if name.is_empty() {
            continue;
        }

        if symbol.is_undefined() {
            lines.push(format!("         U {name}"));
            continue;
        }

        let kind = match symbol.kind() {
            SymbolKind::Text => 'T',
            SymbolKind::Data => 'D',
            _ => 't',
        };
        lines.push(format!("{:08x} {kind} {name}", symbol.address()));
    }
}

/// Runs `nm` for `path` and returns its stdout.
pub fn nm_output(path: &Path) -> String {
    let bytes = read_bytes(path);
    let mut lines = Vec::new();

    if bytes.starts_with(b"!<arch>\n") {
        let archive = ArchiveFile::parse(&bytes[..])
            .unwrap_or_else(|error| panic!("parse archive {path:?}: {error}"));
        for member in archive.members() {
            let member =
                member.unwrap_or_else(|error| panic!("archive member in {path:?}: {error}"));
            let data = member
                .data(&bytes[..])
                .unwrap_or_else(|error| panic!("archive member data in {path:?}: {error}"));
            if let Ok(object) = ObjectFile::parse(data) {
                push_nm_lines(&mut lines, object);
            }
        }
    } else {
        let object = parse_elf_bytes(&bytes, path);
        push_nm_lines(&mut lines, object);
    }

    lines.join("\n")
}

/// Returns all loadable section layouts keyed by section name.
pub fn all_section_layouts(elf: &Path) -> BTreeMap<String, SectionLayout> {
    let bytes = read_bytes(elf);
    let object = parse_elf_bytes(&bytes, elf);
    object
        .sections()
        .filter_map(|section| {
            let name = section.name().ok()?;
            if !name.starts_with('.') {
                return None;
            }
            Some((
                name.to_owned(),
                SectionLayout {
                    name: name.to_owned(),
                    size: section.size() as usize,
                    vma: section.address() as usize,
                },
            ))
        })
        .collect()
}

/// Returns whether the ELF header marks `elf` executable.
pub fn elf_is_executable(elf: &Path) -> bool {
    let bytes = read_bytes(elf);
    let object = parse_elf_bytes(&bytes, elf);
    match object {
        ObjectFile::Elf32(file) => {
            let endian = file.endian();
            file.elf_header().e_type(endian) == object::elf::ET_EXEC
        }
        ObjectFile::Elf64(file) => {
            let endian = file.endian();
            file.elf_header().e_type(endian) == object::elf::ET_EXEC
        }
        _ => false,
    }
}

/// Flattens loadable ELF sections into the package binary image.
pub fn elf_to_flat_binary(elf: &Path) -> Vec<u8> {
    let bytes = read_bytes(elf);
    let object = parse_elf_bytes(&bytes, elf);
    let mut max_end = 0usize;

    for section in object.sections() {
        let Ok(name) = section.name() else {
            continue;
        };
        if !name.starts_with('.') {
            continue;
        }
        if !section_is_loadable(section.flags()) {
            continue;
        }
        let start = section.address() as usize;
        let end = start.saturating_add(section.size() as usize);
        max_end = max_end.max(end);
    }

    let mut blob = vec![0u8; max_end];
    for section in object.sections() {
        let Ok(name) = section.name() else {
            continue;
        };
        if !name.starts_with('.') {
            continue;
        }
        if !section_is_loadable(section.flags()) {
            continue;
        }
        let Ok(data) = section.data() else {
            continue;
        };
        let start = section.address() as usize;
        let end = start.saturating_add(data.len());
        if end > blob.len() {
            continue;
        }
        blob[start..end].copy_from_slice(data);
    }

    blob
}

fn section_is_loadable(flags: SectionFlags) -> bool {
    match flags {
        SectionFlags::Elf { sh_flags } => sh_flags & object::elf::SHF_ALLOC as u64 != 0,
        _ => false,
    }
}

/// Returns whether the ELF has no relocation sections.
pub fn elf_has_no_relocations(elf: &Path) -> bool {
    let bytes = read_bytes(elf);
    let object = parse_elf_bytes(&bytes, elf);
    !object
        .sections()
        .any(|section| section.relocations().next().is_some())
}

/// Returns the named section layout or panics with context if it is missing.
pub fn section_from<'a>(
    sections: &'a BTreeMap<String, SectionLayout>,
    section_name: &str,
) -> &'a SectionLayout {
    sections
        .get(section_name)
        .unwrap_or_else(|| panic!("section {section_name} not found in native-lib ELF headers"))
}

#[cfg(test)]
mod tests {
    use super::SectionLayout;

    fn parse_section_layout(line: &str) -> Option<SectionLayout> {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        let [_, name, size, vma, ..] = parts.as_slice() else {
            return None;
        };
        if !name.starts_with('.') {
            return None;
        }

        Some(SectionLayout {
            name: (*name).to_owned(),
            size: usize::from_str_radix(size, 16).ok()?,
            vma: usize::from_str_radix(vma, 16).ok()?,
        })
    }

    #[test]
    fn parse_section_layout_reads_objdump_header_lines() {
        let line = "  3 .text         00000120  08000000  08000000  00010000  2**4";
        assert_eq!(
            parse_section_layout(line),
            Some(SectionLayout {
                name: ".text".to_owned(),
                size: 0x120,
                vma: 0x0800_0000,
            })
        );
    }
}
