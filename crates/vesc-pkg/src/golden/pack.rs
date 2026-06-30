use std::fs;
use std::io;
use std::path::Path;

use crate::package_format::build_lisp_data;
use crate::{BLE_LOOPBACK_PACKAGE_NAME, PackageAssets, PackageLayout, PackageProvenance};

use super::fixtures;

/// Builds Lisp package data for the loopback golden fixture in `workspace`.

pub fn pack_lisp_data(package_lib: &[u8], workspace: &Path) -> io::Result<Vec<u8>> {
    let assets = PackageAssets::new(
        PackageLayout::new(BLE_LOOPBACK_PACKAGE_NAME, fixtures::VERSION),
        PackageProvenance::empty(),
    );
    let staging_dir = workspace.join(assets.staging_dir());
    let src_dir = staging_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("package_lib.bin"), package_lib)?;
    build_lisp_data(&assets.render_loader(), &staging_dir)
}
