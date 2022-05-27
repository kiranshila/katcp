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

#[derive(Debug)]
/// Katcp log level, these match the typical log level heiarchy of log4j, syslog, etc
pub enum LogLevel {
    /// # Definition
    /// OFF is the highest possible logging level and is intended to turn logging off.
    /// # Content
    /// No information. Devices should never log messages directly to the OFF logging level
    Off,
    /// # Definition
    /// The device has failed. There is no workaround. Recovery is not possible.
    /// # Content
    /// The logged message should capture as much system state information as possible in order to assist with
    /// debugging the problem. Logging information at this level should not directly impact the performance
    /// of the device.
    Fatal,
    /// # Definition
    /// An error has occurred. A function or operation did not complete successfully. A workaround may be
    /// possible. The device can continue, potentially with degraded functionality. Logging information at this
    /// level should not directly impact the performance of the device.
    /// # Content
    /// The error message should capture detailed information relating to the event that has occurred.
    Error,
    /// # Definition
    /// A condition was detected which may lead to functional degradation (e.g. an anomaly threshold has been
    /// crossed), but the device is still fully functional. Logging information at this level should not directly
    /// impact the performance of the device.
    /// # Content
    /// The warning message should capture the information relating to what functional degradation may occur
    /// and list thresholds that have been exceeded.
    Warn,
    /// # Definition
    /// This level of logging should give information about workflow at a coarse-grained level. Information at
    /// this level may be considered useful for tracking process flow. Logging information at this level should
    /// not directly impact the performance of the device.
    /// # Content
    /// The information message should capture information relating to the operation that has completed.
    Info,
    /// # Definition
    /// Verbose output used for detailed analysis and debugging of a device. Logging information at this level
    /// may impact the performance of the device.
    /// # Content
    /// This level of logging should show workflow at a fine-grained level. Information relating to parameters,
    /// data values and device states should be reported.
    Debug,
    /// # Definition
    /// Extremely verbose output for detailed analysis and debugging of a device. Logging information at this
    /// level may impact the performance of the device.
    /// # Content
    /// This level of logging should show function call stacks and provide a high level of debug information.
    Trace,
    /// # Definition
    /// ALL is the lowest possible logging level and is intended to turn on all logging.
    /// # Content
    /// Logging will occur at the most detailed level. Devices should never log messages directly to the ALL
    /// logging level.
    All,
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
