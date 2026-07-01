use core::fmt;

/// Wire protocol version used by snake packets.
pub const SNAKE_PROTOCOL_VERSION: SnakeVersion = SnakeVersion::CURRENT;
/// Maximum number of body cells carried in a snapshot.
pub const MAX_SNAKE_BODY_CELLS: usize = 64;
/// Size of the fixed snake packet header in bytes.
pub const MIN_SNAKE_PACKET_BYTES: usize = 4;
/// Maximum encoded snapshot payload in bytes.
pub const MAX_SNAKE_SNAPSHOT_PAYLOAD_BYTES: usize =
    1 + 4 + 2 + 2 + 2 + 2 + 1 + 1 + (2 * MAX_SNAKE_BODY_CELLS);
/// Maximum encoded snake packet size in bytes.
pub const MAX_SNAKE_PACKET_BYTES: usize = MIN_SNAKE_PACKET_BYTES + MAX_SNAKE_SNAPSHOT_PAYLOAD_BYTES;

/// Snake wire protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeVersion(u8);

impl SnakeVersion {
    /// Current snake protocol version.
    pub const CURRENT: Self = Self(1);

    /// Construct a version tag from the raw wire byte.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Explicitly extract the raw wire value.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl From<SnakeVersion> for u8 {
    fn from(version: SnakeVersion) -> Self {
        version.get()
    }
}

/// Packet kind on the snake wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnakePacketKind {
    Command,
    Message,
}

impl SnakePacketKind {
    const fn wire_value(self) -> u8 {
        match self {
            Self::Command => 1,
            Self::Message => 2,
        }
    }
}

impl TryFrom<u8> for SnakePacketKind {
    type Error = SnakeWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Command),
            2 => Ok(Self::Message),
            _ => Err(SnakeWireError::InvalidPacketKind { code: value }),
        }
    }
}

impl From<SnakePacketKind> for u8 {
    fn from(kind: SnakePacketKind) -> Self {
        kind.wire_value()
    }
}

/// Horizontal/vertical direction for snake motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeDirection {
    /// Move toward smaller Y.
    Up,
    /// Move toward larger Y.
    Down,
    /// Move toward smaller X.
    Left,
    /// Move toward larger X.
    Right,
}

impl SnakeDirection {
    const fn wire_value(self) -> u8 {
        match self {
            Self::Up => 1,
            Self::Down => 2,
            Self::Left => 3,
            Self::Right => 4,
        }
    }
}

impl TryFrom<u8> for SnakeDirection {
    type Error = SnakeWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Up),
            2 => Ok(Self::Down),
            3 => Ok(Self::Left),
            4 => Ok(Self::Right),
            _ => Err(SnakeWireError::InvalidDirection { code: value }),
        }
    }
}

impl From<SnakeDirection> for u8 {
    fn from(direction: SnakeDirection) -> Self {
        direction.wire_value()
    }
}

/// Current session state as reported by the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeSessionState {
    /// The session has not started yet.
    Idle,
    /// The game is actively advancing.
    Running,
    /// The game is paused.
    Paused,
    /// The game has ended.
    GameOver,
}

impl SnakeSessionState {
    const fn wire_value(self) -> u8 {
        match self {
            Self::Idle => 1,
            Self::Running => 2,
            Self::Paused => 3,
            Self::GameOver => 4,
        }
    }
}

impl TryFrom<u8> for SnakeSessionState {
    type Error = SnakeWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Idle),
            2 => Ok(Self::Running),
            3 => Ok(Self::Paused),
            4 => Ok(Self::GameOver),
            _ => Err(SnakeWireError::InvalidSessionState { code: value }),
        }
    }
}

impl From<SnakeSessionState> for u8 {
    fn from(state: SnakeSessionState) -> Self {
        state.wire_value()
    }
}

/// Board dimensions for a snake session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeBoardSize {
    width: u8,
    height: u8,
}

impl SnakeBoardSize {
    /// Create a new board size, rejecting zero-sized dimensions.
    pub const fn new(width: u8, height: u8) -> Result<Self, SnakeWireError> {
        if width == 0 || height == 0 {
            return Err(SnakeWireError::InvalidBoardSize { width, height });
        }

        Ok(Self { width, height })
    }

    /// Board width in cells.
    pub const fn width(self) -> u8 {
        self.width
    }

    /// Board height in cells.
    pub const fn height(self) -> u8 {
        self.height
    }
}

/// Snake cell coordinate within the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeCell {
    x: u8,
    y: u8,
}

impl SnakeCell {
    /// Construct a board cell.
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

/// Tick counter carried in snapshots and heartbeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeTick(u32);

impl SnakeTick {
    /// Create a tick count from the raw wire value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Raw tick count.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<SnakeTick> for u32 {
    fn from(tick: SnakeTick) -> Self {
        tick.get()
    }
}

/// Game score carried in snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeScore(u16);

impl SnakeScore {
    /// Create a score from the raw wire value.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Raw score value.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl From<SnakeScore> for u16 {
    fn from(score: SnakeScore) -> Self {
        score.get()
    }
}

/// Fixed-capacity snake body representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeBody {
    len: u8,
    cells: [SnakeCell; MAX_SNAKE_BODY_CELLS],
}

impl SnakeBody {
    /// Construct a body from an ordered slice of occupied cells.
    pub fn new(cells: &[SnakeCell]) -> Result<Self, SnakeWireError> {
        if cells.len() > MAX_SNAKE_BODY_CELLS {
            return Err(SnakeWireError::BodyTooLong {
                len: cells.len(),
                max: MAX_SNAKE_BODY_CELLS,
            });
        }

        let mut body = Self {
            len: cells.len() as u8,
            cells: [SnakeCell::new(0, 0); MAX_SNAKE_BODY_CELLS],
        };

        let mut index = 0;
        while index < cells.len() {
            body.cells[index] = cells[index];
            index += 1;
        }

        Ok(body)
    }

    /// Number of occupied cells.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// True when the body is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Borrow the occupied cells.
    pub fn cells(&self) -> &[SnakeCell] {
        &self.cells[..self.len()]
    }
}

/// Board-facing snapshot data: board size, head, food, and occupied cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeBoardSnapshot {
    board: SnakeBoardSize,
    head: SnakeCell,
    food: SnakeCell,
    body: SnakeBody,
}

impl SnakeBoardSnapshot {
    /// Construct a board snapshot.
    pub fn new(board: SnakeBoardSize, head: SnakeCell, food: SnakeCell, body: SnakeBody) -> Self {
        Self {
            board,
            head,
            food,
            body,
        }
    }

    /// Board dimensions.
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

    /// Occupied body cells.
    pub fn body(&self) -> &[SnakeCell] {
        self.body.cells()
    }
}

/// Device-side session snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeSnapshot {
    board: SnakeBoardSnapshot,
    state: SnakeSessionState,
    tick: SnakeTick,
    score: SnakeScore,
    direction: SnakeDirection,
}

impl SnakeSnapshot {
    /// Construct a new snapshot.
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

    /// Board snapshot data.
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

    /// Score value.
    pub const fn score(&self) -> SnakeScore {
        self.score
    }

    /// Current direction.
    pub const fn direction(&self) -> SnakeDirection {
        self.direction
    }
}

/// Host-to-device command values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeCommand {
    /// Capability advertisement and version negotiation.
    Hello {
        /// Minimum protocol version supported by the host.
        min_version: SnakeVersion,
        /// Maximum protocol version supported by the host.
        max_version: SnakeVersion,
        /// Host capability bitset.
        capabilities: SnakeCapabilities,
    },
    /// Start a new game with the provided seed and board size.
    Start {
        /// Seed used for deterministic food placement.
        seed: u32,
        /// Requested board size.
        board: SnakeBoardSize,
    },
    /// Pause the game.
    Pause,
    /// Resume the game.
    Resume,
    /// Reset the game state.
    Reset,
    /// Set the next heading direction.
    SetDirection(SnakeDirection),
}

/// Device-to-host message values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnakeMessage {
    /// Full state snapshot for rendering.
    Snapshot(SnakeSnapshot),
    /// State transition or event notification.
    Event(SnakeEvent),
    /// Typed protocol or session error.
    Error(SnakeFault),
    /// Liveness tick.
    Heartbeat(SnakeTick),
}

/// Event notifications carried from device to host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeEvent {
    /// The device accepted a start command.
    Started,
    /// The device entered a paused state.
    Paused,
    /// The device resumed running.
    Resumed,
    /// The snake ate a food cell.
    AteFood,
    /// The game ended with the given reason.
    GameOver(SnakeGameOverReason),
}

/// Reason a game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeGameOverReason {
    /// The snake hit a wall.
    WallCollision,
    /// The snake hit itself.
    SelfCollision,
    /// The host or device sent an invalid input.
    InvalidInput,
    /// The protocol/session entered an unrecoverable error state.
    ProtocolError,
}

impl SnakeGameOverReason {
    const fn wire_value(self) -> u8 {
        match self {
            Self::WallCollision => 1,
            Self::SelfCollision => 2,
            Self::InvalidInput => 3,
            Self::ProtocolError => 4,
        }
    }
}

impl TryFrom<u8> for SnakeGameOverReason {
    type Error = SnakeWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::WallCollision),
            2 => Ok(Self::SelfCollision),
            3 => Ok(Self::InvalidInput),
            4 => Ok(Self::ProtocolError),
            _ => Err(SnakeWireError::InvalidGameOverReason { code: value }),
        }
    }
}

impl From<SnakeGameOverReason> for u8 {
    fn from(reason: SnakeGameOverReason) -> Self {
        reason.wire_value()
    }
}

/// Typed error/fault notifications from the snake session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeFault {
    /// The host and device do not agree on the protocol version.
    UnsupportedVersion,
    /// The command opcode was not understood.
    InvalidCommand,
    /// The command was invalid for the current session state.
    InvalidState,
    /// The host and device got out of sync.
    Desync,
    /// The session is already closed.
    SessionClosed,
}

impl SnakeFault {
    const fn wire_value(self) -> u8 {
        match self {
            Self::UnsupportedVersion => 1,
            Self::InvalidCommand => 2,
            Self::InvalidState => 3,
            Self::Desync => 4,
            Self::SessionClosed => 5,
        }
    }
}

impl TryFrom<u8> for SnakeFault {
    type Error = SnakeWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::UnsupportedVersion),
            2 => Ok(Self::InvalidCommand),
            3 => Ok(Self::InvalidState),
            4 => Ok(Self::Desync),
            5 => Ok(Self::SessionClosed),
            _ => Err(SnakeWireError::InvalidFault { code: value }),
        }
    }
}

impl From<SnakeFault> for u8 {
    fn from(fault: SnakeFault) -> Self {
        fault.wire_value()
    }
}

/// Host capability bitset for the snake handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeCapabilities(u16);

impl SnakeCapabilities {
    /// Create a capability token from the raw wire value.
    pub const fn new(bits: u16) -> Self {
        Self(bits)
    }

    /// Raw capability bits.
    pub const fn bits(self) -> u16 {
        self.0
    }
}

impl From<SnakeCapabilities> for u16 {
    fn from(capabilities: SnakeCapabilities) -> Self {
        capabilities.bits()
    }
}

/// Errors returned when snake protocol encoding or decoding fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnakeWireError {
    /// The packet was shorter than the fixed header.
    PacketTooShort,
    /// The encoded version byte did not match the snake protocol version.
    InvalidVersion {
        /// Expected wire version.
        expected: SnakeVersion,
        /// Actual wire version found in the packet.
        actual: SnakeVersion,
    },
    /// The packet kind byte was not recognized.
    InvalidPacketKind {
        /// Unknown packet kind code.
        code: u8,
    },
    /// The command opcode was not recognized.
    InvalidCommand {
        /// Unknown opcode value.
        code: u8,
    },
    /// The message opcode was not recognized.
    InvalidMessage {
        /// Unknown opcode value.
        code: u8,
    },
    /// The packet payload exceeded the supported size.
    PayloadTooLong {
        /// Payload length from the wire.
        len: usize,
        /// Maximum supported payload length.
        max: usize,
    },
    /// The output buffer cannot hold the encoded packet.
    BufferTooShort {
        /// Provided output buffer length.
        len: usize,
        /// Required encoded packet length.
        required: usize,
    },
    /// The board dimensions were invalid.
    InvalidBoardSize {
        /// Requested width.
        width: u8,
        /// Requested height.
        height: u8,
    },
    /// The body slice exceeded the supported capacity.
    BodyTooLong {
        /// Requested body length.
        len: usize,
        /// Maximum body length.
        max: usize,
    },
    /// The session state opcode was not recognized.
    InvalidSessionState {
        /// Unknown state code.
        code: u8,
    },
    /// The direction opcode was not recognized.
    InvalidDirection {
        /// Unknown direction code.
        code: u8,
    },
    /// The game-over reason opcode was not recognized.
    InvalidGameOverReason {
        /// Unknown reason code.
        code: u8,
    },
    /// The fault opcode was not recognized.
    InvalidFault {
        /// Unknown fault code.
        code: u8,
    },
}

impl fmt::Display for SnakeWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PacketTooShort => f.write_str("packet too short"),
            Self::InvalidVersion { expected, actual } => {
                write!(
                    f,
                    "invalid snake protocol version: expected {}, got {}",
                    expected.get(),
                    actual.get()
                )
            }
            Self::InvalidPacketKind { code } => write!(f, "invalid packet kind: {code}"),
            Self::InvalidCommand { code } => write!(f, "invalid snake command opcode: {code}"),
            Self::InvalidMessage { code } => write!(f, "invalid snake message opcode: {code}"),
            Self::PayloadTooLong { len, max } => {
                write!(f, "payload too long: {len} bytes (max {max})")
            }
            Self::BufferTooShort { len, required } => {
                write!(f, "buffer too short: {len} bytes (need {required})")
            }
            Self::InvalidBoardSize { width, height } => {
                write!(f, "invalid board size: {width}x{height}")
            }
            Self::BodyTooLong { len, max } => {
                write!(f, "snake body too long: {len} cells (max {max})")
            }
            Self::InvalidSessionState { code } => write!(f, "invalid session state: {code}"),
            Self::InvalidDirection { code } => write!(f, "invalid direction: {code}"),
            Self::InvalidGameOverReason { code } => write!(f, "invalid game-over reason: {code}"),
            Self::InvalidFault { code } => write!(f, "invalid fault code: {code}"),
        }
    }
}

/// Borrowed snake command packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeCommandPacket {
    command: SnakeCommand,
}

impl SnakeCommandPacket {
    /// Construct a new command packet.
    pub const fn new(command: SnakeCommand) -> Self {
        Self { command }
    }

    /// Return the typed command.
    pub const fn command(&self) -> SnakeCommand {
        self.command
    }

    /// Encode the command into the provided output buffer.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, SnakeWireError> {
        let required = 4 + snake_command_payload_len(self.command);
        if out.len() < required {
            return Err(SnakeWireError::BufferTooShort {
                len: out.len(),
                required,
            });
        }

        out[0] = SNAKE_PROTOCOL_VERSION.get();
        out[1] = u8::from(SnakePacketKind::Command);
        out[2] = snake_command_opcode(self.command);

        let payload_len = write_snake_command_payload(self.command, &mut out[4..required]);
        out[3] = payload_len as u8;

        Ok(required)
    }

    /// Encode the command into a fixed-size byte buffer and its used length.
    pub fn encode(&self) -> ([u8; MAX_SNAKE_PACKET_BYTES], usize) {
        let mut bytes = [0_u8; MAX_SNAKE_PACKET_BYTES];
        let len = self.encode_into(&mut bytes).expect("packet fits");
        (bytes, len)
    }

    /// Decode a command packet from raw bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, SnakeWireError> {
        let (kind, opcode, payload) = decode_header(bytes)?;
        if kind != SnakePacketKind::Command {
            return Err(SnakeWireError::InvalidPacketKind {
                code: u8::from(kind),
            });
        }

        let command = read_snake_command(opcode, payload)?;
        Ok(Self { command })
    }
}

/// Borrowed snake message packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnakeMessagePacket {
    message: SnakeMessage,
}

impl SnakeMessagePacket {
    /// Construct a new message packet.
    pub fn new(message: SnakeMessage) -> Self {
        Self { message }
    }

    /// Return the typed message.
    pub fn message(&self) -> SnakeMessage {
        self.message.clone()
    }

    /// Encode the message into the provided output buffer.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, SnakeWireError> {
        let required = 4 + snake_message_payload_len(&self.message);
        if out.len() < required {
            return Err(SnakeWireError::BufferTooShort {
                len: out.len(),
                required,
            });
        }

        out[0] = SNAKE_PROTOCOL_VERSION.get();
        out[1] = u8::from(SnakePacketKind::Message);
        out[2] = snake_message_opcode(&self.message);

        let payload_len = write_snake_message_payload(&self.message, &mut out[4..required]);
        out[3] = payload_len as u8;

        Ok(required)
    }

    /// Encode the message into a fixed-size byte buffer and its used length.
    pub fn encode(&self) -> ([u8; MAX_SNAKE_PACKET_BYTES], usize) {
        let mut bytes = [0_u8; MAX_SNAKE_PACKET_BYTES];
        let len = self.encode_into(&mut bytes).expect("packet fits");
        (bytes, len)
    }

    /// Decode a message packet from raw bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, SnakeWireError> {
        let (kind, opcode, payload) = decode_header(bytes)?;
        if kind != SnakePacketKind::Message {
            return Err(SnakeWireError::InvalidPacketKind {
                code: u8::from(kind),
            });
        }

        let message = read_snake_message(opcode, payload)?;
        Ok(Self { message })
    }
}

fn decode_header(bytes: &[u8]) -> Result<(SnakePacketKind, u8, &[u8]), SnakeWireError> {
    if bytes.len() < MIN_SNAKE_PACKET_BYTES {
        return Err(SnakeWireError::PacketTooShort);
    }

    let actual = SnakeVersion::new(bytes[0]);
    if actual != SNAKE_PROTOCOL_VERSION {
        return Err(SnakeWireError::InvalidVersion {
            expected: SNAKE_PROTOCOL_VERSION,
            actual,
        });
    }

    let kind = SnakePacketKind::try_from(bytes[1])?;
    let opcode = bytes[2];
    let payload_len = bytes[3] as usize;
    let required = MIN_SNAKE_PACKET_BYTES + payload_len;
    if bytes.len() < required {
        return Err(SnakeWireError::PacketTooShort);
    }

    Ok((kind, opcode, &bytes[MIN_SNAKE_PACKET_BYTES..required]))
}

fn snake_command_opcode(command: SnakeCommand) -> u8 {
    match command {
        SnakeCommand::Hello { .. } => 1,
        SnakeCommand::Start { .. } => 2,
        SnakeCommand::Pause => 3,
        SnakeCommand::Resume => 4,
        SnakeCommand::Reset => 5,
        SnakeCommand::SetDirection(_) => 6,
    }
}

fn snake_command_payload_len(command: SnakeCommand) -> usize {
    match command {
        SnakeCommand::Hello { .. } => 4,
        SnakeCommand::Start { .. } => 6,
        SnakeCommand::Pause | SnakeCommand::Resume | SnakeCommand::Reset => 0,
        SnakeCommand::SetDirection(_) => 1,
    }
}

fn write_snake_command_payload(command: SnakeCommand, out: &mut [u8]) -> usize {
    match command {
        SnakeCommand::Hello {
            min_version,
            max_version,
            capabilities,
        } => {
            out[0] = min_version.get();
            out[1] = max_version.get();
            write_u16_le(&mut out[2..4], capabilities.bits());
            4
        }
        SnakeCommand::Start { seed, board } => {
            out[0] = board.width();
            out[1] = board.height();
            write_u32_le(&mut out[2..6], seed);
            6
        }
        SnakeCommand::Pause | SnakeCommand::Resume | SnakeCommand::Reset => 0,
        SnakeCommand::SetDirection(direction) => {
            out[0] = direction.into();
            1
        }
    }
}

fn read_snake_command(opcode: u8, payload: &[u8]) -> Result<SnakeCommand, SnakeWireError> {
    match opcode {
        1 => {
            if payload.len() != 4 {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: 4,
                });
            }

            Ok(SnakeCommand::Hello {
                min_version: SnakeVersion::new(payload[0]),
                max_version: SnakeVersion::new(payload[1]),
                capabilities: SnakeCapabilities::new(read_u16_le(&payload[2..4])),
            })
        }
        2 => {
            if payload.len() != 6 {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: 6,
                });
            }

            let board = SnakeBoardSize::new(payload[0], payload[1])?;
            Ok(SnakeCommand::Start {
                seed: read_u32_le(&payload[2..6]),
                board,
            })
        }
        3 => Ok(SnakeCommand::Pause),
        4 => Ok(SnakeCommand::Resume),
        5 => Ok(SnakeCommand::Reset),
        6 => {
            if payload.len() != 1 {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: 1,
                });
            }

            Ok(SnakeCommand::SetDirection(SnakeDirection::try_from(
                payload[0],
            )?))
        }
        _ => Err(SnakeWireError::InvalidCommand { code: opcode }),
    }
}

fn snake_message_opcode(message: &SnakeMessage) -> u8 {
    match message {
        SnakeMessage::Snapshot(_) => 1,
        SnakeMessage::Event(_) => 2,
        SnakeMessage::Error(_) => 3,
        SnakeMessage::Heartbeat(_) => 4,
    }
}

fn snake_message_payload_len(message: &SnakeMessage) -> usize {
    match message {
        SnakeMessage::Snapshot(snapshot) => {
            1 + 4 + 2 + 2 + 2 + 2 + 1 + 1 + (snapshot.board().body().len() * 2)
        }
        SnakeMessage::Event(event) => match event {
            SnakeEvent::GameOver(_) => 2,
            SnakeEvent::Started
            | SnakeEvent::Paused
            | SnakeEvent::Resumed
            | SnakeEvent::AteFood => 1,
        },
        SnakeMessage::Error(_) => 1,
        SnakeMessage::Heartbeat(_) => 4,
    }
}

fn write_snake_message_payload(message: &SnakeMessage, out: &mut [u8]) -> usize {
    match message {
        SnakeMessage::Snapshot(snapshot) => {
            out[0] = u8::from(snapshot.state());
            write_u32_le(&mut out[1..5], snapshot.tick().get());
            write_u16_le(&mut out[5..7], snapshot.score().get());
            out[7] = snapshot.board().board().width();
            out[8] = snapshot.board().board().height();
            out[9] = snapshot.board().head().x();
            out[10] = snapshot.board().head().y();
            out[11] = snapshot.board().food().x();
            out[12] = snapshot.board().food().y();
            out[13] = u8::from(snapshot.direction());
            out[14] = snapshot.board().body().len() as u8;

            let mut index = 0;
            while index < snapshot.board().body().len() {
                let cell = snapshot.board().body()[index];
                let offset = 15 + (index * 2);
                out[offset] = cell.x();
                out[offset + 1] = cell.y();
                index += 1;
            }

            15 + snapshot.board().body().len() * 2
        }
        SnakeMessage::Event(event) => match event {
            SnakeEvent::Started => {
                out[0] = 1;
                1
            }
            SnakeEvent::Paused => {
                out[0] = 2;
                1
            }
            SnakeEvent::Resumed => {
                out[0] = 3;
                1
            }
            SnakeEvent::AteFood => {
                out[0] = 4;
                1
            }
            SnakeEvent::GameOver(reason) => {
                out[0] = 5;
                out[1] = (*reason).into();
                2
            }
        },
        SnakeMessage::Error(fault) => {
            out[0] = (*fault).into();
            1
        }
        SnakeMessage::Heartbeat(tick) => {
            write_u32_le(&mut out[..4], tick.get());
            4
        }
    }
}

fn read_snake_message(opcode: u8, payload: &[u8]) -> Result<SnakeMessage, SnakeWireError> {
    match opcode {
        1 => {
            if payload.len() < 15 {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: 15,
                });
            }

            let state = SnakeSessionState::try_from(payload[0])?;
            let tick = SnakeTick::new(read_u32_le(&payload[1..5]));
            let score = SnakeScore::new(read_u16_le(&payload[5..7]));
            let board = SnakeBoardSize::new(payload[7], payload[8])?;
            let head = SnakeCell::new(payload[9], payload[10]);
            let food = SnakeCell::new(payload[11], payload[12]);
            let direction = SnakeDirection::try_from(payload[13])?;
            let body_len = payload[14] as usize;
            let required = 15 + body_len * 2;
            if body_len > MAX_SNAKE_BODY_CELLS {
                return Err(SnakeWireError::BodyTooLong {
                    len: body_len,
                    max: MAX_SNAKE_BODY_CELLS,
                });
            }
            if payload.len() < required {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: required,
                });
            }

            let mut cells = [SnakeCell::new(0, 0); MAX_SNAKE_BODY_CELLS];
            let mut index = 0;
            while index < body_len {
                let offset = 15 + (index * 2);
                cells[index] = SnakeCell::new(payload[offset], payload[offset + 1]);
                index += 1;
            }

            let body = SnakeBody {
                len: body_len as u8,
                cells,
            };
            let board = SnakeBoardSnapshot::new(board, head, food, body);

            Ok(SnakeMessage::Snapshot(SnakeSnapshot::new(
                board, state, tick, score, direction,
            )))
        }
        2 => {
            if payload.is_empty() {
                return Err(SnakeWireError::PacketTooShort);
            }

            let event = match payload[0] {
                1 => SnakeEvent::Started,
                2 => SnakeEvent::Paused,
                3 => SnakeEvent::Resumed,
                4 => SnakeEvent::AteFood,
                5 => {
                    if payload.len() != 2 {
                        return Err(SnakeWireError::PayloadTooLong {
                            len: payload.len(),
                            max: 2,
                        });
                    }
                    SnakeEvent::GameOver(SnakeGameOverReason::try_from(payload[1])?)
                }
                code => return Err(SnakeWireError::InvalidMessage { code }),
            };

            Ok(SnakeMessage::Event(event))
        }
        3 => {
            if payload.len() != 1 {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: 1,
                });
            }

            Ok(SnakeMessage::Error(SnakeFault::try_from(payload[0])?))
        }
        4 => {
            if payload.len() != 4 {
                return Err(SnakeWireError::PayloadTooLong {
                    len: payload.len(),
                    max: 4,
                });
            }

            Ok(SnakeMessage::Heartbeat(SnakeTick::new(read_u32_le(
                payload,
            ))))
        }
        _ => Err(SnakeWireError::InvalidMessage { code: opcode }),
    }
}

fn write_u16_le(out: &mut [u8], value: u16) {
    out[0] = (value & 0x00ff) as u8;
    out[1] = (value >> 8) as u8;
}

fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from(bytes[0]) | (u16::from(bytes[1]) << 8)
}

fn write_u32_le(out: &mut [u8], value: u32) {
    out[0] = (value & 0x0000_00ff) as u8;
    out[1] = ((value >> 8) & 0x0000_00ff) as u8;
    out[2] = ((value >> 16) & 0x0000_00ff) as u8;
    out[3] = ((value >> 24) & 0x0000_00ff) as u8;
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from(bytes[0])
        | (u32::from(bytes[1]) << 8)
        | (u32::from(bytes[2]) << 16)
        | (u32::from(bytes[3]) << 24)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::ToOwned;
    use std::string::ToString;

    #[test]
    fn exposes_a_stable_current_version() {
        assert_eq!(SnakeVersion::CURRENT.get(), 1);
        assert_eq!(u8::from(SnakeVersion::CURRENT), 1);
    }

    #[test]
    fn command_packets_round_trip() {
        let command = SnakeCommand::Hello {
            min_version: SnakeVersion::new(1),
            max_version: SnakeVersion::new(2),
            capabilities: SnakeCapabilities::new(0b101),
        };
        let packet = SnakeCommandPacket::new(command);
        let (bytes, len) = packet.encode();

        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 1);
        assert_eq!(len, 8);

        let decoded = SnakeCommandPacket::decode(&bytes[..len]).expect("decoded");
        assert_eq!(decoded.command(), command);
    }

    #[test]
    fn command_packets_encode_direction_and_start() {
        let board = SnakeBoardSize::new(12, 8).expect("board");
        let packet = SnakeCommandPacket::new(SnakeCommand::Start {
            seed: 0x1234_5678,
            board,
        });
        let (bytes, len) = packet.encode();
        assert_eq!(len, 10);
        assert_eq!(bytes[2], 2);

        let decoded = SnakeCommandPacket::decode(&bytes[..len]).expect("decoded");
        assert_eq!(
            decoded.command(),
            SnakeCommand::Start {
                seed: 0x1234_5678,
                board
            }
        );

        let packet = SnakeCommandPacket::new(SnakeCommand::SetDirection(SnakeDirection::Left));
        let (bytes, len) = packet.encode();
        assert_eq!(len, 5);
        let decoded = SnakeCommandPacket::decode(&bytes[..len]).expect("decoded");
        assert_eq!(
            decoded.command(),
            SnakeCommand::SetDirection(SnakeDirection::Left)
        );
    }

    #[test]
    fn snapshot_messages_round_trip_with_body_cells() {
        let board = SnakeBoardSize::new(10, 6).expect("board");
        let body = SnakeBody::new(&[
            SnakeCell::new(4, 2),
            SnakeCell::new(3, 2),
            SnakeCell::new(2, 2),
        ])
        .expect("body");
        let snapshot = SnakeSnapshot::new(
            SnakeBoardSnapshot::new(board, SnakeCell::new(4, 2), SnakeCell::new(1, 3), body),
            SnakeSessionState::Running,
            SnakeTick::new(42),
            SnakeScore::new(7),
            SnakeDirection::Right,
        );
        let packet = SnakeMessagePacket::new(SnakeMessage::Snapshot(snapshot.clone()));
        let (bytes, len) = packet.encode();

        assert_eq!(bytes[1], 2);
        assert_eq!(bytes[2], 1);

        let decoded = SnakeMessagePacket::decode(&bytes[..len]).expect("decoded");
        assert_eq!(decoded.message(), SnakeMessage::Snapshot(snapshot));
    }

    #[test]
    fn event_error_and_heartbeat_messages_round_trip() {
        let packet = SnakeMessagePacket::new(SnakeMessage::Event(SnakeEvent::GameOver(
            SnakeGameOverReason::SelfCollision,
        )));
        let (bytes, len) = packet.encode();
        assert_eq!(len, 6);
        assert_eq!(
            SnakeMessagePacket::decode(&bytes[..len]).unwrap().message(),
            packet.message()
        );

        let packet = SnakeMessagePacket::new(SnakeMessage::Error(SnakeFault::Desync));
        let (bytes, len) = packet.encode();
        assert_eq!(len, 5);
        assert_eq!(
            SnakeMessagePacket::decode(&bytes[..len]).unwrap().message(),
            packet.message()
        );

        let packet = SnakeMessagePacket::new(SnakeMessage::Heartbeat(SnakeTick::new(99)));
        let (bytes, len) = packet.encode();
        assert_eq!(len, 8);
        assert_eq!(
            SnakeMessagePacket::decode(&bytes[..len]).unwrap().message(),
            packet.message()
        );
    }

    #[test]
    fn rejects_invalid_wire_values() {
        assert_eq!(
            SnakeBoardSize::new(0, 4).map_err(|error| error.to_string()),
            Err("invalid board size: 0x4".to_owned())
        );

        assert_eq!(
            SnakeDirection::try_from(9).map_err(|error| error.to_string()),
            Err("invalid direction: 9".to_owned())
        );
    }
}
