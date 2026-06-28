use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SectionLayout {
    pub name: String,
    pub size: usize,
    pub vma: usize,
}

pub fn nm_output(path: &Path) -> String {
    let output = Command::new("arm-none-eabi-nm")
        .arg(path)
        .output()
        .expect("arm-none-eabi-nm execution");

    assert!(
        output.status.success(),
        "arm-none-eabi-nm failed for {path:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("nm output to be valid UTF-8")
}

pub fn command_stdout(program: &str, args: impl IntoIterator<Item = impl AsRef<Path>>) -> String {
    let output = Command::new(program)
        .args(args.into_iter().map(|arg| arg.as_ref().to_owned()))
        .output()
        .unwrap_or_else(|error| panic!("{program} execution failed: {error}"));

    assert!(
        output.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("command stdout to be valid UTF-8")
}

pub fn all_section_layouts(elf: &Path) -> BTreeMap<String, SectionLayout> {
    let sections = command_stdout(
        "arm-none-eabi-objdump",
        [PathBuf::from("-h"), elf.to_path_buf()],
    );
    sections
        .lines()
        .filter_map(parse_section_layout)
        .map(|section| (section.name.clone(), section))
        .collect()
}

pub fn section_from<'a>(
    sections: &'a BTreeMap<String, SectionLayout>,
    section_name: &str,
) -> &'a SectionLayout {
    sections
        .get(section_name)
        .unwrap_or_else(|| panic!("section {section_name} not found in native-lib ELF headers"))
}

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

#[cfg(test)]
mod tests {
    use super::parse_section_layout;

    #[test]
    fn parse_section_layout_reads_objdump_header_lines() {
        let section =
            parse_section_layout("  3 .text        00000080  00000010  00000010  00000010  2**4")
                .expect("section");

        assert_eq!(section.name, ".text");
        assert_eq!(section.size, 0x80);
        assert_eq!(section.vma, 0x10);
    }
}
