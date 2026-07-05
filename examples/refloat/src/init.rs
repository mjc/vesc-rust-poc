//! Firmware entrypoints for the Refloat package.

vescpkg_rs::package_main!(crate::package::main);

#[cfg(test)]
mod tests {
    use vescpkg_rs::ffi;

    #[test]
    fn package_lib_init_runs_refloat_main() {
        assert!(super::package_lib_init(
            core::ptr::null_mut::<ffi::LibInfo>()
        ));
    }
}
