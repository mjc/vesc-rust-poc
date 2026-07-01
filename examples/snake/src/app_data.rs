use crate::game::{
    SnakeBoard, SnakeBoardHeight, SnakeBoardWidth, SnakeDirection, SnakeGame, SnakeSeed, SnakeState,
};

const DEFAULT_BOARD: SnakeBoard = SnakeBoard::new(
    SnakeBoardWidth::new(24).expect("snake width"),
    SnakeBoardHeight::new(24).expect("snake height"),
);
const DEFAULT_SEED: SnakeSeed = SnakeSeed::new(1);

/// Package-side app-data command byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeAppCommand(u8);

impl SnakeAppCommand {
    /// Advance the game one tick.
    pub const TICK: Self = Self(b'T');
    /// Queue upward movement.
    pub const UP: Self = Self(b'U');
    /// Queue downward movement.
    pub const DOWN: Self = Self(b'D');
    /// Queue leftward movement.
    pub const LEFT: Self = Self(b'L');
    /// Queue rightward movement.
    pub const RIGHT: Self = Self(b'R');
    /// Reset the game.
    pub const RESET: Self = Self(b'X');
    /// Probe handler entry without touching package state.
    pub const PROBE: Self = Self(b'P');
    /// Query current state without changing it.
    pub const STATE: Self = Self(b'S');

    /// Decode a command byte.
    pub const fn from_byte(value: u8) -> Option<Self> {
        match value {
            b'T' => Some(Self::TICK),
            b'U' => Some(Self::UP),
            b'D' => Some(Self::DOWN),
            b'L' => Some(Self::LEFT),
            b'R' => Some(Self::RIGHT),
            b'X' => Some(Self::RESET),
            b'P' => Some(Self::PROBE),
            b'S' => Some(Self::STATE),
            _ => None,
        }
    }
}

/// Fixed app-data response returned after every accepted command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeAppResponse {
    bytes: [u8; Self::LEN],
}

impl SnakeAppResponse {
    /// Response byte length.
    pub const LEN: usize = 10;

    /// Build a response snapshot from the current game state.
    pub fn from_game(game: SnakeGame, command: SnakeAppCommand, handler_count: u8) -> Self {
        let tick = game.tick().get();
        let score = game.score().get();
        Self {
            bytes: [
                b'S',
                snake_state_byte(game.state()),
                tick as u8,
                (tick >> 8) as u8,
                score as u8,
                (score >> 8) as u8,
                game.head().x(),
                game.head().y(),
                command.0,
                handler_count,
            ],
        }
    }

    /// Borrow the encoded response bytes.
    pub const fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.bytes
    }
}

/// Build the deterministic initial package-side game.
pub const fn new_package_game() -> SnakeGame {
    SnakeGame::new(DEFAULT_BOARD, DEFAULT_SEED)
}

/// Apply one app-data packet to a game and return a response when accepted.
pub fn process_snake_app_data(game: &mut SnakeGame, bytes: &[u8]) -> Option<SnakeAppResponse> {
    process_snake_app_data_with_count(game, bytes, 0)
}

/// Apply one app-data packet and include a target-side handler counter in the response.
pub fn process_snake_app_data_with_count(
    game: &mut SnakeGame,
    bytes: &[u8],
    handler_count: u8,
) -> Option<SnakeAppResponse> {
    let command = SnakeAppCommand::from_byte(*bytes.first()?)?;
    match command {
        SnakeAppCommand::TICK => {
            let _ = game.advance();
        }
        SnakeAppCommand::UP => {
            let _ = game.request_direction(SnakeDirection::Up);
        }
        SnakeAppCommand::DOWN => {
            let _ = game.request_direction(SnakeDirection::Down);
        }
        SnakeAppCommand::LEFT => {
            let _ = game.request_direction(SnakeDirection::Left);
        }
        SnakeAppCommand::RIGHT => {
            let _ = game.request_direction(SnakeDirection::Right);
        }
        SnakeAppCommand::RESET => game.reset(),
        SnakeAppCommand::STATE => {}
        _ => return None,
    }
    Some(SnakeAppResponse::from_game(*game, command, handler_count))
}

fn snake_state_byte(state: SnakeState) -> u8 {
    match state {
        SnakeState::Running => 1,
        SnakeState::Paused => 2,
        SnakeState::GameOver => 3,
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
static mut SNAKE_GAME: SnakeGame = new_package_game();
#[cfg(all(not(test), target_arch = "arm"))]
static mut SNAKE_HANDLER_COUNT: u8 = 0;

#[cfg(all(not(test), target_arch = "arm"))]
fn loaded_image_base() -> usize {
    let loaded_handler: usize;
    unsafe {
        core::arch::asm!(
            "adr {loaded_handler}, {handler}",
            loaded_handler = out(reg) loaded_handler,
            handler = sym snake_handle_app_data,
            options(nomem, nostack, preserves_flags),
        );
    }
    let loaded_handler = loaded_handler & !1;
    let image_handler = snake_handle_app_data as *const () as usize & !1;
    loaded_handler - image_handler
}

#[cfg(all(not(test), target_arch = "arm"))]
unsafe fn rebased_mut<T>(base_addr: usize, ptr: *mut T) -> *mut T {
    (base_addr + ptr as usize) as *mut T
}

/// Rebase an image-relative handler address into firmware-loaded memory.
pub fn rebase_handler_addr(info: &vescpkg_rs::ffi::LibInfo, handler_addr: usize) -> usize {
    vescpkg_rs::ffi::NativeImage::from_info(info).rebase_addr(handler_addr)
}

/// Register the Snake app-data handler with firmware.
#[cfg(all(not(test), target_arch = "arm"))]
pub fn register_snake_app_data_handler(info: &vescpkg_rs::ffi::LibInfo) -> bool {
    let handler_addr = rebase_handler_addr(info, snake_handle_app_data as *const () as usize);
    let handler: vescpkg_rs::ffi::AppDataHandler = unsafe { core::mem::transmute(handler_addr) };
    unsafe { vescpkg_rs::ffi::raw::vesc_set_app_data_handler(handler) }
}

/// Host non-test builds cannot install a firmware callback.
#[cfg(all(not(test), not(target_arch = "arm")))]
pub fn register_snake_app_data_handler(_info: &vescpkg_rs::ffi::LibInfo) -> bool {
    false
}

/// Device entrypoint invoked by firmware app-data delivery.
#[cfg(all(not(test), target_arch = "arm"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn snake_handle_app_data(data: *mut u8, len: u32) {
    if data.is_null() || len == 0 {
        return;
    }

    let bytes = unsafe { core::slice::from_raw_parts(data as *const u8, len as usize) };
    if matches!(
        SnakeAppCommand::from_byte(bytes[0]),
        Some(SnakeAppCommand::PROBE)
    ) {
        let response = [b'S', 0, 0, 0, 0, 0, 0, 0, SnakeAppCommand::PROBE.0, 0];
        unsafe {
            vescpkg_rs::ffi::raw::vesc_send_app_data(response.as_ptr(), response.len() as u32)
        };
        return;
    }

    let base_addr = loaded_image_base();
    let game = unsafe { &mut *rebased_mut(base_addr, core::ptr::addr_of_mut!(SNAKE_GAME)) };
    let handler_count_ptr =
        unsafe { rebased_mut(base_addr, core::ptr::addr_of_mut!(SNAKE_HANDLER_COUNT)) };
    let handler_count = unsafe {
        *handler_count_ptr = (*handler_count_ptr).wrapping_add(1);
        *handler_count_ptr
    };
    if let Some(response) = process_snake_app_data_with_count(game, bytes, handler_count) {
        let bytes = response.as_bytes();
        unsafe { vescpkg_rs::ffi::raw::vesc_send_app_data(bytes.as_ptr(), bytes.len() as u32) };
    }
}

#[cfg(test)]
mod tests {
    use super::{SnakeAppCommand, new_package_game, process_snake_app_data, rebase_handler_addr};
    use crate::game::{SnakeCell, SnakeDirection, SnakeState, SnakeTick};
    use vescpkg_rs::ffi::LibInfo;

    #[test]
    fn app_data_tick_advances_and_reports_state() {
        let mut game = new_package_game();
        let response =
            process_snake_app_data(&mut game, &[SnakeAppCommand::TICK.0]).expect("response");

        assert_eq!(game.tick(), SnakeTick::new(1));
        assert_eq!(game.direction(), SnakeDirection::Right);
        assert_eq!(game.head(), SnakeCell::new(13, 12));
        assert_eq!(
            response.as_bytes(),
            &[
                b'S',
                1,
                1,
                0,
                0,
                0,
                game.head().x(),
                game.head().y(),
                SnakeAppCommand::TICK.0,
                0
            ]
        );
    }

    #[test]
    fn app_data_turn_then_tick_runs_device_logic() {
        let mut game = new_package_game();
        assert!(process_snake_app_data(&mut game, &[SnakeAppCommand::UP.0]).is_some());
        assert!(process_snake_app_data(&mut game, &[SnakeAppCommand::TICK.0]).is_some());

        assert_eq!(game.direction(), SnakeDirection::Up);
        assert_eq!(game.head(), SnakeCell::new(12, 11));
    }

    #[test]
    fn app_data_reset_and_invalid_commands_are_bounded() {
        let mut game = new_package_game();
        assert!(process_snake_app_data(&mut game, &[SnakeAppCommand::TICK.0]).is_some());
        assert_eq!(game.tick(), SnakeTick::new(1));

        assert_eq!(process_snake_app_data(&mut game, &[0]), None);
        assert!(process_snake_app_data(&mut game, &[SnakeAppCommand::RESET.0]).is_some());
        assert_eq!(game.tick(), SnakeTick::new(0));
        assert_eq!(game.state(), SnakeState::Running);
    }

    #[test]
    fn app_data_handler_address_is_rebased_from_loader_metadata() {
        let info = LibInfo {
            stop_fun: None,
            arg: core::ptr::null_mut(),
            base_addr: 0x2000,
        };

        assert_eq!(rebase_handler_addr(&info, 0xb1), 0x20b1);
    }
}
