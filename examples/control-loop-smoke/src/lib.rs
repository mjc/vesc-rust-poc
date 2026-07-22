//! Usage-shaped, no-actuation control-loop example.
//!
//! The loop owns a small state record, updates it under the SDK runtime gate, and sleeps
//! only after releasing the state borrow. App-data commands use the same state gate and
//! expose a bounded status response suitable for a host probe.

#![no_std]
#![forbid(unsafe_code)]
#![forbid(unused_extern_crates)]

#[cfg(test)]
extern crate std;

#[cfg(all(not(test), target_arch = "arm"))]
use core::time::Duration;

#[cfg(all(not(test), target_arch = "arm"))]
use vescpkg_rs::FirmwareThreads;

/// Command byte that changes the signed setpoint.
pub const SETPOINT_COMMAND: u8 = 1;
/// Command byte that reads the shared control-loop state.
pub const STATUS_COMMAND: u8 = 2;
const ACK_BYTES: usize = 2;
/// Encoded response size for [`ControlLoopStatus`].
pub const STATUS_BYTES: usize = 11;

/// Owned status returned by the control-loop example's wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlLoopStatus {
    setpoint: i16,
    sampled_input: i16,
    output: i16,
    tick_count: u32,
}

impl ControlLoopStatus {
    /// Decode one status response without allocating or retaining firmware data.
    pub fn decode(response: &[u8]) -> Result<Self, CommandError> {
        if response.len() != STATUS_BYTES {
            return Err(CommandError::InvalidLength);
        }
        if response[0] != STATUS_COMMAND {
            return Err(CommandError::UnexpectedResponse);
        }
        Ok(Self {
            setpoint: i16::from_le_bytes([response[1], response[2]]),
            sampled_input: i16::from_le_bytes([response[3], response[4]]),
            output: i16::from_le_bytes([response[5], response[6]]),
            tick_count: u32::from_le_bytes(response[7..11].try_into().unwrap()),
        })
    }

    /// Return the requested setpoint.
    pub const fn setpoint(self) -> i16 {
        self.setpoint
    }

    /// Return the synthetic sampled input.
    pub const fn sampled_input(self) -> i16 {
        self.sampled_input
    }

    /// Return the computed, non-actuating output.
    pub const fn output(self) -> i16 {
        self.output
    }

    /// Return the number of completed loop ticks reported by firmware.
    pub const fn tick_count(self) -> u32 {
        self.tick_count
    }
}

/// Encode a setpoint command for the control-loop callback.
pub const fn encode_setpoint_command(setpoint: i16) -> [u8; 3] {
    let [low, high] = setpoint.to_le_bytes();
    [SETPOINT_COMMAND, low, high]
}

/// Encode a status request for the control-loop callback.
pub const fn encode_status_command() -> [u8; 1] {
    [STATUS_COMMAND]
}

/// State shared by the periodic loop and app-data callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlLoopState {
    setpoint: i16,
    sampled_input: i16,
    output: i16,
    tick_count: u32,
}

impl Default for ControlLoopState {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlLoopState {
    /// Create an idle, no-actuation control state.
    pub const fn new() -> Self {
        Self {
            setpoint: 0,
            sampled_input: 0,
            output: 0,
            tick_count: 0,
        }
    }

    /// Return the requested setpoint.
    pub const fn setpoint(self) -> i16 {
        self.setpoint
    }

    /// Return the synthetic sampled input.
    pub const fn sampled_input(self) -> i16 {
        self.sampled_input
    }

    /// Return the computed, non-actuating control output.
    pub const fn output(self) -> i16 {
        self.output
    }

    /// Return the number of completed loop ticks.
    pub const fn tick_count(self) -> u32 {
        self.tick_count
    }

    /// Replace the requested setpoint without touching the sampled input.
    pub const fn set_setpoint(&mut self, setpoint: i16) {
        self.setpoint = setpoint;
    }

    /// Advance the deliberately simple proportional control step.
    pub fn tick(&mut self) {
        let error = i32::from(self.setpoint) - i32::from(self.sampled_input);
        self.sampled_input = self.sampled_input.saturating_add((error / 2) as i16);
        self.output = error.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        self.tick_count = self.tick_count.wrapping_add(1);
    }
}

/// Error returned by the bounded control-loop command decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// The command byte is not part of this example protocol.
    UnknownCommand,
    /// The command did not contain exactly the required bytes.
    InvalidLength,
    /// The response buffer is too short for the requested response.
    ResponseTooShort,
    /// A response did not carry the expected command byte.
    UnexpectedResponse,
}

/// Handle one host command without touching firmware or performing I/O.
pub fn handle_command(
    state: &mut ControlLoopState,
    packet: &[u8],
    response: &mut [u8],
) -> Result<usize, CommandError> {
    let Some(&command) = packet.first() else {
        return Err(CommandError::InvalidLength);
    };
    match command {
        SETPOINT_COMMAND => {
            if packet.len() != 3 {
                return Err(CommandError::InvalidLength);
            }
            if response.len() < ACK_BYTES {
                return Err(CommandError::ResponseTooShort);
            }
            state.set_setpoint(i16::from_le_bytes([packet[1], packet[2]]));
            response[..ACK_BYTES].copy_from_slice(&[SETPOINT_COMMAND, 0]);
            Ok(ACK_BYTES)
        }
        STATUS_COMMAND => {
            if packet.len() != 1 {
                return Err(CommandError::InvalidLength);
            }
            if response.len() < STATUS_BYTES {
                return Err(CommandError::ResponseTooShort);
            }
            response[0] = STATUS_COMMAND;
            response[1..3].copy_from_slice(&state.setpoint.to_le_bytes());
            response[3..5].copy_from_slice(&state.sampled_input.to_le_bytes());
            response[5..7].copy_from_slice(&state.output.to_le_bytes());
            response[7..11].copy_from_slice(&state.tick_count.to_le_bytes());
            Ok(STATUS_BYTES)
        }
        _ => Err(CommandError::UnknownCommand),
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
struct ControlLoopThread;

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::FirmwareThread for ControlLoopThread {
    type State = ControlLoopState;

    fn run(ctx: vescpkg_rs::ThreadContext<Self::State>) {
        let threads = ctx.firmware().threads();
        while !threads.should_terminate() {
            let _ = ctx.with_state_mut(ControlLoopState::tick);
            threads.sleep_for(Duration::from_millis(33));
        }
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
struct ControlLoopAppData;

#[cfg(all(not(test), target_arch = "arm"))]
impl vescpkg_rs::AppDataHandler for ControlLoopAppData {
    type State = ControlLoopState;

    fn handle(state: &mut Self::State, packet: vescpkg_rs::AppDataPacket<'_>) {
        let mut response = [0_u8; STATUS_BYTES];
        let Ok(response_len) = handle_command(state, packet.as_bytes(), &mut response) else {
            return;
        };
        let _ = vescpkg_rs::Firmware::new()
            .app_data()
            .send(&response[..response_len]);
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
vescpkg_rs::firmware_stateful_app_data_callback!(
    control_loop_app_data_callback,
    ControlLoopAppData
);

vescpkg_rs::package_start!(crate::start, ControlLoopState);

/// Initialize the example package.
#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub fn start(start: &mut vescpkg_rs::PackageStart) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state(ControlLoopState::new())?;
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        let stack = vescpkg_rs::ThreadWorkingAreaSize::try_from_bytes(1_024)
            .expect("control-loop thread stack satisfies ChibiOS alignment");
        start.spawn_threads([vescpkg_rs::ThreadSpec::<ControlLoopState>::new::<
            ControlLoopThread,
        >(stack, vescpkg_rs::thread_name!("Control Loop"))])?;
        start
            .app_data_callback::<ControlLoopAppData>()
            .ok_or(vescpkg_rs::PackageStartError::StateTypeMismatch)?
            .register()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CommandError, ControlLoopState, ControlLoopStatus, SETPOINT_COMMAND, STATUS_BYTES,
        STATUS_COMMAND, encode_setpoint_command, encode_status_command, handle_command,
    };

    #[test]
    fn control_step_moves_sample_and_reports_error_output() {
        let mut state = ControlLoopState::new();
        state.set_setpoint(100);
        state.tick();

        assert_eq!(state.sampled_input(), 50);
        assert_eq!(state.output(), 100);
        assert_eq!(state.tick_count(), 1);
    }

    #[test]
    fn setpoint_and_status_commands_share_state() {
        let mut state = ControlLoopState::new();
        let mut response = [0_u8; STATUS_BYTES];

        assert_eq!(
            handle_command(&mut state, &encode_setpoint_command(100), &mut response,),
            Ok(2)
        );
        state.tick();
        let len =
            handle_command(&mut state, &encode_status_command(), &mut response).expect("status");

        assert_eq!(len, STATUS_BYTES);
        let status = ControlLoopStatus::decode(&response).expect("decode status");
        assert_eq!(status.setpoint(), 100);
        assert_eq!(status.sampled_input(), 50);
        assert_eq!(status.output(), 100);
        assert_eq!(status.tick_count(), 1);
    }

    #[test]
    fn command_decoder_rejects_malformed_requests_and_buffers() {
        let mut state = ControlLoopState::new();
        let mut response = [0_u8; STATUS_BYTES];

        assert_eq!(
            handle_command(&mut state, &[1, 1], &mut response),
            Err(CommandError::InvalidLength)
        );
        assert_eq!(
            handle_command(&mut state, &[99], &mut response),
            Err(CommandError::UnknownCommand)
        );
        assert_eq!(
            handle_command(&mut state, &[2], &mut [0_u8; 2]),
            Err(CommandError::ResponseTooShort)
        );
    }

    #[test]
    fn status_decoder_rejects_wrong_command_and_length() {
        assert_eq!(
            ControlLoopStatus::decode(&[STATUS_COMMAND]),
            Err(CommandError::InvalidLength)
        );
        let mut response = [0_u8; STATUS_BYTES];
        response[0] = SETPOINT_COMMAND;
        assert_eq!(
            ControlLoopStatus::decode(&response),
            Err(CommandError::UnexpectedResponse)
        );
    }

    #[test]
    fn package_start_installs_the_shared_state_on_host() {
        let mut info = vescpkg_rs::test_support::LoaderInfo::new();
        let mut start = vescpkg_rs::PackageStart::from_info(&mut info);

        assert_eq!(super::start(&mut start), Ok(()));
        assert!(start.finish_start(true));
        assert!(info.has_stop_handler());
    }
}
