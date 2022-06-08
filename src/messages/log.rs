//! The core katcp message type [`Log`]
//!
//! # Examples
//! ```rust
//! use katcp::{messages::log::Log, protocol::Message};
//! let log: Log = r"#log warn 10000 device.sub-system Something\_may\_be\_wrong"
//!     .try_into()
//!     .unwrap();
//! ```
use katcp_derive::{KatcpDiscrete, KatcpMessage};

use crate::prelude::*;

#[derive(KatcpDiscrete, Debug, PartialEq, Eq, Copy, Clone)]
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

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Log messages
pub enum Log {
    Inform {
        level: LogLevel,
        timestamp: KatcpTimestamp,
        name: String,
        message: String,
    },
    Reply {
        ret_code: RetCode,
        level: LogLevel,
    },
    Request {
        level: Option<LogLevel>,
    },
}

impl Log {
    /// Constructs a new [`Log`] inform message
    pub fn inform<T: AsRef<str>, U: AsRef<str>>(
        level: LogLevel,
        timestamp: KatcpTimestamp,
        name: T,
        message: U,
    ) -> Self {
        Self::Inform {
            level,
            timestamp,
            name: name.as_ref().to_owned(),
            message: message.as_ref().to_owned(),
        }
    }

    /// Constructs a new [`Log`] request message
    pub fn request(level: Option<LogLevel>) -> Self {
        Self::Request { level }
    }

    /// Constructs a new [`Log`] reply message
    pub fn reply(ret_code: RetCode, level: LogLevel) -> Self {
        Self::Reply { ret_code, level }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::messages::common::roundtrip_test;

    #[test]
    fn test_log() {
        roundtrip_test(Log::Inform {
            level: LogLevel::Error,
            timestamp: Utc.timestamp(420, 3),
            name: "foo.bar.baz".to_owned(),
            message: "This is a test message".to_owned(),
        });
        roundtrip_test(Log::Reply {
            ret_code: RetCode::Ok,
            level: LogLevel::Trace,
        });
        roundtrip_test(Log::Request {
            level: Some(LogLevel::Info),
        });
    }
}
