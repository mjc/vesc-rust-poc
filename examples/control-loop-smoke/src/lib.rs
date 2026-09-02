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

pub use vesc_protocol::control_loop::{
    CommandError, ControlLoopStatus, SETPOINT_COMMAND, STATUS_BYTES, STATUS_COMMAND,
    encode_setpoint_command, encode_status_command,
};

const ACK_BYTES: usize = 2;

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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            setpoint: 0,
            sampled_input: 0,
            output: 0,
            tick_count: 0,
        }
    }

    /// Return the requested setpoint.
    #[must_use]
    pub const fn setpoint(self) -> i16 {
        self.setpoint
    }

    /// Return the synthetic sampled input.
    #[must_use]
    pub const fn sampled_input(self) -> i16 {
        self.sampled_input
    }

    /// Return the computed, non-actuating control output.
    #[must_use]
    pub const fn output(self) -> i16 {
        self.output
    }

    /// Return the number of completed loop ticks.
    #[must_use]
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
        self.sampled_input = self.sampled_input.saturating_add(saturating_i16(error / 2));
        self.output = saturating_i16(error);
        self.tick_count = self.tick_count.wrapping_add(1);
    }
}

fn saturating_i16(value: i32) -> i16 {
    i16::try_from(value).unwrap_or(if value.is_negative() {
        i16::MIN
    } else {
        i16::MAX
    })
}

/// Handle one host command without touching firmware or performing I/O.
///
/// # Errors
///
/// Returns a typed command error for an unknown command, malformed request, or
/// response buffer that cannot hold the selected reply.
pub fn handle_command(
    state: &mut ControlLoopState,
    packet: &[u8],
    response: &mut [u8],
) -> Result<usize, CommandError> {
    let command = packet.first().copied().ok_or(CommandError::InvalidLength)?;
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

    fn handle(
        context: &mut vescpkg_rs::StatefulCallbackContext<'_, Self::State>,
        packet: vescpkg_rs::AppDataPacket<'_>,
        reply: &mut vescpkg_rs::AppDataReply<'_>,
    ) {
        let mut bytes = [0_u8; STATUS_BYTES];
        let Ok(response_len) =
            context.with_state(|state| handle_command(state, packet.as_bytes(), &mut bytes))
        else {
            return;
        };
        let _ = reply.write(&bytes[..response_len]);
    }
}

#[cfg(all(not(test), target_arch = "arm"))]
vescpkg_rs::firmware_stateful_app_data_callback!(
    control_loop_app_data_callback,
    ControlLoopAppData
);

vescpkg_rs::package_start!(crate::start, ControlLoopState);

/// Initialize the example package.
///
/// # Errors
///
/// Returns an error when runtime-state installation, thread creation, or
/// app-data callback registration fails.
#[cfg(any(test, all(not(test), target_arch = "arm")))]
pub fn start(start: &mut vescpkg_rs::PackageStart) -> Result<(), vescpkg_rs::PackageStartError> {
    start.install_runtime_state(ControlLoopState::new())?;
    #[cfg(all(not(test), target_arch = "arm"))]
    {
        let stack = vescpkg_rs::ThreadWorkingAreaSize::try_from_bytes(1_024)
            .map_err(|_| vescpkg_rs::PackageStartError::ThreadSpawnFailed)?;
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
    fn control_step_saturates_extreme_errors() {
        let mut state = ControlLoopState {
            setpoint: i16::MAX,
            sampled_input: i16::MIN,
            output: 0,
            tick_count: 0,
        };
        state.tick();
        assert_eq!(state.output(), i16::MAX);

        state.setpoint = i16::MIN;
        state.sampled_input = i16::MAX;
        state.tick();
        assert_eq!(state.output(), i16::MIN);
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
