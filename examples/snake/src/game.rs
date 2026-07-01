//! Strongly typed no_std Snake game state for the package-side example.

/// Board width in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeBoardWidth(u8);

impl SnakeBoardWidth {
    /// Create a non-zero board width.
    pub const fn new(value: u8) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    /// Extract the width in cells.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Board height in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeBoardHeight(u8);

impl SnakeBoardHeight {
    /// Create a non-zero board height.
    pub const fn new(value: u8) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    /// Extract the height in cells.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Board dimensions for package-side Snake state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeBoard {
    width: SnakeBoardWidth,
    height: SnakeBoardHeight,
}

impl SnakeBoard {
    /// Create a board from typed dimensions.
    pub const fn new(width: SnakeBoardWidth, height: SnakeBoardHeight) -> Self {
        Self { width, height }
    }

    /// Board width.
    pub const fn width(self) -> SnakeBoardWidth {
        self.width
    }

    /// Board height.
    pub const fn height(self) -> SnakeBoardHeight {
        self.height
    }
}

/// Deterministic game seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeSeed(u32);

impl SnakeSeed {
    /// Create a deterministic game seed.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Extract the raw seed value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A single board cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeCell {
    x: u8,
    y: u8,
}

impl SnakeCell {
    /// Create a board cell.
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    /// X coordinate.
    pub const fn x(self) -> u8 {
        self.x
    }

    /// Y coordinate.
    pub const fn y(self) -> u8 {
        self.y
    }
}

/// Current package-side snake direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeDirection {
    /// Move toward smaller Y values.
    Up,
    /// Move toward larger Y values.
    Down,
    /// Move toward smaller X values.
    Left,
    /// Move toward larger X values.
    Right,
}

/// Tick counter for the package-side game loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeTick(u32);

impl SnakeTick {
    /// Create a typed tick count.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Extract the raw tick count.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Score counter for the package-side game loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeScore(u16);

impl SnakeScore {
    /// Create a typed score.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Extract the raw score.
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Current package-side game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeState {
    /// Game is advancing on ticks.
    Running,
    /// Game is paused.
    Paused,
    /// Game ended.
    GameOver,
}

/// Invalid state transition requested by the package or host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeError {
    /// Requested turn directly reverses the current direction.
    ReverseTurn {
        /// Current direction.
        current: SnakeDirection,
        /// Requested direction.
        requested: SnakeDirection,
    },
    /// Requested transition requires a running game.
    NotRunning,
}

/// Outcome of advancing the package-side game by one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeStep {
    /// The snake moved one cell.
    Advanced,
    /// The game was paused and did not advance.
    Paused,
    /// The game is over.
    GameOver,
}

/// Deterministic package-side game model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeGame {
    board: SnakeBoard,
    seed: SnakeSeed,
    state: SnakeState,
    direction: SnakeDirection,
    pending_direction: Option<SnakeDirection>,
    tick: SnakeTick,
    score: SnakeScore,
    head: SnakeCell,
}

impl SnakeGame {
    /// Create a deterministic package-side game model.
    pub const fn new(board: SnakeBoard, seed: SnakeSeed) -> Self {
        Self {
            board,
            seed,
            state: SnakeState::Running,
            direction: SnakeDirection::Right,
            pending_direction: None,
            tick: SnakeTick::new(0),
            score: SnakeScore::new(0),
            head: SnakeCell::new(board.width().get() / 2, board.height().get() / 2),
        }
    }

    /// Current board.
    pub const fn board(self) -> SnakeBoard {
        self.board
    }

    /// Initial seed.
    pub const fn seed(self) -> SnakeSeed {
        self.seed
    }

    /// Current game state.
    pub const fn state(self) -> SnakeState {
        self.state
    }

    /// Current direction.
    pub const fn direction(self) -> SnakeDirection {
        self.direction
    }

    /// Current tick.
    pub const fn tick(self) -> SnakeTick {
        self.tick
    }

    /// Current score.
    pub const fn score(self) -> SnakeScore {
        self.score
    }

    /// Current head cell.
    pub const fn head(self) -> SnakeCell {
        self.head
    }

    /// Request a direction for the next tick.
    pub fn request_direction(&mut self, direction: SnakeDirection) -> Result<(), SnakeError> {
        if self.state != SnakeState::Running {
            return Err(SnakeError::NotRunning);
        }

        if is_reverse(self.direction, direction) {
            return Err(SnakeError::ReverseTurn {
                current: self.direction,
                requested: direction,
            });
        }

        self.pending_direction = Some(direction);
        Ok(())
    }

    /// Pause the game.
    pub fn pause(&mut self) -> Result<(), SnakeError> {
        if self.state != SnakeState::Running {
            return Err(SnakeError::NotRunning);
        }
        self.state = SnakeState::Paused;
        Ok(())
    }

    /// Resume the game.
    pub fn resume(&mut self) -> Result<(), SnakeError> {
        if self.state != SnakeState::Paused {
            return Err(SnakeError::NotRunning);
        }
        self.state = SnakeState::Running;
        Ok(())
    }

    /// Reset the game to its deterministic initial state.
    pub fn reset(&mut self) {
        *self = Self::new(self.board, self.seed);
    }

    /// Advance the game by one tick.
    pub fn advance(&mut self) -> SnakeStep {
        match self.state {
            SnakeState::Paused => return SnakeStep::Paused,
            SnakeState::GameOver => return SnakeStep::GameOver,
            SnakeState::Running => {}
        }

        if let Some(direction) = self.pending_direction.take() {
            self.direction = direction;
        }

        let next_head = step_head(self.head, self.direction);
        if next_head.x() >= self.board.width().get() || next_head.y() >= self.board.height().get() {
            self.state = SnakeState::GameOver;
            return SnakeStep::GameOver;
        }

        self.head = next_head;
        self.tick = SnakeTick::new(self.tick.get().wrapping_add(1));
        SnakeStep::Advanced
    }
}

fn is_reverse(current: SnakeDirection, requested: SnakeDirection) -> bool {
    matches!(
        (current, requested),
        (SnakeDirection::Up, SnakeDirection::Down)
            | (SnakeDirection::Down, SnakeDirection::Up)
            | (SnakeDirection::Left, SnakeDirection::Right)
            | (SnakeDirection::Right, SnakeDirection::Left)
    )
}

fn step_head(head: SnakeCell, direction: SnakeDirection) -> SnakeCell {
    match direction {
        SnakeDirection::Up => SnakeCell::new(head.x(), head.y().saturating_sub(1)),
        SnakeDirection::Down => SnakeCell::new(head.x(), head.y().saturating_add(1)),
        SnakeDirection::Left => SnakeCell::new(head.x().saturating_sub(1), head.y()),
        SnakeDirection::Right => SnakeCell::new(head.x().saturating_add(1), head.y()),
    }
}
