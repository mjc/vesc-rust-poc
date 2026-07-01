#![allow(missing_docs)]

/// Maximum number of cells the example snake can occupy.
pub const MAX_SNAKE_BODY_CELLS: usize = 256;

/// Board dimensions for the example snake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeBoardSize {
    width: u8,
    height: u8,
}

impl SnakeBoardSize {
    /// Create a board size.
    pub const fn new(width: u8, height: u8) -> Option<Self> {
        if width == 0 || height == 0 {
            None
        } else {
            Some(Self { width, height })
        }
    }

    /// Board width.
    pub const fn width(self) -> u8 {
        self.width
    }

    /// Board height.
    pub const fn height(self) -> u8 {
        self.height
    }
}

/// A single grid cell on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeCell {
    x: u8,
    y: u8,
}

impl SnakeCell {
    /// Create a new cell.
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

/// Current snake direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Current session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeSessionState {
    Idle,
    Running,
    Paused,
    GameOver,
}

/// Reason the snake stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeGameOverReason {
    WallCollision,
    SelfCollision,
    ProtocolError,
}

/// Tick counter for the example loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeTick(u32);

impl SnakeTick {
    /// Create a tick counter.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Extract the primitive tick value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Score counter for the example loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeScore(u16);

impl SnakeScore {
    /// Create a score counter.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Extract the primitive score value.
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Owned body cells, stored from head to tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeBody {
    cells: Vec<SnakeCell>,
}

impl SnakeBody {
    /// Create a bounded body snapshot.
    pub fn new(cells: &[SnakeCell]) -> Result<Self, SnakeModelError> {
        if cells.len() > MAX_SNAKE_BODY_CELLS {
            return Err(SnakeModelError::BodyTooLong {
                len: cells.len(),
                max: MAX_SNAKE_BODY_CELLS,
            });
        }

        Ok(Self {
            cells: cells.to_vec(),
        })
    }

    /// Borrow the occupied cells.
    pub fn cells(&self) -> &[SnakeCell] {
        &self.cells
    }
}

/// Snapshot of the board state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeBoardSnapshot {
    board: SnakeBoardSize,
    head: SnakeCell,
    food: SnakeCell,
    body: SnakeBody,
}

impl SnakeBoardSnapshot {
    /// Create a board snapshot.
    pub fn new(board: SnakeBoardSize, head: SnakeCell, food: SnakeCell, body: SnakeBody) -> Self {
        Self {
            board,
            head,
            food,
            body,
        }
    }

    /// Board size.
    pub const fn board(&self) -> SnakeBoardSize {
        self.board
    }

    /// Snake head position.
    pub const fn head(&self) -> SnakeCell {
        self.head
    }

    /// Food position.
    pub const fn food(&self) -> SnakeCell {
        self.food
    }

    /// Snake body cells.
    pub fn body(&self) -> &[SnakeCell] {
        self.body.cells()
    }
}

/// Full snapshot for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeSnapshot {
    board: SnakeBoardSnapshot,
    state: SnakeSessionState,
    tick: SnakeTick,
    score: SnakeScore,
    direction: SnakeDirection,
}

impl SnakeSnapshot {
    /// Create a full snapshot.
    pub fn new(
        board: SnakeBoardSnapshot,
        state: SnakeSessionState,
        tick: SnakeTick,
        score: SnakeScore,
        direction: SnakeDirection,
    ) -> Self {
        Self {
            board,
            state,
            tick,
            score,
            direction,
        }
    }

    /// Board state.
    pub const fn board(&self) -> &SnakeBoardSnapshot {
        &self.board
    }

    /// Session state.
    pub const fn state(&self) -> SnakeSessionState {
        self.state
    }

    /// Tick counter.
    pub const fn tick(&self) -> SnakeTick {
        self.tick
    }

    /// Score counter.
    pub const fn score(&self) -> SnakeScore {
        self.score
    }

    /// Heading direction.
    pub const fn direction(&self) -> SnakeDirection {
        self.direction
    }
}

/// Error returned when a transition is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeTransitionError {
    ReverseTurn {
        current: SnakeDirection,
        requested: SnakeDirection,
    },
    AlreadyPaused,
    AlreadyRunning,
    GameOver(SnakeGameOverReason),
}

/// Result of advancing the model by one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeStepOutcome {
    Advanced,
    AteFood,
    GameOver(SnakeGameOverReason),
    Paused,
}

/// Error returned when constructing or snapshotting the example model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeModelError {
    BoardTooSmall,
    BodyTooLong { len: usize, max: usize },
}

#[derive(Debug, Clone, Copy)]
struct SnakeRng(u32);

impl SnakeRng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn reseed(&mut self, seed: u32) {
        self.0 = seed;
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn next_index(&mut self, limit: usize) -> usize {
        (self.next_u32() as usize) % limit
    }
}

/// Deterministic host-side snake model used by tests and future UI wiring.
#[derive(Debug, Clone)]
pub struct SnakeModel {
    board: SnakeBoardSize,
    seed: u32,
    rng: SnakeRng,
    state: SnakeSessionState,
    direction: SnakeDirection,
    pending_direction: Option<SnakeDirection>,
    game_over_reason: Option<SnakeGameOverReason>,
    tick: SnakeTick,
    score: SnakeScore,
    head: SnakeCell,
    food: SnakeCell,
    body: Vec<SnakeCell>,
}

impl SnakeModel {
    /// Create a new running snake model with a deterministic seed.
    pub fn new(board: SnakeBoardSize, seed: u32) -> Self {
        let mut model = Self {
            board,
            seed,
            rng: SnakeRng::new(seed),
            state: SnakeSessionState::Running,
            direction: SnakeDirection::Right,
            pending_direction: None,
            game_over_reason: None,
            tick: SnakeTick::new(0),
            score: SnakeScore::new(0),
            head: SnakeCell::new(0, 0),
            food: SnakeCell::new(0, 0),
            body: Vec::new(),
        };
        model.reset_internal(seed);
        model
    }

    /// Current board dimensions.
    pub const fn board(&self) -> SnakeBoardSize {
        self.board
    }

    /// Current session state.
    pub const fn state(&self) -> SnakeSessionState {
        self.state
    }

    /// Current heading direction.
    pub const fn direction(&self) -> SnakeDirection {
        self.direction
    }

    /// Current score.
    pub const fn score(&self) -> SnakeScore {
        self.score
    }

    /// Current tick counter.
    pub const fn tick(&self) -> SnakeTick {
        self.tick
    }

    /// Current snake head position.
    pub const fn head(&self) -> SnakeCell {
        self.head
    }

    /// Current food position.
    pub const fn food(&self) -> SnakeCell {
        self.food
    }

    /// Borrow the occupied body cells from head to tail.
    pub fn body(&self) -> &[SnakeCell] {
        &self.body
    }

    /// Return a snapshot for rendering.
    pub fn snapshot(&self) -> Result<SnakeSnapshot, SnakeModelError> {
        let body = SnakeBody::new(self.body())?;
        Ok(SnakeSnapshot::new(
            SnakeBoardSnapshot::new(self.board, self.head, self.food, body),
            self.state,
            self.tick,
            self.score,
            self.direction,
        ))
    }

    /// Request a new heading direction.
    pub fn set_direction(&mut self, direction: SnakeDirection) -> Result<(), SnakeTransitionError> {
        if self.state == SnakeSessionState::GameOver {
            return Err(SnakeTransitionError::GameOver(
                self.game_over_reason
                    .unwrap_or(SnakeGameOverReason::ProtocolError),
            ));
        }

        if is_reverse(self.direction, direction) {
            return Err(SnakeTransitionError::ReverseTurn {
                current: self.direction,
                requested: direction,
            });
        }

        self.pending_direction = Some(direction);
        Ok(())
    }

    /// Pause the simulation.
    pub fn pause(&mut self) -> Result<(), SnakeTransitionError> {
        match self.state {
            SnakeSessionState::Running => {
                self.state = SnakeSessionState::Paused;
                Ok(())
            }
            SnakeSessionState::Paused | SnakeSessionState::Idle => {
                Err(SnakeTransitionError::AlreadyPaused)
            }
            SnakeSessionState::GameOver => Err(SnakeTransitionError::GameOver(
                self.game_over_reason
                    .unwrap_or(SnakeGameOverReason::ProtocolError),
            )),
        }
    }

    /// Resume a paused simulation.
    pub fn resume(&mut self) -> Result<(), SnakeTransitionError> {
        match self.state {
            SnakeSessionState::Paused => {
                self.state = SnakeSessionState::Running;
                Ok(())
            }
            SnakeSessionState::Running | SnakeSessionState::Idle => {
                Err(SnakeTransitionError::AlreadyRunning)
            }
            SnakeSessionState::GameOver => Err(SnakeTransitionError::GameOver(
                self.game_over_reason
                    .unwrap_or(SnakeGameOverReason::ProtocolError),
            )),
        }
    }

    /// Reset the simulation to its initial deterministic state.
    pub fn reset(&mut self) {
        self.reset_internal(self.seed);
    }

    /// Advance the simulation by one tick.
    pub fn advance(&mut self) -> SnakeStepOutcome {
        match self.state {
            SnakeSessionState::Paused | SnakeSessionState::Idle => {
                return SnakeStepOutcome::Paused;
            }
            SnakeSessionState::GameOver => {
                return SnakeStepOutcome::GameOver(
                    self.game_over_reason
                        .unwrap_or(SnakeGameOverReason::ProtocolError),
                );
            }
            SnakeSessionState::Running => {}
        }

        if let Some(direction) = self.pending_direction.take() {
            self.direction = direction;
        }

        let next_head = step_head(self.head, self.direction);
        if !within_board(self.board, next_head) {
            self.end_game(SnakeGameOverReason::WallCollision);
            return SnakeStepOutcome::GameOver(SnakeGameOverReason::WallCollision);
        }

        if self.body.contains(&next_head) {
            self.end_game(SnakeGameOverReason::SelfCollision);
            return SnakeStepOutcome::GameOver(SnakeGameOverReason::SelfCollision);
        }

        self.body.insert(0, next_head);
        self.head = next_head;

        if next_head == self.food {
            self.score = SnakeScore::new(self.score.get().saturating_add(1));
            self.tick = SnakeTick::new(self.tick.get().wrapping_add(1));
            self.food = self.next_food();
            SnakeStepOutcome::AteFood
        } else {
            if self.body.len() > 1 {
                self.body.pop();
            }
            self.tick = SnakeTick::new(self.tick.get().wrapping_add(1));
            SnakeStepOutcome::Advanced
        }
    }

    fn reset_internal(&mut self, seed: u32) {
        self.seed = seed;
        self.rng.reseed(seed);
        self.state = SnakeSessionState::Running;
        self.direction = SnakeDirection::Right;
        self.pending_direction = None;
        self.game_over_reason = None;
        self.tick = SnakeTick::new(0);
        self.score = SnakeScore::new(0);
        self.body.clear();
        self.head = SnakeCell::new(self.board.width() / 2, self.board.height() / 2);
        self.body.push(self.head);
        self.food = self.next_food();
    }

    fn end_game(&mut self, reason: SnakeGameOverReason) {
        self.state = SnakeSessionState::GameOver;
        self.game_over_reason = Some(reason);
    }

    fn next_food(&mut self) -> SnakeCell {
        let area = usize::from(self.board.width()) * usize::from(self.board.height());
        if area <= self.body.len() {
            return self.head;
        }

        loop {
            let index = self.rng.next_index(area);
            let x = (index % usize::from(self.board.width())) as u8;
            let y = (index / usize::from(self.board.width())) as u8;
            let candidate = SnakeCell::new(x, y);
            if !self.body.contains(&candidate) {
                return candidate;
            }
        }
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

fn within_board(board: SnakeBoardSize, cell: SnakeCell) -> bool {
    cell.x() < board.width() && cell.y() < board.height()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_model_is_deterministic() {
        let board = SnakeBoardSize::new(8, 6).expect("board");
        let a = SnakeModel::new(board, 42);
        let b = SnakeModel::new(board, 42);

        assert_eq!(a.state(), SnakeSessionState::Running);
        assert_eq!(a.direction(), SnakeDirection::Right);
        assert_eq!(a.head(), SnakeCell::new(4, 3));
        assert_eq!(a.body(), &[SnakeCell::new(4, 3)]);
        assert_eq!(a.food(), b.food());
    }

    #[test]
    fn eating_food_grows_and_scores() {
        let board = SnakeBoardSize::new(6, 6).expect("board");
        let mut model = SnakeModel::new(board, 7);
        model.body = vec![SnakeCell::new(2, 2)];
        model.head = SnakeCell::new(2, 2);
        model.food = SnakeCell::new(3, 2);
        model.direction = SnakeDirection::Right;

        assert_eq!(model.advance(), SnakeStepOutcome::AteFood);
        assert_eq!(model.head(), SnakeCell::new(3, 2));
        assert_eq!(model.score(), SnakeScore::new(1));
        assert_eq!(model.body(), &[SnakeCell::new(3, 2), SnakeCell::new(2, 2)]);
    }

    #[test]
    fn rejects_reverse_turns_and_pause_halts() {
        let board = SnakeBoardSize::new(6, 6).expect("board");
        let mut model = SnakeModel::new(board, 11);

        assert_eq!(
            model.set_direction(SnakeDirection::Left),
            Err(SnakeTransitionError::ReverseTurn {
                current: SnakeDirection::Right,
                requested: SnakeDirection::Left
            })
        );

        model.pause().expect("pause");
        assert_eq!(model.advance(), SnakeStepOutcome::Paused);
        model.resume().expect("resume");
        assert_eq!(model.state(), SnakeSessionState::Running);
    }

    #[test]
    fn wall_and_self_collision_end_game() {
        let board = SnakeBoardSize::new(4, 4).expect("board");
        let mut model = SnakeModel::new(board, 1);
        model.body = vec![SnakeCell::new(3, 1)];
        model.head = SnakeCell::new(3, 1);
        model.direction = SnakeDirection::Right;
        model.food = SnakeCell::new(0, 0);

        assert_eq!(
            model.advance(),
            SnakeStepOutcome::GameOver(SnakeGameOverReason::WallCollision)
        );

        let mut model = SnakeModel::new(board, 2);
        model.body = vec![
            SnakeCell::new(2, 2),
            SnakeCell::new(1, 2),
            SnakeCell::new(1, 3),
            SnakeCell::new(2, 3),
            SnakeCell::new(2, 2),
        ];
        model.head = SnakeCell::new(2, 2);
        model.direction = SnakeDirection::Left;
        model.food = SnakeCell::new(0, 0);

        assert_eq!(
            model.advance(),
            SnakeStepOutcome::GameOver(SnakeGameOverReason::SelfCollision)
        );
    }

    #[test]
    fn snapshot_reflects_current_state() {
        let board = SnakeBoardSize::new(8, 6).expect("board");
        let mut model = SnakeModel::new(board, 99);
        model.advance();
        let snapshot = model.snapshot().expect("snapshot");

        assert_eq!(snapshot.board().board(), board);
        assert_eq!(snapshot.state(), SnakeSessionState::Running);
        assert_eq!(snapshot.direction(), SnakeDirection::Right);
        assert_eq!(snapshot.tick(), model.tick());
        assert_eq!(snapshot.score(), model.score());
    }
}
