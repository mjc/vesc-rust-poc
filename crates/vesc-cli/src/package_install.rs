//! Host-side VESC package install protocol.
//!
//! Thin re-exports of the library API in [`vescpkg_rs_build`].

pub use vescpkg_rs_build::Package as VescPackage;
pub use vescpkg_rs_build::install::{
    FakeInstallTransport as FakePackageInstallTransport, InstallError as PackageInstallError,
    InstallReport as PackageInstallReport, InstallStep as PackageInstallStep,
    InstallTransport as PackageInstallTransport, Installer, erase_package, install_package,
};

/// Reads and decodes a package from a filesystem path.
pub fn read_package_from_path(
    path: impl AsRef<std::path::Path>,
) -> Result<VescPackage, PackageInstallError> {
    VescPackage::read(path).map_err(Into::into)
}

/// Decodes raw package bytes into an installable VESC package model.
pub fn decode_package(data: &[u8]) -> Result<VescPackage, PackageInstallError> {
    VescPackage::from_bytes(data).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{decode_package, erase_package, install_package, read_package_from_path};
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;
    use std::path::Path;
    use vescpkg_rs_build::FakeInstallTransport;

    fn build_package_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        write_string(&mut data, "VESC Packet");
        write_field(&mut data, "name", b"Rust BLE loopback test package");
        write_field(&mut data, "qmlFile", b"import QtQuick 2.15\nItem {}\n");
        write_field(
            &mut data,
            "lispData",
            b"(load-native-lib \"src/package_lib.bin\")\n",
        );
        write_field(&mut data, "qmlIsFullscreen", &[1]);
        q_compress(&data)
    }

    fn write_string(buf: &mut Vec<u8>, value: &str) {
        buf.extend_from_slice(value.as_bytes());
        buf.push(0);
    }

    fn write_field(buf: &mut Vec<u8>, name: &str, data: &[u8]) {
        write_string(buf, name);
        buf.extend_from_slice(&(data.len() as i32).to_be_bytes());
        buf.extend_from_slice(data);
    }

    fn q_compress(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut output = Vec::with_capacity(4 + compressed.len());
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(&compressed);
        output
    }

    #[test]
    fn decodes_a_compressed_vesc_package() {
        let package = decode_package(&build_package_bytes()).expect("package");
        assert_eq!(package.name, "Rust BLE loopback test package");
        assert!(package.qml_is_fullscreen);
        assert!(package.is_valid());
    }

    #[test]
    fn decodes_refloat_vesc_tool_fixture_when_present() {
        let path = Path::new("/home/mjc/projects/refloat/refloat.vescpkg");
        if !path.exists() {
            return;
        }

        let package = read_package_from_path(path).expect("refloat package");
        assert_eq!(package.name, "Refloat");
        assert!(!package.qml_file.is_empty());
        assert!(!package.lisp_data.is_empty());
    }

    #[test]
    fn installs_package_in_vesc_tool_order() {
        let package = decode_package(&build_package_bytes()).expect("package");
        let transport = FakeInstallTransport::default();
        let report = install_package(&package, &transport).expect("report");
        assert_eq!(report.steps.len(), 6);
    }

    #[test]
    fn erases_package_in_vesc_tool_order() {
        let transport = FakeInstallTransport::default();
        erase_package(&transport).expect("report");
        assert_eq!(transport.steps.borrow().len(), 3);
    }
}
