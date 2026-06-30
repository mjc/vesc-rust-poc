use std::process::Command;

/// Toolchain abstraction used by native-lib build orchestration.
pub trait NativeLibToolchain {
    /// Runs `program` with `args`, returning a human-readable failure string on error.
    fn run(&self, program: &str, args: &[&str]) -> Result<(), String>;
}

/// Host toolchain implementation that invokes real subprocesses.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealNativeLibToolchain;

impl NativeLibToolchain for RealNativeLibToolchain {
    fn run(&self, program: &str, args: &[&str]) -> Result<(), String> {
        let status = Command::new(program)
            .args(args)
            .status()
            .map_err(|error| format!("{program} execution failed: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{program} exited with {status}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeLibToolchain, RecordingNativeLibToolchain};

    #[test]
    fn recording_toolchain_captures_invocations() {
        let toolchain = RecordingNativeLibToolchain::default();
        NativeLibToolchain::run(&toolchain, "arm-none-eabi-gcc", &["-v"])
            .expect("record gcc invocation");
        let calls = toolchain.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "arm-none-eabi-gcc");
        assert_eq!(calls[0].1, vec!["-v".to_owned()]);
    }
}

#[cfg(test)]
/// Test toolchain that records native-lib subprocess invocations.
#[derive(Default)]
pub struct RecordingNativeLibToolchain {
    /// Recorded program names and argument lists.
    pub calls: std::cell::RefCell<Vec<(String, Vec<String>)>>,
}

#[cfg(test)]
impl NativeLibToolchain for RecordingNativeLibToolchain {
    fn run(&self, program: &str, args: &[&str]) -> Result<(), String> {
        self.calls.borrow_mut().push((
            program.to_owned(),
            args.iter().map(|arg| (*arg).to_owned()).collect(),
        ));
        Ok(())
    }
}
