use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use crate::package_conversion::{PackageBinaryConversionCommand, PackageBinaryConversionRunner};
use crate::{PackageLayout, BLE_LOOPBACK_PACKAGE_NAME};

const LOOPBACK_PACKAGE_VERSION: &str = "0.1.0";

pub struct TempWorkspace {
    _temp: tempfile::TempDir,
    pub root: PathBuf,
}

impl TempWorkspace {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        Self { _temp: temp, root }
    }
}

pub struct PackageTestHarness {
    workspace: TempWorkspace,
}

impl PackageTestHarness {
    pub fn new() -> Self {
        Self {
            workspace: TempWorkspace::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.workspace.root
    }

    pub fn write_bytes(self, relative: &str, contents: impl AsRef<[u8]>) -> Self {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("artifact parent directory");
        }
        fs::write(path, contents.as_ref()).expect("artifact contents");
        self
    }

    pub fn write_text(self, relative: &str, contents: &str) -> Self {
        self.write_bytes(relative, contents.as_bytes())
    }

    pub fn write_native_payload(self, payload: impl AsRef<[u8]>) -> Self {
        self.write_bytes("src/package_lib.bin", payload)
    }

    pub fn loopback_layout(&self) -> PackageLayout {
        PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, LOOPBACK_PACKAGE_VERSION)
    }

    pub fn loopback_staging_dir(&self) -> PathBuf {
        self.root().join(self.loopback_layout().staging_dir())
    }

    pub fn ensure_loopback_staging(self) -> Self {
        fs::create_dir_all(self.loopback_staging_dir()).expect("loopback staging dir");
        self
    }

    pub fn loopback_loader_lisp(&self) -> String {
        "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n".to_owned()
    }

    pub fn loopback_loader_lisp_import_only(&self) -> String {
        "(import \"src/package_lib.bin\" 'package-lib)\n".to_owned()
    }

    pub fn write_loopback_staging_text(self, relative: &str, contents: &str) -> Self {
        let staging_dir = self.loopback_layout().staging_dir();
        self.ensure_loopback_staging()
            .write_text(&format!("{}/{}", staging_dir.display(), relative), contents)
    }
}

pub struct FakeConversionRunner {
    calls: RefCell<Vec<PackageBinaryConversionCommand>>,
    result: RefCell<Result<(), String>>,
    materialize_package_binary: bool,
}

impl Default for FakeConversionRunner {
    fn default() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            result: RefCell::new(Ok(())),
            materialize_package_binary: false,
        }
    }
}

impl FakeConversionRunner {
    pub fn recording() -> Self {
        Self::default()
    }

    pub fn materializing() -> Self {
        Self {
            materialize_package_binary: true,
            ..Self::default()
        }
    }

    pub fn failing(reason: impl Into<String>) -> Self {
        Self {
            result: RefCell::new(Err(reason.into())),
            ..Self::default()
        }
    }

    pub fn calls(&self) -> Vec<PackageBinaryConversionCommand> {
        self.calls.borrow().clone()
    }
}

impl PackageBinaryConversionRunner for FakeConversionRunner {
    fn run(&self, command: &PackageBinaryConversionCommand) -> Result<(), String> {
        self.calls.borrow_mut().push(command.clone());

        if self.materialize_package_binary {
            if let Some(parent) = command.package_binary_path().parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(command.package_binary_path(), b"payload")
                .map_err(|error| error.to_string())?;
        }

        self.result.borrow().clone()
    }
}
