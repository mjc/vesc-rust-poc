use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use proc_macro2::{LineColumn, Span};
use syn::{
    Arm, Attribute, Expr, Field, FieldValue, ForeignItem, ImplItem, Item, Local, Meta, StmtMacro,
    Token, TraitItem, Variant,
    parse::Parser,
    punctuated::Punctuated,
    spanned::Spanned,
    visit::{self, Visit},
};

#[derive(Clone, Copy, Eq, PartialEq)]
enum Truth {
    False,
    Unknown,
    True,
}

impl Truth {
    fn not(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
            Self::True => Self::False,
        }
    }
}

#[derive(Clone, Copy)]
struct Removal {
    start: LineColumn,
    end: LineColumn,
}

#[derive(Default)]
struct TestOnlyVisitor {
    removals: Vec<Removal>,
}

impl TestOnlyVisitor {
    fn remove(&mut self, attrs: &[Attribute], span: Span) -> bool {
        if attrs.iter().any(cfg_is_false_without_tests) {
            let start = attrs
                .first()
                .map_or_else(|| span.start(), |attr| attr.span().start());
            self.removals.push(Removal {
                start,
                end: span.end(),
            });
            true
        } else {
            false
        }
    }
}

macro_rules! enum_attrs {
    ($value:expr, $enum:ident, $($variant:ident),+ $(,)?) => {
        match $value {
            $($enum::$variant(node) => node.attrs.as_slice(),)+
            _ => &[],
        }
    };
}

impl<'ast> Visit<'ast> for TestOnlyVisitor {
    fn visit_item(&mut self, node: &'ast Item) {
        let attrs = enum_attrs!(
            node,
            Item,
            Const,
            Enum,
            ExternCrate,
            Fn,
            ForeignMod,
            Impl,
            Macro,
            Mod,
            Static,
            Struct,
            Trait,
            TraitAlias,
            Type,
            Union,
            Use,
        );
        if !self.remove(attrs, node.span()) {
            visit::visit_item(self, node);
        }
    }

    fn visit_impl_item(&mut self, node: &'ast ImplItem) {
        let attrs = enum_attrs!(node, ImplItem, Const, Fn, Macro, Type);
        if !self.remove(attrs, node.span()) {
            visit::visit_impl_item(self, node);
        }
    }

    fn visit_trait_item(&mut self, node: &'ast TraitItem) {
        let attrs = enum_attrs!(node, TraitItem, Const, Fn, Macro, Type);
        if !self.remove(attrs, node.span()) {
            visit::visit_trait_item(self, node);
        }
    }

    fn visit_foreign_item(&mut self, node: &'ast ForeignItem) {
        let attrs = enum_attrs!(node, ForeignItem, Fn, Macro, Static, Type);
        if !self.remove(attrs, node.span()) {
            visit::visit_foreign_item(self, node);
        }
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        let attrs = enum_attrs!(
            node, Expr, Array, Assign, Async, Await, Binary, Block, Break, Call, Cast, Closure,
            Const, Continue, Field, ForLoop, Group, If, Index, Infer, Let, Lit, Loop, Macro, Match,
            MethodCall, Paren, Path, Range, RawAddr, Reference, Repeat, Return, Struct, Try,
            TryBlock, Tuple, Unary, Unsafe, While, Yield,
        );
        if !self.remove(attrs, node.span()) {
            visit::visit_expr(self, node);
        }
    }

    fn visit_arm(&mut self, node: &'ast Arm) {
        if !self.remove(&node.attrs, node.span()) {
            visit::visit_arm(self, node);
        }
    }

    fn visit_field(&mut self, node: &'ast Field) {
        if !self.remove(&node.attrs, node.span()) {
            visit::visit_field(self, node);
        }
    }

    fn visit_field_value(&mut self, node: &'ast FieldValue) {
        if !self.remove(&node.attrs, node.span()) {
            visit::visit_field_value(self, node);
        }
    }

    fn visit_local(&mut self, node: &'ast Local) {
        if !self.remove(&node.attrs, node.span()) {
            visit::visit_local(self, node);
        }
    }

    fn visit_stmt_macro(&mut self, node: &'ast StmtMacro) {
        if !self.remove(&node.attrs, node.span()) {
            visit::visit_stmt_macro(self, node);
        }
    }

    fn visit_variant(&mut self, node: &'ast Variant) {
        if !self.remove(&node.attrs, node.span()) {
            visit::visit_variant(self, node);
        }
    }
}

fn cfg_is_false_without_tests(attr: &Attribute) -> bool {
    let Meta::List(cfg) = &attr.meta else {
        return false;
    };
    if !cfg.path.is_ident("cfg") {
        return false;
    }
    syn::parse2::<Meta>(cfg.tokens.clone()).is_ok_and(|meta| evaluate(&meta) == Truth::False)
}

fn evaluate(meta: &Meta) -> Truth {
    match meta {
        Meta::Path(path) if path.is_ident("test") => Truth::False,
        Meta::NameValue(value) if value.path.is_ident("feature") => {
            let Expr::Lit(literal) = &value.value else {
                return Truth::Unknown;
            };
            match &literal.lit {
                syn::Lit::Str(feature) if feature.value() == "test-support" => Truth::False,
                _ => Truth::Unknown,
            }
        }
        Meta::NameValue(value) if value.path.is_ident("target_arch") => {
            let Expr::Lit(literal) = &value.value else {
                return Truth::Unknown;
            };
            match &literal.lit {
                syn::Lit::Str(arch) if arch.value() == "arm" => Truth::True,
                syn::Lit::Str(_) => Truth::False,
                _ => Truth::Unknown,
            }
        }
        Meta::Path(_) | Meta::NameValue(_) => Truth::Unknown,
        Meta::List(list) => {
            let Ok(items) =
                Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())
            else {
                return Truth::Unknown;
            };
            if list.path.is_ident("all") {
                items.iter().map(evaluate).fold(Truth::True, and)
            } else if list.path.is_ident("any") {
                items.iter().map(evaluate).fold(Truth::False, or)
            } else if list.path.is_ident("not") && items.len() == 1 {
                evaluate(&items[0]).not()
            } else {
                Truth::Unknown
            }
        }
    }
}

fn and(left: Truth, right: Truth) -> Truth {
    match (left, right) {
        (Truth::False, _) | (_, Truth::False) => Truth::False,
        (Truth::True, Truth::True) => Truth::True,
        _ => Truth::Unknown,
    }
}

fn or(left: Truth, right: Truth) -> Truth {
    match (left, right) {
        (Truth::True, _) | (_, Truth::True) => Truth::True,
        (Truth::False, Truth::False) => Truth::False,
        _ => Truth::Unknown,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input = PathBuf::from(args.next().ok_or("missing input directory")?);
    let output = PathBuf::from(args.next().ok_or("missing output directory")?);
    if args.next().is_some() {
        return Err("usage: production-rust INPUT OUTPUT".into());
    }
    copy_production_rust(&input, &output, &input)?;
    Ok(())
}

fn copy_production_rust(input: &Path, output: &Path, root: &Path) -> io::Result<()> {
    for entry in fs::read_dir(input)? {
        let path = entry?.path();
        let relative = path
            .strip_prefix(root)
            .expect("walked path stays under root");
        if path.is_dir() {
            if path.file_name().is_none_or(|name| !is_test_dir(name)) {
                copy_production_rust(&path, output, root)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && !path.file_name().is_some_and(is_test_file)
        {
            let destination = output.join(relative);
            fs::create_dir_all(destination.parent().expect("source file has parent"))?;
            fs::write(destination, without_tests(&fs::read_to_string(path)?))?;
        }
    }
    Ok(())
}

fn is_test_dir(name: &std::ffi::OsStr) -> bool {
    matches!(name.to_str(), Some("test" | "tests" | "test_support"))
}

fn is_test_file(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        matches!(name, "test.rs" | "tests.rs" | "test_support.rs")
            || name.ends_with("_test.rs")
            || name.ends_with("_tests.rs")
    })
}

fn without_tests(source: &str) -> String {
    let syntax = syn::parse_file(source).expect("Rust source must parse");
    let mut visitor = TestOnlyVisitor::default();
    visitor.visit_file(&syntax);

    let starts = line_starts(source);
    let mut removals = visitor
        .removals
        .into_iter()
        .map(|removal| (offset(&starts, removal.start), offset(&starts, removal.end)))
        .collect::<Vec<_>>();
    removals.sort_unstable();

    let mut bytes = source.as_bytes().to_vec();
    for (start, mut end) in removals.into_iter().rev() {
        while end < bytes.len() && bytes[end].is_ascii_whitespace() && bytes[end] != b'\n' {
            end += 1;
        }
        if end < bytes.len() && matches!(bytes[end], b',' | b';') {
            end += 1;
        }
        for byte in &mut bytes[start..end] {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(bytes).expect("input was UTF-8")
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

fn offset(starts: &[usize], location: LineColumn) -> usize {
    starts[location.line - 1] + location.column
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::{is_test_dir, is_test_file, without_tests};

    #[test]
    fn recognizes_separate_test_modules() {
        for name in [
            "test.rs",
            "tests.rs",
            "test_support.rs",
            "wire_test.rs",
            "wire_tests.rs",
        ] {
            assert!(is_test_file(OsStr::new(name)));
        }
        assert!(!is_test_file(OsStr::new("latest.rs")));
        assert!(is_test_dir(OsStr::new("tests")));
        assert!(is_test_dir(OsStr::new("test_support")));
        assert!(!is_test_dir(OsStr::new("handtest")));
    }

    #[test]
    fn removes_only_test_only_syntax() {
        let source = r#"
#[cfg(test)]
fn test_helper() {
    unreachable!();
}

#[cfg(any(test, target_arch = "arm"))]
fn firmware_and_test() {}

#[cfg(target_arch = "arm")]
fn firmware_only() {}

#[cfg(not(target_arch = "arm"))]
fn host_only() {}

#[cfg(any(test, feature = "test-support"))]
mod support {}

fn runtime() {
    #[cfg(test)]
    assert!(false);
    #[cfg(not(test))]
    run();
}
"#;
        let filtered = without_tests(source);
        assert!(!filtered.contains("test_helper"));
        assert!(filtered.contains("firmware_and_test"));
        assert!(filtered.contains("firmware_only"));
        assert!(!filtered.contains("host_only"));
        assert!(!filtered.contains("mod support"));
        assert!(!filtered.contains("assert!(false)"));
        assert!(filtered.contains("run();"));
        assert_eq!(filtered.lines().count(), source.lines().count());
    }
}
