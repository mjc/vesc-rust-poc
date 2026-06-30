use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use crate::package_conversion::{PackageBinaryConversionCommand, PackageBinaryConversionRunner};
use crate::{BLE_LOOPBACK_PACKAGE_NAME, PackageLayout};

/// Version used by loopback package test fixtures.
const LOOPBACK_PACKAGE_VERSION: &str = "0.1.0";

/// Temporary workspace that keeps its backing directory alive for tests.
pub struct TempWorkspace {
    _temp: tempfile::TempDir,
    /// Root path of the temporary workspace.
    pub root: PathBuf,
}

impl Default for TempWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl TempWorkspace {
    /// Creates a new empty temporary workspace.
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        Self { _temp: temp, root }
    }
}

/// Builder-style test harness for package staging and artifact assertions.
pub struct PackageTestHarness {
    workspace: TempWorkspace,
}

impl Default for PackageTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageTestHarness {
    /// Creates an empty package test harness.
    pub fn new() -> Self {
        Self {
            workspace: TempWorkspace::new(),
        }
    }

    /// Returns the temporary workspace root.
    pub fn root(&self) -> &Path {
        &self.workspace.root
    }

    /// Writes bytes at `relative` under the temporary workspace root.
    pub fn write_bytes(self, relative: &str, contents: impl AsRef<[u8]>) -> Self {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("artifact parent directory");
        }
        fs::write(path, contents.as_ref()).expect("artifact contents");
        self
    }

    /// Writes UTF-8 text at `relative` under the temporary workspace root.
    pub fn write_text(self, relative: &str, contents: &str) -> Self {
        self.write_bytes(relative, contents.as_bytes())
    }

    /// Writes a native payload at the loopback package import path.
    pub fn write_native_payload(self, payload: impl AsRef<[u8]>) -> Self {
        self.write_bytes("src/package_lib.bin", payload)
    }

    /// Returns the standard loopback package layout used by tests.
    pub fn loopback_layout(&self) -> PackageLayout {
        PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, LOOPBACK_PACKAGE_VERSION)
    }

    /// Returns the standard loopback package staging directory path.
    pub fn loopback_staging_dir(&self) -> PathBuf {
        self.root().join(self.loopback_layout().staging_dir())
    }

    /// Ensures the standard loopback staging directory exists.
    pub fn ensure_loopback_staging(self) -> Self {
        fs::create_dir_all(self.loopback_staging_dir()).expect("loopback staging dir");
        self
    }

    /// Returns a loader script that imports and loads the native payload.
    pub fn loopback_loader_lisp(&self) -> String {
        "(import \"src/package_lib.bin\" 'package-lib)\n(load-native-lib package-lib)\n".to_owned()
    }

    /// Returns a loader script that only imports the native payload.
    pub fn loopback_loader_lisp_import_only(&self) -> String {
        "(import \"src/package_lib.bin\" 'package-lib)\n".to_owned()
    }

    /// Writes text inside the standard loopback staging directory.
    pub fn write_loopback_staging_text(self, relative: &str, contents: &str) -> Self {
        let staging_dir = self.loopback_layout().staging_dir();
        self.ensure_loopback_staging()
            .write_text(&format!("{}/{}", staging_dir.display(), relative), contents)
    }
}

/// Fake package binary conversion runner used by package target tests.
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
    /// Creates a fake runner that records calls and reports success.
    pub fn recording() -> Self {
        Self::default()
    }

    /// Creates a fake runner that also materializes the expected package binary.
    pub fn materializing() -> Self {
        Self {
            materialize_package_binary: true,
            ..Self::default()
        }
    }

    /// Creates a fake runner that fails with `reason`.
    pub fn failing(reason: impl Into<String>) -> Self {
        Self {
            result: RefCell::new(Err(reason.into())),
            ..Self::default()
        }
    }

    /// Returns the conversion commands recorded by this fake runner.
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
