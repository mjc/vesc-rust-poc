use std::path::PathBuf;
use std::sync::LazyLock;

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

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub struct NativeBuildWorkspace {
    plan: crate::native_lib_link::NativeLibLinkPlan,
}

static SHARED_REPO_WORKSPACE: LazyLock<NativeBuildWorkspace> =
    LazyLock::new(NativeBuildWorkspace::from_repo_root);

impl NativeBuildWorkspace {
    pub fn from_repo_root() -> Self {
        Self {
            plan: crate::native_lib_link::native_lib_link_plan(),
        }
    }

    pub fn shared() -> &'static Self {
        &SHARED_REPO_WORKSPACE
    }

    pub fn plan(&self) -> &crate::native_lib_link::NativeLibLinkPlan {
        &self.plan
    }

    pub fn native_lib_elf_path(&self) -> PathBuf {
        self.plan.elf_path()
    }

    pub fn native_lib_bin_path(&self) -> PathBuf {
        self.plan.native_lib_bin_path()
    }

    pub fn rust_staticlib_path(&self) -> PathBuf {
        self.plan.rust_staticlib_path()
    }

    pub fn package_lib_object_path(&self) -> PathBuf {
        self.plan.package_c_object_path()
    }
}
