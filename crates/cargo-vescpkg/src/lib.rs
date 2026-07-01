//! Command-line tool for building, installing, and debugging Rust VESC packages.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{execute, queue};

/// Parsed top-level command requested by the operator-facing CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print CLI usage information.
    Help,
    /// Print package layout information.
    Layout,
    /// Print host and build status information.
    Status,
    /// Scan for nearby VESC BLE UART devices.
    Scan,
    /// Run the BLE loopback protocol against a target device.
    Loopback(LoopbackCommand),
    /// Run the Lisp probe diagnostic against a target device.
    LispProbe(LispProbeCommand),
    /// Install a package on a target device.
    PackageInstall(PackageInstallCommand),
    /// Erase an installed package from a target device.
    ErasePackage(PackageEraseCommand),
    /// Build, install, and smoke-test a package on a target device.
    Deploy(PackageInstallCommand),
    /// Install Snake and smoke-test its app-data handler on a target device.
    SnakeDeploy(PackageInstallCommand),
    /// Run the terminal Snake example surface.
    Snake(SnakeCommand),
}

/// Arguments for the `loopback` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `lisp-probe` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispProbeCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for package install and deploy commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInstallCommand {
    /// Path to the `.vescpkg` file to install.
    pub package_path: String,
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `erase-package` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEraseCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
}

/// Arguments for the `snake` example command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeCommand {
    /// Optional BLE device-name filter.
    pub device_name: Option<String>,
    /// Optional BLE address filter.
    pub address: Option<String>,
    /// Board width in cells.
    pub board_width: snake::SnakeBoardWidth,
    /// Board height in cells.
    pub board_height: snake::SnakeBoardHeight,
    /// Deterministic seed used by scripted and fake sessions.
    pub seed: snake::SnakeSeed,
    /// Scripted WASD-style direction input for CLI-only sessions.
    pub moves: Vec<snake::SnakeDirection>,
    /// Scripted local-play input actions for CLI-only sessions.
    pub actions: Vec<snake::SnakeLocalAction>,
    /// How the local Snake command should consume input.
    pub run_mode: SnakeRunMode,
    /// Number of ticks to render after the initial frame.
    pub tick_limit: snake::SnakeTickLimit,
    /// Milliseconds between automatic Snake ticks in interactive mode.
    pub tick_interval: snake::SnakeTickInterval,
}

/// Input mode for the local Snake command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeRunMode {
    /// Run a timer-driven terminal game loop.
    Interactive,
    /// Consume `--moves` as one direction per tick.
    ScriptedMoves,
    /// Consume `--actions` as explicit local session actions.
    ScriptedActions,
}

/// Errors returned while parsing CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The first non-program argument did not match a supported command.
    UnknownCommand(String),
}

/// Run a parsed CLI invocation and return the process exit code.
pub fn run_args<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match parse_args(args) {
        Ok(Command::Help) => {
            println!(
                "cargo vescpkg: use `build`, `layout`, `status`, `scan`, `loopback`, `lisp-probe`, `deploy`, `snake-deploy`, `package-install`, `erase-package`, or `snake`"
            );
            ExitCode::SUCCESS
        }
        Ok(Command::Layout) => {
            println!("workspace layout is documented in docs/workspace-layout.md");
            ExitCode::SUCCESS
        }
        Ok(Command::Status) => {
            println!("status: placeholder host surface");
            ExitCode::SUCCESS
        }
        Ok(Command::Scan) => match btle::scan_devices() {
            Ok(devices) => {
                devices.into_iter().for_each(|device| {
                    println!(
                        "{} {:?} {:?}",
                        device.identifier, device.local_name, device.services
                    );
                });
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("scan failed: {error}");
                ExitCode::from(1)
            }
        },
        Ok(Command::Loopback(command)) => {
            let target = loopback_target(command.address, command.device_name);

            match loopback_debug::run_loopback_with_diagnostics(target, |event| {
                if event.should_print_to_cli() {
                    println!("loopback: {}", event.describe());
                }
            }) {
                Ok(report) => {
                    println!(
                        "loopback ok on device={} service={}: {:?}",
                        report.target().device_name_hint(),
                        report.target().service_name_hint(),
                        report.commands()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("loopback failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::LispProbe(command)) => {
            let target = loopback_target(command.address, command.device_name);

            match btle::run_lisp_probe_with_progress(target, |event| {
                if event.should_print_to_cli() {
                    println!("lisp probe: {}", event.describe());
                }
            }) {
                Ok(report) => {
                    let ok = report
                        .prints()
                        .iter()
                        .any(|line| line.contains("vesc-rust-probe-ok-42"));
                    if ok {
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("lisp probe: missing expected vesc-rust-probe-ok-42 print");
                        ExitCode::from(1)
                    }
                }
                Err(error) => {
                    eprintln!("lisp probe failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::Deploy(command)) => {
            let package_path = command.package_path;
            let target = loopback_target(command.address, command.device_name);

            match deploy::run_deploy(&package_path, target, |event| {
                if event.should_print_to_cli() {
                    println!("loopback: {}", event.describe());
                }
            }) {
                Ok((install, report)) => {
                    println!(
                        "package install ok for {}: {:?}",
                        install.package_name, install.steps
                    );
                    println!(
                        "loopback ok on device={} service={}: {:?}",
                        report.target().device_name_hint(),
                        report.target().service_name_hint(),
                        report.commands()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("deploy failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(Command::PackageInstall(command)) => run_package_install(command),
        Ok(Command::ErasePackage(command)) => run_package_erase(command),
        Ok(Command::SnakeDeploy(command)) => run_snake_deploy(command),
        Ok(Command::Snake(command)) => run_snake(command),
        Err(ParseError::UnknownCommand(command)) => {
            eprintln!("unknown command: {command}");
            ExitCode::from(2)
        }
    }
}

fn run_snake(command: SnakeCommand) -> ExitCode {
    let Some(board) =
        snake::SnakeBoardSize::new(command.board_width.get(), command.board_height.get())
    else {
        eprintln!("snake failed: invalid board size");
        return ExitCode::from(2);
    };
    let model = snake::SnakeModel::new(board, command.seed.get());
    let mut model = model;
    match command.run_mode {
        SnakeRunMode::Interactive => {
            let stdout = io::stdout();
            match run_snake_terminal(
                &mut model,
                stdout.lock(),
                command.tick_limit,
                command.tick_interval,
            ) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("snake failed: {error:?}");
                    ExitCode::from(1)
                }
            }
        }
        SnakeRunMode::ScriptedMoves => {
            match snake::render_scripted_terminal_session(
                &mut model,
                command.moves,
                command.tick_limit,
            ) {
                Ok(output) => {
                    print!("{output}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("snake failed: {error:?}");
                    ExitCode::from(1)
                }
            }
        }
        SnakeRunMode::ScriptedActions => {
            match snake::render_local_terminal_session(
                &mut model,
                command.actions,
                command.tick_limit,
            ) {
                Ok(report) => {
                    print!("{}", report.output());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("snake failed: {error:?}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

fn run_snake_deploy(command: PackageInstallCommand) -> ExitCode {
    let package_path = command.package_path;
    let target = loopback_target(command.address, command.device_name);

    match snake_deploy::run_snake_deploy(&package_path, target) {
        Ok((install, report)) => {
            println!(
                "package install ok for {}: {:?}",
                install.package_name, install.steps
            );
            println!(
                "snake app-data ok on device={} service={}: reset={:?} tick={:?} state={:?}",
                report.target.device_name_hint(),
                report.target.service_name_hint(),
                report.reset,
                report.tick,
                report.state
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("snake deploy failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_snake_terminal<W>(
    model: &mut snake::SnakeModel,
    mut output: W,
    tick_limit: snake::SnakeTickLimit,
    tick_interval: snake::SnakeTickInterval,
) -> Result<(), snake::SnakeTransitionError>
where
    W: Write,
{
    enable_raw_mode().map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?;
    execute!(output, EnterAlternateScreen, Hide)
        .map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?;

    let result = run_snake_terminal_loop(model, &mut output, tick_limit, tick_interval);

    let _ = execute!(output, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalSnakePoint {
    x: u16,
    y: u16,
}

#[derive(Debug)]
struct TerminalSnakeGame {
    width: u16,
    height: u16,
    snake: VecDeque<TerminalSnakePoint>,
    direction: snake::SnakeDirection,
    pending_direction: Option<snake::SnakeDirection>,
    food: TerminalSnakePoint,
    score: u32,
    rng: u32,
    game_over: bool,
}

impl TerminalSnakeGame {
    fn new(board: snake::SnakeBoardSize, seed: u32) -> Self {
        let width = u16::from(board.width()).max(12);
        let height = u16::from(board.height()).max(12);
        let head = TerminalSnakePoint {
            x: width / 2,
            y: height / 2,
        };
        let mut snake = VecDeque::new();
        snake.push_front(head);
        snake.push_back(TerminalSnakePoint {
            x: head.x.saturating_sub(1),
            y: head.y,
        });
        snake.push_back(TerminalSnakePoint {
            x: head.x.saturating_sub(2),
            y: head.y,
        });

        let mut game = Self {
            width,
            height,
            snake,
            direction: snake::SnakeDirection::Right,
            pending_direction: None,
            food: TerminalSnakePoint { x: 1, y: 1 },
            score: 0,
            rng: seed.max(1),
            game_over: false,
        };
        game.spawn_food();
        game
    }

    fn queue_direction(&mut self, direction: snake::SnakeDirection) {
        if self.pending_direction.is_some()
            || self.direction == direction
            || is_terminal_reverse(self.direction, direction)
        {
            return;
        }
        self.pending_direction = Some(direction);
    }

    fn step(&mut self) {
        if self.game_over {
            return;
        }

        if let Some(direction) = self.pending_direction.take() {
            self.direction = direction;
        }

        let Some(head) = self.snake.front().copied() else {
            self.game_over = true;
            return;
        };

        let next = match self.direction {
            snake::SnakeDirection::Up => TerminalSnakePoint {
                x: head.x,
                y: head.y.saturating_sub(1),
            },
            snake::SnakeDirection::Down => TerminalSnakePoint {
                x: head.x,
                y: head.y.saturating_add(1),
            },
            snake::SnakeDirection::Left => TerminalSnakePoint {
                x: head.x.saturating_sub(1),
                y: head.y,
            },
            snake::SnakeDirection::Right => TerminalSnakePoint {
                x: head.x.saturating_add(1),
                y: head.y,
            },
        };

        let ate_food = next == self.food;
        if !ate_food {
            self.snake.pop_back();
        }

        if next.x == 0
            || next.y == 0
            || next.x >= self.width - 1
            || next.y >= self.height - 1
            || self.snake.iter().any(|point| *point == next)
        {
            self.game_over = true;
            return;
        }

        self.snake.push_front(next);
        if ate_food {
            self.score = self.score.saturating_add(10);
            self.spawn_food();
        }
    }

    fn spawn_food(&mut self) {
        let inner_width = self.width.saturating_sub(2).max(1);
        let inner_height = self.height.saturating_sub(2).max(1);
        for _ in 0..512 {
            self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = 1 + (self.rng % u32::from(inner_width)) as u16;
            self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let y = 1 + (self.rng % u32::from(inner_height)) as u16;
            let candidate = TerminalSnakePoint { x, y };
            if !self.snake.iter().any(|point| *point == candidate) {
                self.food = candidate;
                return;
            }
        }
    }

    fn draw<W: Write>(&self, output: &mut W) -> io::Result<()> {
        queue!(output, MoveTo(0, 0))?;

        let head = self.snake.front().copied();
        for y in 0..self.height {
            for x in 0..self.width {
                let point = TerminalSnakePoint { x, y };
                if x == 0 || y == 0 || x == self.width - 1 || y == self.height - 1 {
                    queue!(
                        output,
                        SetForegroundColor(Color::White),
                        Print("#"),
                        ResetColor
                    )?;
                } else if Some(point) == head {
                    queue!(
                        output,
                        SetForegroundColor(Color::Green),
                        Print("O"),
                        ResetColor
                    )?;
                } else if self.snake.iter().skip(1).any(|body| *body == point) {
                    queue!(
                        output,
                        SetForegroundColor(Color::Green),
                        Print("o"),
                        ResetColor
                    )?;
                } else if point == self.food {
                    queue!(
                        output,
                        SetForegroundColor(Color::Red),
                        Print("*"),
                        ResetColor
                    )?;
                } else {
                    queue!(output, Print(" "))?;
                }
            }
            queue!(output, Print("\r\n"))?;
        }

        queue!(
            output,
            ResetColor,
            Print(format!(
                "Score: {}  WASD/arrows move, q quits{}",
                self.score,
                if self.game_over {
                    "  GAME OVER"
                } else {
                    "           "
                }
            ))
        )?;
        output.flush()
    }
}

fn is_terminal_reverse(current: snake::SnakeDirection, requested: snake::SnakeDirection) -> bool {
    matches!(
        (current, requested),
        (snake::SnakeDirection::Up, snake::SnakeDirection::Down)
            | (snake::SnakeDirection::Down, snake::SnakeDirection::Up)
            | (snake::SnakeDirection::Left, snake::SnakeDirection::Right)
            | (snake::SnakeDirection::Right, snake::SnakeDirection::Left)
    )
}

fn run_snake_terminal_loop<W>(
    model: &mut snake::SnakeModel,
    output: &mut W,
    tick_limit: snake::SnakeTickLimit,
    tick_interval: snake::SnakeTickInterval,
) -> Result<(), snake::SnakeTransitionError>
where
    W: Write,
{
    queue!(output, Clear(ClearType::All))
        .map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?;

    let interval = Duration::from_millis(u64::from(tick_interval.as_millis()));
    let mut last_tick = Instant::now();
    let mut ticks = snake::SnakeTick::new(0);
    let mut game = TerminalSnakeGame::new(model.board(), 1);
    game.draw(output)
        .map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?;

    while ticks.get() < u32::from(tick_limit.get()) {
        let timeout = interval
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout).map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?
            && let Event::Key(key) =
                event::read().map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?
            && let Some(action) = snake_action_from_key(key)
        {
            match action {
                snake::SnakeLocalAction::Quit => return Ok(()),
                snake::SnakeLocalAction::Turn(direction) => game.queue_direction(direction),
                snake::SnakeLocalAction::Reset => {
                    game = TerminalSnakeGame::new(model.board(), 1);
                    ticks = snake::SnakeTick::new(0);
                    last_tick = Instant::now();
                }
                snake::SnakeLocalAction::Pause
                | snake::SnakeLocalAction::Resume
                | snake::SnakeLocalAction::TogglePause
                | snake::SnakeLocalAction::Tick => {}
            }
        }

        if last_tick.elapsed() >= interval {
            game.step();
            ticks = snake::SnakeTick::new(ticks.get().wrapping_add(1));
            game.draw(output)
                .map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?;
            last_tick = Instant::now();
        }

        if game.game_over {
            loop {
                if event::poll(Duration::from_millis(50))
                    .map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?
                    && let Event::Key(key) =
                        event::read().map_err(|_| snake::SnakeTransitionError::AlreadyRunning)?
                    && matches!(key.code, KeyCode::Char('q' | 'Q') | KeyCode::Esc)
                {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

fn snake_action_from_key(key: KeyEvent) -> Option<snake::SnakeLocalAction> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match key.code {
        KeyCode::Char('q' | 'Q') | KeyCode::Esc => Some(snake::SnakeLocalAction::Quit),
        KeyCode::Char('r' | 'R') => Some(snake::SnakeLocalAction::Reset),
        KeyCode::Char(' ') => Some(snake::SnakeLocalAction::TogglePause),
        KeyCode::Char('p' | 'P') => Some(snake::SnakeLocalAction::Pause),
        KeyCode::Char('u' | 'U') => Some(snake::SnakeLocalAction::Resume),
        KeyCode::Char('w' | 'W') | KeyCode::Up => {
            Some(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Up))
        }
        KeyCode::Char('s' | 'S') | KeyCode::Down => {
            Some(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Down))
        }
        KeyCode::Char('a' | 'A') | KeyCode::Left => {
            Some(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Left))
        }
        KeyCode::Char('d' | 'D') | KeyCode::Right => {
            Some(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Right))
        }
        _ => None,
    }
}

fn loopback_target(
    address: Option<String>,
    device_name: Option<String>,
) -> loopback::LoopbackTarget {
    match (address, device_name) {
        (Some(address), _) => loopback::LoopbackTarget::addressed(address),
        (None, Some(device_name)) => loopback::LoopbackTarget::named(device_name),
        (None, None) => loopback::LoopbackTarget::default(),
    }
}

fn run_package_install(command: PackageInstallCommand) -> ExitCode {
    let package = match package_install::read_package_from_path(&command.package_path) {
        Ok(package) => package,
        Err(error) => {
            eprintln!("failed to read package {}: {error}", command.package_path);
            return ExitCode::from(1);
        }
    };

    let transport = match package_transport::BtlePackageInstallTransport::new() {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("failed to initialize package transport: {error}");
            return ExitCode::from(1);
        }
    };

    if let Err(error) = transport.open(loopback_target(command.address, command.device_name)) {
        eprintln!("failed to open package transport: {error}");
        return ExitCode::from(1);
    }

    match package_install::install_package(&package, &transport) {
        Ok(report) => {
            transport.close();
            println!(
                "package install ok for {}: {:?}",
                report.package_name, report.steps
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package install failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_package_erase(command: PackageEraseCommand) -> ExitCode {
    let transport = match package_transport::BtlePackageInstallTransport::new() {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("failed to initialize package transport: {error}");
            return ExitCode::from(1);
        }
    };

    if let Err(error) = transport.open(loopback_target(command.address, command.device_name)) {
        eprintln!("failed to open package transport: {error}");
        return ExitCode::from(1);
    }

    match package_install::erase_package(&transport) {
        Ok(report) => {
            transport.close();
            println!("package erase ok: {:?}", report.steps);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("package erase failed: {error}");
            ExitCode::from(1)
        }
    }
}

/// Parses command-line arguments into a top-level CLI command.
pub fn parse_args<I, S>(args: I) -> Result<Command, ParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());
    let _program = iter.next();

    match iter.next().as_deref() {
        None | Some("-h") | Some("--help") => Ok(Command::Help),
        Some("layout") => Ok(Command::Layout),
        Some("status") => Ok(Command::Status),
        Some("scan") => Ok(Command::Scan),
        Some("loopback") => parse_loopback(iter).map(Command::Loopback),
        Some("lisp-probe") => parse_lisp_probe(iter).map(Command::LispProbe),
        Some("package-install") => {
            parse_package_install(iter, "package-install").map(Command::PackageInstall)
        }
        Some("deploy") => parse_package_install(iter, "deploy").map(Command::Deploy),
        Some("snake-deploy") => {
            parse_package_install(iter, "snake-deploy").map(Command::SnakeDeploy)
        }
        Some("erase-package") => parse_erase_package(iter).map(Command::ErasePackage),
        Some("snake") => parse_snake(iter).map(Command::Snake),
        Some(other) => Err(ParseError::UnknownCommand(other.to_owned())),
    }
}

fn parse_device_flags(
    mut iter: impl Iterator<Item = String>,
) -> Result<(Option<String>, Option<String>), ParseError> {
    let mut device_name = None;
    let mut address = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok((device_name, address))
}

fn parse_loopback(iter: impl Iterator<Item = String>) -> Result<LoopbackCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(LoopbackCommand {
        device_name,
        address,
    })
}

fn parse_lisp_probe(iter: impl Iterator<Item = String>) -> Result<LispProbeCommand, ParseError> {
    let (device_name, address) = parse_device_flags(iter)?;

    Ok(LispProbeCommand {
        device_name,
        address,
    })
}

fn parse_package_install(
    mut iter: impl Iterator<Item = String>,
    command_name: &'static str,
) -> Result<PackageInstallCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;
    let mut package_path = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            _ if package_path.is_none() => package_path = Some(arg),
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    package_path
        .map(|package_path| PackageInstallCommand {
            package_path,
            device_name,
            address,
        })
        .ok_or_else(|| ParseError::UnknownCommand(command_name.to_owned()))
}

fn parse_erase_package(
    mut iter: impl Iterator<Item = String>,
) -> Result<PackageEraseCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok(PackageEraseCommand {
        device_name,
        address,
    })
}

fn parse_snake(mut iter: impl Iterator<Item = String>) -> Result<SnakeCommand, ParseError> {
    let mut device_name = None;
    let mut address = None;
    let mut board_width = snake::SnakeBoardWidth::new(24).expect("default width");
    let mut board_height = snake::SnakeBoardHeight::new(24).expect("default height");
    let mut seed = snake::SnakeSeed::new(1);
    let mut moves = Vec::new();
    let mut actions = Vec::new();
    let mut run_mode = SnakeRunMode::Interactive;
    let mut tick_limit = snake::SnakeTickLimit::new(1000).expect("default tick limit");
    let mut tick_interval = snake::SnakeTickInterval::new_millis(170).expect("default interval");

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--device" => {
                device_name = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--device".to_owned()))?,
                );
            }
            "--address" => {
                address = Some(
                    iter.next()
                        .ok_or_else(|| ParseError::UnknownCommand("--address".to_owned()))?,
                );
            }
            "--board" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--board".to_owned()))?;
                let (width, height) = parse_snake_board(&value)?;
                board_width = width;
                board_height = height;
            }
            "--seed" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--seed".to_owned()))?;
                seed = parse_snake_seed(&value)?;
            }
            "--moves" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--moves".to_owned()))?;
                moves = parse_snake_moves(&value)?;
                run_mode = SnakeRunMode::ScriptedMoves;
            }
            "--actions" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--actions".to_owned()))?;
                actions = parse_snake_actions(&value)?;
                run_mode = SnakeRunMode::ScriptedActions;
            }
            "--ticks" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--ticks".to_owned()))?;
                tick_limit = parse_snake_tick_limit(&value)?;
            }
            "--tick-ms" => {
                let value = iter
                    .next()
                    .ok_or_else(|| ParseError::UnknownCommand("--tick-ms".to_owned()))?;
                tick_interval = parse_snake_tick_interval(&value)?;
            }
            other => return Err(ParseError::UnknownCommand(other.to_owned())),
        }
    }

    Ok(SnakeCommand {
        device_name,
        address,
        board_width,
        board_height,
        seed,
        moves,
        actions,
        run_mode,
        tick_limit,
        tick_interval,
    })
}

fn parse_snake_board(
    value: &str,
) -> Result<(snake::SnakeBoardWidth, snake::SnakeBoardHeight), ParseError> {
    let Some((width, height)) = value.split_once('x') else {
        return Err(ParseError::UnknownCommand(value.to_owned()));
    };
    let width = width
        .parse::<u8>()
        .ok()
        .and_then(snake::SnakeBoardWidth::new)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))?;
    let height = height
        .parse::<u8>()
        .ok()
        .and_then(snake::SnakeBoardHeight::new)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))?;

    Ok((width, height))
}

fn parse_snake_seed(value: &str) -> Result<snake::SnakeSeed, ParseError> {
    value
        .parse::<u32>()
        .map(snake::SnakeSeed::new)
        .map_err(|_| ParseError::UnknownCommand(value.to_owned()))
}

fn parse_snake_moves(value: &str) -> Result<Vec<snake::SnakeDirection>, ParseError> {
    value
        .chars()
        .map(|ch| match ch {
            'w' | 'W' => Ok(snake::SnakeDirection::Up),
            'a' | 'A' => Ok(snake::SnakeDirection::Left),
            's' | 'S' => Ok(snake::SnakeDirection::Down),
            'd' | 'D' => Ok(snake::SnakeDirection::Right),
            _ => Err(ParseError::UnknownCommand(value.to_owned())),
        })
        .collect()
}

fn parse_snake_actions(value: &str) -> Result<Vec<snake::SnakeLocalAction>, ParseError> {
    value
        .chars()
        .map(|ch| match ch {
            'w' | 'W' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Up)),
            'a' | 'A' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Left)),
            's' | 'S' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Down)),
            'd' | 'D' => Ok(snake::SnakeLocalAction::Turn(snake::SnakeDirection::Right)),
            '.' => Ok(snake::SnakeLocalAction::Tick),
            'p' | 'P' => Ok(snake::SnakeLocalAction::Pause),
            'u' | 'U' => Ok(snake::SnakeLocalAction::Resume),
            'r' | 'R' => Ok(snake::SnakeLocalAction::Reset),
            'q' | 'Q' => Ok(snake::SnakeLocalAction::Quit),
            _ => Err(ParseError::UnknownCommand(value.to_owned())),
        })
        .collect()
}

fn parse_snake_tick_limit(value: &str) -> Result<snake::SnakeTickLimit, ParseError> {
    value
        .parse::<u16>()
        .ok()
        .and_then(snake::SnakeTickLimit::new)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))
}

fn parse_snake_tick_interval(value: &str) -> Result<snake::SnakeTickInterval, ParseError> {
    value
        .parse::<u16>()
        .ok()
        .and_then(snake::SnakeTickInterval::new_millis)
        .ok_or_else(|| ParseError::UnknownCommand(value.to_owned()))
}

mod ble_discovery;

/// BLE UART transport, discovery, and Lisp probe helpers.
pub mod btle;
pub mod deploy;
/// Loopback protocol runner and transport abstractions.
pub mod loopback;
pub mod loopback_debug;
pub mod package_install;
/// Package install transport implementations and BLE command helpers.
pub mod package_transport;
/// Deterministic snake game model for host-side tests and future UI wiring.
pub mod snake;
/// Snake package deploy and app-data smoke-test helper.
pub mod snake_deploy;
/// VESC UART packet encoding, decoding, and checksum helpers.
pub mod vesc_uart;

#[cfg(test)]
mod tests {
    use super::{
        Command, LispProbeCommand, LoopbackCommand, PackageEraseCommand, PackageInstallCommand,
        ParseError, SnakeCommand, SnakeRunMode, parse_args, snake_action_from_key,
    };
    use crate::snake::{
        SnakeBoardHeight, SnakeBoardWidth, SnakeDirection, SnakeLocalAction, SnakeSeed,
        SnakeTickInterval, SnakeTickLimit,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use vesc_protocol::{WireCommand, WireVersion};

    #[test]
    fn parse_args_covers_cli_commands() {
        assert_eq!(parse_args(["cargo-vescpkg", "layout"]), Ok(Command::Layout));
        assert_eq!(parse_args(["cargo-vescpkg", "status"]), Ok(Command::Status));
        assert_eq!(parse_args(["cargo-vescpkg", "scan"]), Ok(Command::Scan));
        assert_eq!(parse_args(["cargo-vescpkg"]), Ok(Command::Help));
        assert_eq!(
            parse_args(["cargo-vescpkg", "spoon"]),
            Err(ParseError::UnknownCommand("spoon".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "loopback"]),
            Ok(Command::Loopback(LoopbackCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "loopback", "--device", "Floatwheel PintV"]),
            Ok(Command::Loopback(LoopbackCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "lisp-probe"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "lisp-probe", "--device", "VESC BLE UART"]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "lisp-probe",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::LispProbe(LispProbeCommand {
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "package-install", "foo.vescpkg"]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "package-install",
                "--device",
                "Floatwheel PintV",
                "foo.vescpkg"
            ]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "package-install",
                "--address",
                "AA:BB:CC:DD:EE:FF",
                "foo.vescpkg"
            ]),
            Ok(Command::PackageInstall(PackageInstallCommand {
                package_path: "foo.vescpkg".to_owned(),
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "snake-deploy", "snake.vescpkg"]),
            Ok(Command::SnakeDeploy(PackageInstallCommand {
                package_path: "snake.vescpkg".to_owned(),
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "snake-deploy",
                "--device",
                "VESC BLE UART",
                "snake.vescpkg"
            ]),
            Ok(Command::SnakeDeploy(PackageInstallCommand {
                package_path: "snake.vescpkg".to_owned(),
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "snake-deploy"]),
            Err(ParseError::UnknownCommand("snake-deploy".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package"]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: None,
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--device",
                "Floatwheel PintV"
            ]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: None,
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: None,
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase"]),
            Err(ParseError::UnknownCommand("erase".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--force"]),
            Err(ParseError::UnknownCommand("--force".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--device"]),
            Err(ParseError::UnknownCommand("--device".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "--address"]),
            Err(ParseError::UnknownCommand("--address".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "erase-package", "extra.vescpkg"]),
            Err(ParseError::UnknownCommand("extra.vescpkg".to_owned()))
        );
        assert_eq!(
            parse_args(["cargo-vescpkg", "snake"]),
            Ok(Command::Snake(SnakeCommand {
                device_name: None,
                address: None,
                board_width: SnakeBoardWidth::new(24).expect("width"),
                board_height: SnakeBoardHeight::new(24).expect("height"),
                seed: SnakeSeed::new(1),
                moves: Vec::new(),
                actions: Vec::new(),
                run_mode: SnakeRunMode::Interactive,
                tick_limit: SnakeTickLimit::new(1000).expect("tick limit"),
                tick_interval: SnakeTickInterval::new_millis(170).expect("tick interval"),
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "snake",
                "--device",
                "VESC BLE UART",
                "--board",
                "12x8",
                "--seed",
                "99",
                "--moves",
                "sa",
                "--ticks",
                "2",
                "--tick-ms",
                "150"
            ]),
            Ok(Command::Snake(SnakeCommand {
                device_name: Some("VESC BLE UART".to_owned()),
                address: None,
                board_width: SnakeBoardWidth::new(12).expect("width"),
                board_height: SnakeBoardHeight::new(8).expect("height"),
                seed: SnakeSeed::new(99),
                moves: vec![SnakeDirection::Down, SnakeDirection::Left],
                actions: Vec::new(),
                run_mode: SnakeRunMode::ScriptedMoves,
                tick_limit: SnakeTickLimit::new(2).expect("tick limit"),
                tick_interval: SnakeTickInterval::new_millis(150).expect("tick interval"),
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "snake",
                "--board",
                "12x8",
                "--actions",
                "s.pu.rq",
                "--ticks",
                "12"
            ]),
            Ok(Command::Snake(SnakeCommand {
                device_name: None,
                address: None,
                board_width: SnakeBoardWidth::new(12).expect("width"),
                board_height: SnakeBoardHeight::new(8).expect("height"),
                seed: SnakeSeed::new(1),
                moves: Vec::new(),
                actions: vec![
                    SnakeLocalAction::Turn(SnakeDirection::Down),
                    SnakeLocalAction::Tick,
                    SnakeLocalAction::Pause,
                    SnakeLocalAction::Resume,
                    SnakeLocalAction::Tick,
                    SnakeLocalAction::Reset,
                    SnakeLocalAction::Quit,
                ],
                run_mode: SnakeRunMode::ScriptedActions,
                tick_limit: SnakeTickLimit::new(12).expect("tick limit"),
                tick_interval: SnakeTickInterval::new_millis(170).expect("tick interval"),
            }))
        );
        assert_eq!(
            parse_args([
                "cargo-vescpkg",
                "erase-package",
                "--device",
                "Floatwheel PintV",
                "--address",
                "AA:BB:CC:DD:EE:FF"
            ]),
            Ok(Command::ErasePackage(PackageEraseCommand {
                device_name: Some("Floatwheel PintV".to_owned()),
                address: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            }))
        );

        assert_eq!(WireVersion::CURRENT.get(), 1);
        assert_eq!(u8::from(WireCommand::Status), 3);
    }

    #[test]
    fn snake_terminal_key_mapping_matches_real_game_controls() {
        assert_eq!(
            snake_action_from_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE)),
            Some(SnakeLocalAction::Turn(SnakeDirection::Up))
        );
        assert_eq!(
            snake_action_from_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(SnakeLocalAction::Turn(SnakeDirection::Left))
        );
        assert_eq!(
            snake_action_from_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(SnakeLocalAction::TogglePause)
        );
        assert_eq!(
            snake_action_from_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(SnakeLocalAction::Quit)
        );
        assert_eq!(
            snake_action_from_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            snake_action_from_key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Repeat,
                state: KeyEventState::NONE,
            }),
            None
        );
    }
}
