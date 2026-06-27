use std::path::{Path, PathBuf};
use std::sync::LazyLock;

#[cfg(unix)]
use std::os::unix::fs::symlink;

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

    pub fn with_repo_fixture_layout() -> Self {
        let workspace = Self::new();
        link_repo_fixtures(&workspace.root);
        workspace
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(unix)]
pub fn link_repo_fixtures(root: &Path) {
    let repo = repo_root();
    symlink(repo.join("fixtures"), root.join("fixtures")).expect("fixtures symlink");
    symlink(repo.join("scripts"), root.join("scripts")).expect("scripts symlink");
    symlink(
        repo.join("fixtures/native-lib-baseline/package"),
        root.join("package"),
    )
    .expect("package symlink");
}

#[cfg(not(unix))]
pub fn link_repo_fixtures(root: &Path) {
    let _ = root;
    panic!("TempWorkspace fixture layout requires Unix symlinks");
}

enum NativeBuildRoot {
    Isolated(TempWorkspace),
    Repo,
}

pub struct NativeBuildWorkspace {
    root: NativeBuildRoot,
    plan: crate::native_lib_link::NativeLibLinkPlan,
}

static SHARED_REPO_WORKSPACE: LazyLock<NativeBuildWorkspace> =
    LazyLock::new(NativeBuildWorkspace::from_repo_root);

impl NativeBuildWorkspace {
    pub fn new() -> Self {
        let workspace = TempWorkspace::with_repo_fixture_layout();
        let plan = crate::native_lib_link::NativeLibLinkPlan::new(workspace.root.clone());
        Self {
            root: NativeBuildRoot::Isolated(workspace),
            plan,
        }
    }

    pub fn from_repo_root() -> Self {
        Self {
            root: NativeBuildRoot::Repo,
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
