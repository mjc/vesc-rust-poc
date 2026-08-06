//! Validated descriptor for Refloat's optional recorder firmware buffer.

// TODO(vescpkg-rs): Split the Refloat 6.05 descriptor address, magic, version,
// and CCM range from the generic recorder ring and protocol before supporting
// another recorder-firmware layout.

use core::ptr::NonNull;

use crate::{SampleRate, TimestampTicks};

const DATA_RECORDER_MAGIC_BASE: u32 = 0xcafe_1000;
const DATA_RECORDER_REQUIRED_MAJOR: u8 = 1;
const DATA_RECORDER_REQUIRED_MINOR: u8 = 1;
const DATA_RECORDER_RAM_START: u32 = 0x1000_0000;
const DATA_RECORDER_RAM_END: u32 = 0x1000_f800;
const DATA_RECORDER_ALIGNMENT: u32 = 4;
#[cfg(target_arch = "arm")]
const DATA_RECORDER_DESCRIPTOR_ADDRESS: usize = 0x1000_fff4;

/// Storage-agnostic cursor for a fixed-capacity circular buffer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RingCursor {
    next: usize,
    len: usize,
}

fn advance_ring_index(index: usize, capacity: usize) -> usize {
    index
        .checked_add(1)
        .filter(|next| *next < capacity)
        .unwrap_or(0)
}

impl RingCursor {
    /// Forget all committed slots.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Return the slot for the next write, or `None` when capacity is zero.
    #[must_use]
    pub fn write_slot(self, capacity: usize) -> Option<usize> {
        (capacity > 0).then_some(self.next)
    }

    /// Commit a successful write and advance the cursor.
    pub fn commit_write(&mut self, capacity: usize) {
        self.next = advance_ring_index(self.next, capacity);
        self.len = self.len.saturating_add(1).min(capacity);
    }

    /// Return the number of committed slots available at this capacity.
    #[must_use]
    pub fn len(self, capacity: usize) -> usize {
        self.len.min(capacity)
    }

    /// Return whether no committed slots are available at this capacity.
    #[must_use]
    pub fn is_empty(self, capacity: usize) -> bool {
        self.len(capacity) == 0
    }

    /// Resolve an oldest-first logical index to a physical storage slot.
    #[must_use]
    pub fn slot_at(self, index: usize, capacity: usize) -> Option<usize> {
        let len = self.len(capacity);
        if index >= len || capacity == 0 {
            return None;
        }
        let oldest = self
            .next
            .checked_add(capacity)?
            .checked_sub(len)?
            .checked_rem(capacity)?;
        oldest.checked_add(index)?.checked_rem(capacity)
    }
}

/// Checked byte storage for a fixed-size record ring.
pub trait FixedRecordStorage {
    /// Return the available byte length.
    fn len(&self) -> usize;

    /// Return whether the storage contains no bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Copy a complete checked range into storage.
    fn write(&mut self, offset: usize, bytes: &[u8]) -> bool;

    /// Copy a complete checked range out of storage.
    fn read(&self, offset: usize, bytes: &mut [u8]) -> bool;
}

impl<const N: usize> FixedRecordStorage for [u8; N] {
    fn len(&self) -> usize {
        N
    }

    fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        let Some(target) = offset
            .checked_add(bytes.len())
            .and_then(|end| self.get_mut(offset..end))
        else {
            return false;
        };
        target.copy_from_slice(bytes);
        true
    }

    fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        let Some(source) = offset
            .checked_add(bytes.len())
            .and_then(|end| self.get(offset..end))
        else {
            return false;
        };
        bytes.copy_from_slice(source);
        true
    }
}

impl<T: FixedRecordStorage> FixedRecordStorage for Option<T> {
    fn len(&self) -> usize {
        self.as_ref().map_or(0, FixedRecordStorage::len)
    }

    fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        self.as_mut()
            .is_some_and(|storage| storage.write(offset, bytes))
    }

    fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        self.as_ref()
            .is_some_and(|storage| storage.read(offset, bytes))
    }
}

/// A checked oldest-first ring over fixed-size records in caller-owned storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRecordRing<S, const RECORD_SIZE: usize> {
    cursor: RingCursor,
    storage: S,
}

impl<S: FixedRecordStorage, const RECORD_SIZE: usize> FixedRecordRing<S, RECORD_SIZE> {
    /// Build an empty record ring over `storage`.
    pub const fn new(storage: S) -> Self {
        Self {
            cursor: RingCursor { next: 0, len: 0 },
            storage,
        }
    }

    /// Replace the backing storage and forget all prior records.
    pub fn replace_storage(&mut self, storage: S) {
        self.storage = storage;
        self.clear();
    }

    /// Forget all committed records without modifying storage bytes.
    pub fn clear(&mut self) {
        self.cursor.clear();
    }

    /// Return the number of complete records the storage can retain.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.storage.len().checked_div(RECORD_SIZE).unwrap_or(0)
    }

    /// Return the number of committed records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cursor.len(self.capacity())
    }

    /// Return whether no records are committed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Append one record, overwriting the oldest record when full.
    #[must_use]
    pub fn push(&mut self, record: &[u8; RECORD_SIZE]) -> bool {
        let capacity = self.capacity();
        let Some(offset) = self
            .cursor
            .write_slot(capacity)
            .and_then(|slot| slot.checked_mul(RECORD_SIZE))
        else {
            return false;
        };
        if !self.storage.write(offset, record) {
            return false;
        }
        self.cursor.commit_write(capacity);
        true
    }

    /// Copy one oldest-first logical record from storage.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<[u8; RECORD_SIZE]> {
        let capacity = self.capacity();
        let slot = self.cursor.slot_at(index, capacity)?;
        let offset = slot.checked_mul(RECORD_SIZE)?;
        let mut record = [0; RECORD_SIZE];
        self.storage.read(offset, &mut record).then_some(record)
    }
}

/// Fixed-record storage with sample-rate bookkeeping and every-N decimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimatedRecordRing<S, const RECORD_SIZE: usize> {
    records: FixedRecordRing<S, RECORD_SIZE>,
    decimation: u8,
    decimation_counter: u8,
    sample_rate_hz: u16,
}

impl<S: FixedRecordStorage, const RECORD_SIZE: usize> DecimatedRecordRing<S, RECORD_SIZE> {
    /// Build an empty recorder at the given whole-Hz sample rate.
    pub const fn new(storage: S, sample_rate_hz: u16) -> Self {
        Self {
            records: FixedRecordRing::new(storage),
            decimation: 1,
            decimation_counter: 0,
            sample_rate_hz: if sample_rate_hz == 0 {
                1
            } else {
                sample_rate_hz
            },
        }
    }

    /// Replace the backing storage and forget all prior records.
    pub fn replace_storage(&mut self, storage: S) {
        self.records.replace_storage(storage);
    }

    /// Forget prior records and restart the every-N sample counter.
    pub fn reset(&mut self) {
        self.records.clear();
        self.decimation_counter = 0;
    }

    /// Return whether the current input sample should be recorded.
    pub fn sample_due(&mut self) -> bool {
        self.decimation_counter = self.decimation_counter.wrapping_add(1);
        if self.decimation_counter < self.decimation {
            return false;
        }
        self.decimation_counter = 0;
        true
    }

    /// Set the every-N sample factor, treating zero as every sample.
    pub const fn set_decimation(&mut self, decimation: u8) {
        self.decimation = if decimation == 0 { 1 } else { decimation };
    }

    /// Return the every-N sample factor.
    #[must_use]
    pub const fn decimation(&self) -> u8 {
        self.decimation
    }

    /// Update the live sample rate and optionally derive decimation for a target window.
    pub fn configure_sample_rate(&mut self, sample_rate: SampleRate, target_seconds: Option<u16>) {
        let rate =
            crate::protocol_buffer::saturating_trunc_f32_to_u32(sample_rate.as_hertz()).max(1);
        self.sample_rate_hz = u16::try_from(rate).unwrap_or(u16::MAX);
        if let Some(target_seconds) = target_seconds {
            self.recalculate_decimation(target_seconds);
        }
    }

    /// Derive every-N decimation for a full-buffer target duration.
    pub fn recalculate_decimation(&mut self, target_seconds: u16) {
        let sample_count = u32::try_from(self.capacity()).unwrap_or(u32::MAX).max(1);
        let value = u32::from(self.sample_rate_hz)
            .saturating_mul(u32::from(target_seconds))
            .checked_div(sample_count)
            .unwrap_or(0)
            .max(1);
        let [decimation, ..] = value.to_le_bytes();
        self.decimation = decimation;
    }

    /// Return the full-capacity recording duration in centiseconds.
    #[must_use]
    pub fn recording_duration_centiseconds(&self) -> u16 {
        self.recording_duration_centiseconds_at_capacity(self.capacity())
    }

    /// Return the recording duration for an explicit logical record capacity.
    #[must_use]
    pub fn recording_duration_centiseconds_at_capacity(&self, capacity: usize) -> u16 {
        let capacity = u32::try_from(capacity).unwrap_or(u32::MAX);
        let duration = capacity
            .saturating_mul(100)
            .checked_div(u32::from(self.sample_rate_hz))
            .unwrap_or(0);
        u16::try_from(duration).unwrap_or(u16::MAX)
    }

    /// Return the number of complete records the storage can retain.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.records.capacity()
    }

    /// Return the number of committed records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Return whether no records are committed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Append one record, overwriting the oldest record when full.
    #[must_use]
    pub fn push(&mut self, record: &[u8; RECORD_SIZE]) -> bool {
        self.records.push(record)
    }

    /// Copy one oldest-first logical record from storage.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<[u8; RECORD_SIZE]> {
        self.records.get(index)
    }
}

bitflags::bitflags! {
    /// Standard VESC package data-recorder state flags.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct DataRecorderFlags: u8 {
        /// Data recording is active.
        const RECORDING = 1 << 0;
        /// Recording starts automatically when the package engages.
        const AUTOSTART = 1 << 1;
        /// Recording stops automatically when the package disengages.
        const AUTOSTOP = 1 << 2;
    }
}

/// Reply action selected by the standard VESC package recorder request protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRecorderReply {
    /// The request is malformed, unknown, or unavailable and needs no reply.
    None,
    /// Return the current recorder status.
    Status,
    /// Stop recording and return the package-specific field header.
    Header,
    /// Return recorded data beginning at the given logical sample offset.
    Data(u32),
}

/// Package-specific command IDs and header bytes for the standard recorder wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataRecorderProtocol<const HEADER_SIZE: usize, const DATA_RESPONSE_CAPACITY: usize> {
    package_id: u8,
    status_command: u8,
    header_command: u8,
    data_command: u8,
    header: [u8; HEADER_SIZE],
}

impl<const HEADER_SIZE: usize, const DATA_RESPONSE_CAPACITY: usize>
    DataRecorderProtocol<HEADER_SIZE, DATA_RESPONSE_CAPACITY>
{
    /// Describe one package's recorder response commands and field header.
    #[must_use]
    pub const fn new(
        package_id: u8,
        status_command: u8,
        header_command: u8,
        data_command: u8,
        header: [u8; HEADER_SIZE],
    ) -> Self {
        Self {
            package_id,
            status_command,
            header_command,
            data_command,
            header,
        }
    }
}

/// Fixed-storage state for the standard VESC package data-recorder protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataRecorder<S, const RECORD_SIZE: usize> {
    flags: DataRecorderFlags,
    records: DecimatedRecordRing<S, RECORD_SIZE>,
    last_timestamp: TimestampTicks,
}

impl<S: FixedRecordStorage, const RECORD_SIZE: usize> DataRecorder<S, RECORD_SIZE> {
    /// Build an empty recorder with the standard autostart and autostop policy.
    pub const fn new(storage: S, sample_rate_hz: u16) -> Self {
        Self {
            flags: DataRecorderFlags::AUTOSTART.union(DataRecorderFlags::AUTOSTOP),
            records: DecimatedRecordRing::new(storage, sample_rate_hz),
            last_timestamp: TimestampTicks::from_ticks(0),
        }
    }

    /// Replace storage, stop recording, and derive decimation for a target duration.
    pub fn initialize(&mut self, storage: S, target_seconds: u16) {
        self.records.replace_storage(storage);
        self.stop();
        self.records.recalculate_decimation(target_seconds);
    }

    /// Replace storage, clear prior records, and stop recording.
    pub fn replace_storage(&mut self, storage: S) {
        self.records.replace_storage(storage);
        self.stop();
    }

    /// Return whether at least one complete record fits in storage.
    #[must_use]
    pub fn has_capability(&self) -> bool {
        self.records.capacity() > 0
    }

    /// Return the configured recorder state flags.
    #[must_use]
    pub const fn flags(&self) -> DataRecorderFlags {
        self.flags
    }

    /// Return live flags only when recorder storage is available.
    #[must_use]
    pub fn available_flags(&self) -> DataRecorderFlags {
        if self.has_capability() {
            self.flags
        } else {
            DataRecorderFlags::empty()
        }
    }

    /// Start or stop recording when the corresponding automatic policy is enabled.
    pub fn trigger(&mut self, engage: bool) {
        if self.flags.contains(DataRecorderFlags::AUTOSTART) && engage {
            self.start();
        } else if self.flags.contains(DataRecorderFlags::AUTOSTOP) && !engage {
            self.stop();
        }
    }

    /// Clear prior samples and start recording when storage is available.
    pub fn start(&mut self) {
        self.records.reset();
        self.last_timestamp = TimestampTicks::from_ticks(0);
        self.flags
            .set(DataRecorderFlags::RECORDING, self.has_capability());
    }

    /// Stop recording without clearing samples.
    pub fn stop(&mut self) {
        self.flags.remove(DataRecorderFlags::RECORDING);
    }

    /// Record one sample when active and due, making its leading timestamp monotonic.
    #[must_use]
    pub fn sample(&mut self, mut sample: [u8; RECORD_SIZE]) -> bool {
        if !self.flags.contains(DataRecorderFlags::RECORDING) {
            return false;
        }
        let Some(timestamp_bytes) = sample.get(..4).and_then(|bytes| bytes.try_into().ok()) else {
            return false;
        };
        if !self.records.sample_due() {
            return false;
        }
        let timestamp = u32::from_be_bytes(timestamp_bytes);
        let timestamp = if timestamp <= self.last_timestamp.as_ticks() {
            self.last_timestamp.as_ticks().wrapping_add(1)
        } else {
            timestamp
        };
        self.last_timestamp = TimestampTicks::from_ticks(timestamp);
        let Some(target) = sample.get_mut(..4) else {
            return false;
        };
        target.copy_from_slice(&timestamp.to_be_bytes());
        self.records.push(&sample)
    }

    /// Apply one standard recorder request and return the package-specific reply action.
    pub fn handle_request(&mut self, payload: &[u8]) -> DataRecorderReply {
        let control = matches!(payload, [1, _, ..]);
        if !self.has_capability() && !control {
            return DataRecorderReply::None;
        }
        match payload {
            [1, 1, value, ..] => {
                if *value > 0 {
                    self.start();
                } else {
                    self.stop();
                }
                DataRecorderReply::Status
            }
            [1, 2, value, ..] => {
                self.flags.set(DataRecorderFlags::AUTOSTART, *value > 0);
                DataRecorderReply::Status
            }
            [1, 3, value, ..] => {
                self.flags.set(DataRecorderFlags::AUTOSTOP, *value > 0);
                DataRecorderReply::Status
            }
            [1, 4, value, ..] => {
                self.records.set_decimation(*value);
                DataRecorderReply::Status
            }
            [1, 0, ..] | [1, _, _, ..] => DataRecorderReply::Status,
            [2, 1, ..] => {
                self.stop();
                DataRecorderReply::Header
            }
            [2, 2, a, b, c, d, ..] => DataRecorderReply::Data(u32::from_be_bytes([*a, *b, *c, *d])),
            _ => DataRecorderReply::None,
        }
    }

    /// Apply one recorder request and emit its standard package wire response, if any.
    pub fn reply_to_request<const HEADER_SIZE: usize, const DATA_RESPONSE_CAPACITY: usize>(
        &mut self,
        payload: &[u8],
        protocol: &DataRecorderProtocol<HEADER_SIZE, DATA_RESPONSE_CAPACITY>,
        reported_capacity: Option<usize>,
        reply: &mut impl FnMut(&[u8]) -> bool,
    ) {
        match self.handle_request(payload) {
            DataRecorderReply::None => {}
            DataRecorderReply::Status => {
                let duration = reported_capacity.map_or_else(
                    || self.records.recording_duration_centiseconds(),
                    |capacity| {
                        self.records
                            .recording_duration_centiseconds_at_capacity(capacity)
                    },
                );
                let duration = duration.to_be_bytes();
                let response = [
                    protocol.package_id,
                    protocol.status_command,
                    u8::from(self.has_capability()),
                    self.flags.bits(),
                    self.records.decimation(),
                    duration[0],
                    duration[1],
                ];
                let _ = reply(&response);
            }
            DataRecorderReply::Header => {
                let mut response = protocol.header;
                let count = u32::try_from(self.records.len()).unwrap_or(u32::MAX);
                if let Some(command) = response.get_mut(1) {
                    *command = protocol.header_command;
                }
                if let Some(target) = response.get_mut(2..6) {
                    target.copy_from_slice(&count.to_be_bytes());
                    let _ = reply(&response);
                }
            }
            DataRecorderReply::Data(offset) => {
                let mut response =
                    crate::protocol_buffer::FixedBuffer::<DATA_RESPONSE_CAPACITY>::new();
                response.push(protocol.package_id);
                response.push(protocol.data_command);
                response.push_u32(offset);
                let mut sample_index = usize::try_from(offset).unwrap_or(usize::MAX);
                while response.remaining() >= RECORD_SIZE {
                    let Some(sample) = self.records.get(sample_index) else {
                        break;
                    };
                    response.extend(&sample);
                    sample_index = sample_index.saturating_add(1);
                }
                if !self.records.is_empty() {
                    let _ = reply(response.as_bytes());
                }
            }
        }
    }

    /// Borrow the fixed-record ring and its sample-rate bookkeeping.
    #[must_use]
    pub const fn records(&self) -> &DecimatedRecordRing<S, RECORD_SIZE> {
        &self.records
    }

    /// Mutably borrow the fixed-record ring and its sample-rate bookkeeping.
    pub const fn records_mut(&mut self) -> &mut DecimatedRecordRing<S, RECORD_SIZE> {
        &mut self.records
    }
}

/// A validated recorder-buffer descriptor from Refloat's special VESC 6.05 firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareDataRecorderDescriptor {
    version: FirmwareDataRecorderVersion,
    start_address: u32,
    len: u32,
}

/// Recorder-firmware descriptor version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FirmwareDataRecorderVersion {
    major: u8,
    minor: u8,
}

impl FirmwareDataRecorderVersion {
    /// Return the compatibility-breaking major version.
    #[must_use]
    pub const fn major(self) -> u8 {
        self.major
    }

    /// Return the backwards-compatible minor version.
    #[must_use]
    pub const fn minor(self) -> u8 {
        self.minor
    }
}

impl FirmwareDataRecorderDescriptor {
    /// Validate the descriptor words stored immediately after the VESC interface.
    ///
    /// # Errors
    ///
    /// Returns the specific descriptor invariant that failed.
    pub fn try_from_words(
        [magic, start_address, len]: [u32; 3],
    ) -> Result<Self, FirmwareDataRecorderDescriptorError> {
        if magic & 0xffff_f000 != DATA_RECORDER_MAGIC_BASE {
            return Err(FirmwareDataRecorderDescriptorError::BadMagic);
        }
        let [magic_low, ..] = magic.to_le_bytes();
        let version = FirmwareDataRecorderVersion {
            major: magic_low >> 4,
            minor: magic_low & 0x0f,
        };
        if version.major != DATA_RECORDER_REQUIRED_MAJOR
            || version.minor < DATA_RECORDER_REQUIRED_MINOR
        {
            return Err(FirmwareDataRecorderDescriptorError::IncompatibleVersion);
        }
        if start_address % DATA_RECORDER_ALIGNMENT != 0 || len % DATA_RECORDER_ALIGNMENT != 0 {
            return Err(FirmwareDataRecorderDescriptorError::Misaligned);
        }
        if len < DATA_RECORDER_ALIGNMENT {
            return Err(FirmwareDataRecorderDescriptorError::Undersized);
        }
        let end = start_address
            .checked_add(len)
            .ok_or(FirmwareDataRecorderDescriptorError::AddressOverflow)?;
        if start_address < DATA_RECORDER_RAM_START || end > DATA_RECORDER_RAM_END {
            return Err(FirmwareDataRecorderDescriptorError::OutsideRecorderRam);
        }
        Ok(Self {
            version,
            start_address,
            len,
        })
    }

    /// Return the validated recorder descriptor version.
    #[must_use]
    pub const fn version(self) -> FirmwareDataRecorderVersion {
        self.version
    }

    /// Return the validated native start address.
    #[must_use]
    pub const fn start_address(self) -> u32 {
        self.start_address
    }

    /// Return the validated byte length.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Return whether the validated region is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Reason a recorder-firmware descriptor was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FirmwareDataRecorderDescriptorError {
    /// The firmware magic did not identify Refloat's recorder build.
    BadMagic,
    /// The recorder descriptor major version differs or its minor version is too old.
    IncompatibleVersion,
    /// The buffer start or length did not preserve ARM word alignment.
    Misaligned,
    /// The buffer cannot hold even one aligned firmware word.
    Undersized,
    /// Adding the byte length overflowed the 32-bit firmware address.
    AddressOverflow,
    /// The buffer was not wholly inside the recorder firmware's reserved CCM region.
    OutsideRecorderRam,
}

impl core::fmt::Display for FirmwareDataRecorderDescriptorError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::BadMagic => "bad recorder firmware magic",
            Self::IncompatibleVersion => "incompatible recorder firmware version",
            Self::Misaligned => "misaligned recorder buffer",
            Self::Undersized => "undersized recorder buffer",
            Self::AddressOverflow => "recorder buffer address overflow",
            Self::OutsideRecorderRam => "recorder buffer outside reserved CCM RAM",
        })
    }
}

impl core::error::Error for FirmwareDataRecorderDescriptorError {}

/// Exclusive package handle to a validated recorder-firmware RAM region.
pub struct FirmwareDataRecorderBuffer {
    address: NonNull<u8>,
    len: usize,
}

impl core::fmt::Debug for FirmwareDataRecorderBuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("FirmwareDataRecorderBuffer")
            .field("address", &self.address)
            .field("len", &self.len)
            .finish()
    }
}

#[cfg(target_arch = "arm")]
// SAFETY: the special recorder firmware dedicates this static CCM region to
// one recorder package. Moving its unique handle between package threads does
// not change the region's lifetime or introduce another owner.
unsafe impl Send for FirmwareDataRecorderBuffer {}

impl FirmwareDataRecorderBuffer {
    /// Build a handle from a validated descriptor and its live native address.
    ///
    /// # Safety
    ///
    /// `address` must remain readable and writable for `descriptor.len()`
    /// bytes for the handle's lifetime, and no other code may access that
    /// region concurrently.
    #[cfg(any(test, target_arch = "arm"))]
    unsafe fn from_descriptor_and_address(
        descriptor: FirmwareDataRecorderDescriptor,
        address: *mut u8,
    ) -> Self {
        Self {
            // SAFETY: the caller guarantees a live non-null region matching
            // the already validated descriptor.
            address: unsafe { NonNull::new_unchecked(address) },
            len: usize::try_from(descriptor.len()).unwrap_or(0),
        }
    }

    #[cfg(target_arch = "arm")]
    pub(crate) fn discover() -> Option<Self> {
        let words = core::array::from_fn(|index| {
            // SAFETY: native ARM VESC packages use the fixed F407 VESC_IF
            // region. The special firmware places three aligned words at
            // `0x1000fff4`; standard F40x firmware leaves that mapped trailer
            // unused, so its non-magic contents fail closed.
            unsafe {
                (DATA_RECORDER_DESCRIPTOR_ADDRESS as *const u32)
                    .add(index)
                    .read_volatile()
            }
        });
        let descriptor = FirmwareDataRecorderDescriptor::try_from_words(words).ok()?;
        let address = usize::try_from(descriptor.start_address()).ok()? as *mut u8;
        // SAFETY: descriptor validation proves a non-null, aligned range wholly
        // inside the special firmware's reserved CCM region. The package-start
        // coordinator issues at most one handle for this lifecycle.
        Some(unsafe { Self::from_descriptor_and_address(descriptor, address) })
    }

    /// Return the firmware-reserved byte length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether the reserved region is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Copy bytes into the reserved region when the complete range is valid.
    pub fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };
        if end > self.len {
            return false;
        }
        // SAFETY: construction establishes exclusive access to `self.len`
        // bytes, and the checked range is wholly inside that region.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.address.as_ptr().add(offset),
                bytes.len(),
            );
        }
        true
    }

    /// Copy bytes from the reserved region when the complete range is valid.
    pub fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        let Some(end) = offset.checked_add(bytes.len()) else {
            return false;
        };
        if end > self.len {
            return false;
        }
        // SAFETY: construction establishes readable access to `self.len`
        // bytes, and the checked range is wholly inside that region.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.address.as_ptr().add(offset),
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }
        true
    }
}

impl FixedRecordStorage for FirmwareDataRecorderBuffer {
    fn len(&self) -> usize {
        Self::len(self)
    }

    fn write(&mut self, offset: usize, bytes: &[u8]) -> bool {
        Self::write(self, offset, bytes)
    }

    fn read(&self, offset: usize, bytes: &mut [u8]) -> bool {
        Self::read(self, offset, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ring_cursor_is_empty() {
        assert_eq!(RingCursor::default().len(24), 0);
    }

    #[test]
    fn ring_cursor_preserves_newest_slots_across_capacities_and_wraps() {
        for capacity in 0_usize..=32 {
            for writes in 0..=capacity.saturating_mul(3).saturating_add(1) {
                let mut ring = RingCursor::default();
                for _ in 0..writes {
                    if ring.write_slot(capacity).is_some() {
                        ring.commit_write(capacity);
                    }
                }

                let len = writes.min(capacity);
                assert_eq!(ring.len(capacity), len);
                for index in 0..len {
                    let expected = writes.saturating_sub(len).saturating_add(index) % capacity;
                    assert_eq!(ring.slot_at(index, capacity), Some(expected));
                }
                assert_eq!(ring.slot_at(len, capacity), None);
            }
        }
    }

    #[test]
    fn recorder_buffer_reads_and_writes_only_checked_ranges() {
        let mut storage = [0_u8; 32];
        let descriptor =
            FirmwareDataRecorderDescriptor::try_from_words([0xcafe_1111, 0x1000_0000, 32])
                .expect("test descriptor");
        // SAFETY: `storage` remains alive and exclusively borrowed for the
        // handle's complete use below.
        let mut buffer = unsafe {
            FirmwareDataRecorderBuffer::from_descriptor_and_address(
                descriptor,
                storage.as_mut_ptr(),
            )
        };

        assert!(buffer.write(4, &[1, 2, 3, 4]));
        let mut copied = [0; 4];
        assert!(buffer.read(4, &mut copied));
        assert_eq!(copied, [1, 2, 3, 4]);
        assert!(!buffer.write(31, &[5, 6]));
        assert!(!buffer.read(usize::MAX, &mut copied));
        assert_eq!(&storage[4..8], &[1, 2, 3, 4]);
    }

    #[test]
    fn fixed_record_ring_keeps_latest_records_in_oldest_first_order() {
        let mut records = FixedRecordRing::<[u8; 6], 2>::new([0; 6]);
        for record in [[1, 2], [3, 4], [5, 6], [7, 8]] {
            assert!(records.push(&record));
        }

        assert_eq!(records.len(), 3);
        assert_eq!(records.get(0), Some([3, 4]));
        assert_eq!(records.get(1), Some([5, 6]));
        assert_eq!(records.get(2), Some([7, 8]));
        assert_eq!(records.get(3), None);
    }

    #[test]
    fn replacing_optional_record_storage_clears_and_disables_the_ring() {
        let mut records = FixedRecordRing::<Option<[u8; 4]>, 2>::new(Some([0; 4]));
        assert!(records.push(&[1, 2]));

        records.replace_storage(None);

        assert_eq!(records.capacity(), 0);
        assert!(records.is_empty());
        assert!(!records.push(&[3, 4]));
        assert_eq!(records.get(0), None);
    }

    #[test]
    fn decimated_record_ring_records_every_nth_sample_and_reset_restarts_the_count() {
        let mut records = DecimatedRecordRing::<[u8; 6], 2>::new([0; 6], 620);
        records.set_decimation(3);

        assert!(!records.sample_due());
        assert!(!records.sample_due());
        assert!(records.sample_due());
        assert!(records.push(&[1, 2]));
        assert_eq!(records.get(0), Some([1, 2]));

        assert!(!records.sample_due());
        records.reset();
        assert!(!records.sample_due());
        assert!(!records.sample_due());
        assert!(records.sample_due());
    }

    #[test]
    fn sample_rate_refresh_can_preserve_decimation_while_updating_duration() {
        let mut records = DecimatedRecordRing::<[u8; 48], 2>::new([0; 48], 620);

        records.configure_sample_rate(SampleRate::from_hertz(48.0), Some(10));
        assert_eq!(records.decimation(), 20);
        assert_eq!(records.recording_duration_centiseconds(), 50);

        records.configure_sample_rate(SampleRate::from_hertz(24.0), None);
        assert_eq!(records.decimation(), 20);
        assert_eq!(records.recording_duration_centiseconds(), 100);
    }

    #[test]
    fn package_data_recorder_preserves_the_shared_vesc_request_state_machine() {
        let mut recorder = DataRecorder::<[u8; 12], 4>::new([0; 12], 100);

        assert_eq!(
            recorder.flags(),
            DataRecorderFlags::AUTOSTART | DataRecorderFlags::AUTOSTOP
        );
        assert_eq!(
            recorder.handle_request(&[1, 2, 0]),
            DataRecorderReply::Status
        );
        assert_eq!(recorder.flags(), DataRecorderFlags::AUTOSTOP);
        assert_eq!(
            recorder.handle_request(&[1, 1, 1]),
            DataRecorderReply::Status
        );
        assert!(recorder.flags().contains(DataRecorderFlags::RECORDING));

        assert!(recorder.sample([0, 0, 0, 0]));
        assert!(recorder.sample([0, 0, 0, 0]));
        assert_eq!(recorder.records().get(0), Some([0, 0, 0, 1]));
        assert_eq!(recorder.records().get(1), Some([0, 0, 0, 2]));
        assert_eq!(recorder.handle_request(&[2, 1]), DataRecorderReply::Header);
        assert!(!recorder.flags().contains(DataRecorderFlags::RECORDING));
        assert_eq!(
            recorder.handle_request(&[2, 2, 0, 0, 0, 1]),
            DataRecorderReply::Data(1)
        );
        assert_eq!(recorder.handle_request(&[2, 2, 0]), DataRecorderReply::None);
    }

    #[test]
    fn package_data_recorder_frames_standard_status_header_and_data_replies() {
        let mut recorder = DataRecorder::<[u8; 8], 2>::new([0; 8], 100);
        let protocol = DataRecorderProtocol::<7, 11>::new(101, 41, 42, 43, [101, 0, 0, 0, 0, 0, 9]);
        recorder.start();
        assert!(recorder.records_mut().push(&[0, 1]));

        let mut replies = std::vec::Vec::new();
        for request in [&[1, 0][..], &[2, 1], &[2, 2, 0, 0, 0, 0]] {
            recorder.reply_to_request(request, &protocol, None, &mut |bytes| {
                replies.push(bytes.to_vec());
                true
            });
        }

        assert_eq!(replies[0], [101, 41, 1, 0b111, 1, 0, 4]);
        assert_eq!(replies[1], [101, 42, 0, 0, 0, 1, 9]);
        assert_eq!(replies[2], [101, 43, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn package_data_recorder_omits_unavailable_and_empty_data_replies() {
        let protocol = DataRecorderProtocol::<6, 8>::new(101, 41, 42, 43, [101, 0, 0, 0, 0, 0]);
        let mut recorder = DataRecorder::<Option<[u8; 2]>, 2>::new(None, 100);
        let mut replies = std::vec::Vec::new();

        recorder.reply_to_request(&[1, 0], &protocol, Some(1), &mut |bytes| {
            replies.push(bytes.to_vec());
            true
        });
        recorder.replace_storage(Some([0; 2]));
        recorder.reply_to_request(&[2, 2, 0, 0, 0, 0], &protocol, None, &mut |bytes| {
            replies.push(bytes.to_vec());
            true
        });

        assert_eq!(replies, [[101, 41, 0, 0b110, 1, 0, 1]]);
    }
}
