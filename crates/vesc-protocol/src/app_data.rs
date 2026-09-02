//! Fixed-shape VESC app-data request parsing.

/// Error returned when a fixed app-data request does not match its wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedAppDataRequestError {
    /// The request has the wrong total byte length.
    Length {
        /// Actual request byte length.
        actual: usize,
    },
    /// The request carries a different package ID.
    PackageId {
        /// Rejected package ID.
        value: u8,
    },
    /// The request carries a different command ID.
    Command {
        /// Rejected command ID.
        value: u8,
    },
}

/// Parse a fixed-size payload after exact package and command bytes.
///
/// # Errors
///
/// Returns the first wire-shape, package-ID, or command-ID mismatch.
pub fn parse_fixed_app_data_request<const N: usize>(
    bytes: &[u8],
    package_id: u8,
    command_id: u8,
) -> Result<&[u8; N], FixedAppDataRequestError> {
    let [actual_package_id, actual_command_id, payload @ ..] = bytes else {
        return Err(FixedAppDataRequestError::Length {
            actual: bytes.len(),
        });
    };
    let payload = <&[u8; N]>::try_from(payload).map_err(|_| FixedAppDataRequestError::Length {
        actual: bytes.len(),
    })?;
    if *actual_package_id != package_id {
        return Err(FixedAppDataRequestError::PackageId {
            value: *actual_package_id,
        });
    }
    if *actual_command_id != command_id {
        return Err(FixedAppDataRequestError::Command {
            value: *actual_command_id,
        });
    }
    Ok(payload)
}

/// Parse a package-prefixed app-data command and borrow its remaining payload.
#[must_use]
pub fn parse_app_data_command<C>(bytes: &[u8], package_id: u8) -> Option<(C, &[u8])>
where
    C: TryFrom<u8>,
{
    let [actual_package_id, command_id, payload @ ..] = bytes else {
        return None;
    };
    (*actual_package_id == package_id)
        .then(|| {
            C::try_from(*command_id)
                .ok()
                .map(|command| (command, payload))
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::{FixedAppDataRequestError, parse_app_data_command, parse_fixed_app_data_request};

    #[derive(Debug, PartialEq, Eq)]
    enum Command {
        One,
    }

    impl TryFrom<u8> for Command {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            (value == 1).then_some(Self::One).ok_or(())
        }
    }

    #[test]
    fn fixed_request_parser_validates_shape_package_and_command() {
        assert_eq!(
            parse_fixed_app_data_request::<1>(&[1, 2, 3], 1, 2),
            Ok(&[3])
        );
        assert_eq!(
            parse_fixed_app_data_request::<1>(&[1, 2], 1, 2),
            Err(FixedAppDataRequestError::Length { actual: 2 })
        );
        assert_eq!(
            parse_fixed_app_data_request::<1>(&[9, 2, 3], 1, 2),
            Err(FixedAppDataRequestError::PackageId { value: 9 })
        );
        assert_eq!(
            parse_fixed_app_data_request::<1>(&[1, 9, 3], 1, 2),
            Err(FixedAppDataRequestError::Command { value: 9 })
        );
    }

    #[test]
    fn command_parser_borrows_payload_after_valid_header() {
        assert_eq!(
            parse_app_data_command::<Command>(&[7, 1, 2, 3], 7),
            Some((Command::One, &[2, 3][..]))
        );
        assert_eq!(parse_app_data_command::<Command>(&[8, 1], 7), None);
        assert_eq!(parse_app_data_command::<Command>(&[7, 2], 7), None);
    }
}
