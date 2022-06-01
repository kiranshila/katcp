//! This module provides the core katcp message type [`Log`]
//!
//! # Examples
//! ```rust
//! use katcp::{messages::log::Log,protocol::Message};
//! let log: Log = r"#log warn 10000 device.sub-system Something\_may\_be\_wrong"
//!     .parse::<Message>()
//!     .unwrap()
//!     .try_into()
//!     .unwrap();
//! ```

use crate::{
    messages::common::{KatcpMessage, RetCode},
    protocol::{KatcpError, Message, MessageKind, MessageResult},
    utils::{escape, unescape},
};
use chrono::{DateTime, TimeZone, Utc};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, PartialEq, Eq)]
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

impl FromStr for LogLevel {
    type Err = KatcpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = match s {
            "off" => LogLevel::Off,
            "fatal" => LogLevel::Fatal,
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            "all" => LogLevel::All,
            _ => return Err(KatcpError::BadArgument),
        };
        Ok(level)
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self {
            LogLevel::Off => "off",
            LogLevel::Fatal => "fatal",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
            LogLevel::All => "all",
        };
        write!(f, "{}", level)
    }
}

#[derive(Debug, PartialEq, Eq)]
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

impl TryFrom<Message> for Log {
    type Error = KatcpError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        // First we ensure that this message is actually log
        if message.name != "log" {
            return Err(KatcpError::IncorrectType);
        }

        // Parse the right kind out of the arguments
        match message.kind {
            MessageKind::Request => request_from_message(message),
            MessageKind::Reply => reply_from_message(message),
            MessageKind::Inform => inform_from_message(message),
        }
    }
}

type LogResult = Result<Log, KatcpError>;

fn request_from_message(message: Message) -> LogResult {
    message
        .arguments
        .get(0)
        .map(|s| s.parse())
        .transpose()
        .map(Log::request)
}

fn reply_from_message(message: Message) -> LogResult {
    let ret_code = message
        .arguments
        .get(0)
        .ok_or(KatcpError::MissingArgument)?
        .as_str()
        .parse()?;
    let level = message
        .arguments
        .get(1)
        .ok_or(KatcpError::MissingArgument)?
        .as_str()
        .parse()?;
    Ok(Log::reply(ret_code, level))
}

fn str_to_timestamp(s: &str) -> Result<DateTime<Utc>, KatcpError> {
    let dot_idx = s.find('.').unwrap_or_else(|| s.chars().count());
    let (sec, _) = s.split_at(dot_idx);
    Ok(Utc.timestamp(sec.parse().map_err(|_| KatcpError::BadArgument)?, 0_u32))
}

fn timestamp_to_str(t: &DateTime<Utc>) -> String {
    let secs = t.timestamp() as f64;
    let nano = t.timestamp_subsec_nanos();
    let frac = (nano as f64) / 1e9;
    format!("{}", secs + frac)
}

// We require here that message is named log and that it's kind is Inform
fn inform_from_message(message: Message) -> LogResult {
    let level = message
        .arguments
        .get(0)
        .ok_or(KatcpError::MissingArgument)?
        .as_str()
        .parse()?;

    let time = str_to_timestamp(
        message
            .arguments
            .get(1)
            .ok_or(KatcpError::MissingArgument)?
            .as_str(),
    )?;

    let name = unescape(
        message
            .arguments
            .get(2)
            .ok_or(KatcpError::MissingArgument)?,
    );
    let msg = unescape(
        message
            .arguments
            .get(3)
            .ok_or(KatcpError::MissingArgument)?,
    );
    Ok(Log::inform(level, time, name, msg))
}

impl KatcpMessage for Log {
    fn into_message(self, id: Option<u32>) -> MessageResult {
        let (kind, args) = match self {
            log @ Log::Inform { .. } => args_from_inform(&log)?,
            log @ Log::Reply { .. } => args_from_reply(&log)?,
            log @ Log::Request { .. } => args_from_request(&log)?,
        };
        // Safety: we're escaping the strings when we build the args,
        // so we're guaranteed the things are ok
        Ok(unsafe { Message::new_unchecked(kind, "log", id, args) })
    }
}

// We require here that log is indeed the reply variant
fn args_from_reply(log: &Log) -> Result<(MessageKind, Vec<String>), KatcpError> {
    if let Log::Reply { ret_code, level } = log {
        let level = level.to_string();
        let ret_code = ret_code.to_string();
        Ok((MessageKind::Reply, vec![ret_code, level]))
    } else {
        Err(KatcpError::BadArgument)
    }
}

// We require here that log is indeed the request variant
fn args_from_request(log: &Log) -> Result<(MessageKind, Vec<String>), KatcpError> {
    if let Log::Request { level } = log {
        Ok((
            MessageKind::Request,
            match level {
                Some(s) => vec![s.to_string()],
                None => Vec::new(),
            },
        ))
    } else {
        Err(KatcpError::BadArgument)
    }
}

// We require here that log is indeed the inform variant
fn args_from_inform(log: &Log) -> Result<(MessageKind, Vec<String>), KatcpError> {
    if let Log::Inform {
        level,
        timestamp,
        name,
        message,
    } = log
    {
        let level = level.to_string();
        let time = timestamp_to_str(timestamp);
        Ok((
            MessageKind::Inform,
            vec![level, time, escape(name), escape(message)],
        ))
    } else {
        Err(KatcpError::BadArgument)
    }
}

#[cfg(test)]
mod log_tests {
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
                    &timestamp_to_str(&time),
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
                    &timestamp_to_str(&time),
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
