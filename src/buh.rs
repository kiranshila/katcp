#[derive(Debug)]
/// The return codes for a `MessageType::Reply`
pub enum RetCode {
    /// Request successfully processed. Further arguments are request-specific.
    Ok,
    /// Request malformed. Second argument is a human-reaedable description of the error
    Invalid,
    /// Valid request that could not be processed. Second argument is a human-readable description of the error.
    Fail,
}

/// The valid katcp "sensor" statuses
pub enum SensorStatus {
    /// The sensor is in the process of being initialized and no value has yet been
    /// seen. Sensors should not remain in this state indefinitely.
    Unknown,
    /// The sensor reading is within the expected range of nominal operating values.
    Nominal,
    /// The sensor reading is outside the nominal operating range.
    Warn,
    /// The sensor reading indicates a critical condition for the device.
    Error,
    /// Taking a sensor reading failed and seems unlikely to succeed in future
    /// without maintenance.
    Failure,
    /// The sensor could not be reached. This should only be used by a server that
    /// is proxying the sensor for another KATCP device. A sensor that is read by
    /// the server from a source other than another KATCP device should not be set
    /// to this status.
    Unreachable,
    /// The sensor is inactive; while the sensor does not provide a valid value, this
    /// status does not represent a failure condition. It could indicate that optional
    /// sensing hardware is not connected; in multi-mode devices it may indicate
    /// that a particular sensor is not applicable to the current mode of operation.
    Inactive,
}

impl SensorStatus {
    /// Returns if a given `SensorStatus` is valid
    pub fn is_valid(self) -> bool {
        if let SensorStatus::Nominal | SensorStatus::Warn | SensorStatus::Error = self {
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_validity() {
        assert!(!SensorStatus::Unknown.is_valid());
        assert!(SensorStatus::Nominal.is_valid());
        assert!(SensorStatus::Warn.is_valid());
        assert!(SensorStatus::Error.is_valid());
        assert!(!SensorStatus::Failure.is_valid());
        assert!(!SensorStatus::Unreachable.is_valid());
        assert!(!SensorStatus::Inactive.is_valid());
    }
}
