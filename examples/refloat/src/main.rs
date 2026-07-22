#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]
#![allow(missing_docs)]

#[cfg(target_arch = "arm")]
use vesc_example_refloat as _;

#[cfg(not(target_arch = "arm"))]
fn main() {}
