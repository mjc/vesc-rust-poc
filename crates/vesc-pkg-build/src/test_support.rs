use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

use crate::package_conversion::{PackageBinaryConversionCommand, PackageBinaryConversionRunner};

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
