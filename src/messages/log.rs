//! This module provides the core katcp message type [`Log`]
//!
//! # Examples
//! ```rust
//! use katcp::{messages::log::Log,protocol::Message};
//! let log: Log = r"#log warn 10000 device.sub-system Something\_may\_be\_wrong"
//!     .try_into()
//!     .unwrap();
//! ```
use crate::{
    messages::common::{FromKatcpArgument, KatcpMessage, RetCode, ToKatcpArgument},
    protocol::{KatcpError, Message, MessageKind, MessageResult},
};
use chrono::{DateTime, Utc};
use katcp_derive::{KatcpDiscrete, KatcpMessage};

#[derive(KatcpDiscrete, Debug, PartialEq, Eq)]
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

#[derive(KatcpMessage, Debug, PartialEq, Eq)]
pub enum Log {
    Inform {
        level: LogLevel,
        timestamp: DateTime<Utc>,
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
        timestamp: DateTime<Utc>,
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
mod log_tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_to_message() {
        let time = Utc.timestamp(1234567, 1234567);
        assert_eq!(
            Message::new(MessageKind::Request, "log", None, vec!["fatal"]).unwrap(),
            Log::request(Some(LogLevel::Fatal))
                .into_message(None)
                .unwrap()
        );
        assert_eq!(
            Message::new(MessageKind::Reply, "log", None, vec!["ok", "trace"]).unwrap(),
            Log::reply(RetCode::Ok, LogLevel::Trace)
                .into_message(None)
                .unwrap()
        );
        assert_eq!(
            Message::new(
                MessageKind::Inform,
                "log",
                None,
                vec![
                    "error",
                    "1234567.001234567",
                    "some.device.somewhere",
                    r"You\_goofed\_up",
                ],
            )
            .unwrap(),
            Log::inform(
                LogLevel::Error,
                time,
                "some.device.somewhere",
                "You goofed up",
            )
            .into_message(None)
            .unwrap()
        );
    }

    #[test]
    fn test_from_message() {
        let time = Utc.timestamp(1234567, 0);
        assert_eq!(
            Log::request(Some(LogLevel::Fatal)),
            Message::new(MessageKind::Request, "log", None, vec!["fatal"])
                .unwrap()
                .try_into()
                .unwrap()
        );
        assert_eq!(
            Log::reply(RetCode::Ok, LogLevel::Trace),
            Message::new(MessageKind::Reply, "log", None, vec!["ok", "trace"])
                .unwrap()
                .try_into()
                .unwrap()
        );
        assert_eq!(
            Log::inform(
                LogLevel::Error,
                time,
                "some.device.somewhere",
                "You goofed up"
            ),
            Message::new(
                MessageKind::Inform,
                "log",
                None,
                vec![
                    "error",
                    "1234567.0",
                    "some.device.somewhere",
                    r"You\_goofed\_up"
                ]
            )
            .unwrap()
            .try_into()
            .unwrap(),
        );
    }

    #[test]
    fn test_from_message_str() {
        assert_eq!(
            Log::inform(
                LogLevel::Warn,
                Utc.timestamp(100, 0),
                "foo.bar.baz",
                "Hey there kiddo"
            )
            .into_message(None)
            .unwrap(),
            r"#log warn 100 foo.bar.baz Hey\_there\_kiddo"
                .parse()
                .unwrap()
        );
        assert_eq!(
            Log::inform(
                LogLevel::Warn,
                Utc.timestamp(100, 0),
                "foo.bar.baz",
                "Hey there kiddo"
            )
            .into_message(Some(123))
            .unwrap(),
            r"#log[123] warn 100 foo.bar.baz Hey\_there\_kiddo"
                .parse()
                .unwrap()
        );
        assert_eq!(
            Log::inform(
                LogLevel::Error,
                Utc.timestamp(420, 69),
                "foo.bar.baz",
                "Hey there kiddo"
            )
            .into_message(Some(123))
            .unwrap(),
            r"#log[123] error 420.000000069 foo.bar.baz Hey\_there\_kiddo"
                .parse()
                .unwrap()
        );
    }
}
