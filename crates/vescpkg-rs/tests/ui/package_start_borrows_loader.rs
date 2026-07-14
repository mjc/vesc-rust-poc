fn dangling_start() -> vescpkg_rs::PackageStart<'static> {
    let mut info = unsafe { core::mem::MaybeUninit::<vescpkg_rs::LoaderInfo>::zeroed().assume_init() };
    vescpkg_rs::PackageStart::from_info(&mut info)
}

fn main() {}
