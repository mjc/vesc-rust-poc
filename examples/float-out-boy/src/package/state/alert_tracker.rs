const ALERT_RECORD_CAPACITY: usize = 20;

#[cfg(test)]
pub(super) type AlertRecord = vescpkg_rs::FirmwareFaultRecord;
pub(super) type AlertTrackerState = vescpkg_rs::FirmwareFaultHistory<ALERT_RECORD_CAPACITY>;

#[cfg(test)]
mod tests;
