//! Snake VESC package payload.
//!
//! This crate is the linkable staticlib artifact (`libvesc_example_snake.a`) for the
//! unofficial Snake example. Generic loader, lifecycle, and firmware wrapper code lives in
//! `vescpkg-rs`.
//!
//! Device builds must stay `no_std` and must not link `alloc` or `std`.

#![no_std]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

/// App-data command handler that lets VESC firmware run the package-side Snake logic.
pub mod app_data;
pub mod game;
pub mod init;

pub use init::package_lib_init;
pub use vescpkg_rs::{ffi, lifecycle};

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::{ffi, game, init};

    #[test]
    fn package_lib_init_installs_stop_hook() {
        let mut info = ffi::LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0,
        };

        assert!(init::package_lib_init(&mut info));
        assert!(info.stop_fun.is_some());
    }

    #[test]
    fn package_game_advances_with_typed_state() {
        let board = game::SnakeBoard::new(
            game::SnakeBoardWidth::new(8).expect("width"),
            game::SnakeBoardHeight::new(6).expect("height"),
        );
        let mut model = game::SnakeGame::new(board, game::SnakeSeed::new(42));

        assert_eq!(model.tick(), game::SnakeTick::new(0));
        assert_eq!(model.score(), game::SnakeScore::new(0));
        assert_eq!(model.direction(), game::SnakeDirection::Right);

        model
            .request_direction(game::SnakeDirection::Down)
            .expect("turn down");
        assert_eq!(model.advance(), game::SnakeStep::Advanced);
        assert_eq!(model.tick(), game::SnakeTick::new(1));
        assert_eq!(model.direction(), game::SnakeDirection::Down);
    }
}
