use crate::{
    messages::common::{KatcpMessage, RetCode},
    protocol::{KatcpError, Message, MessageKind, MessageResult},
    utils::{escape, unescape},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

#[derive(Debug, PartialEq, Eq)]
pub enum Log {
    Inform {
        level: LogLevel,
        timestamp: SystemTime,
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
        timestamp: SystemTime,
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
            MessageKind::Request => todo!(),
            MessageKind::Reply => todo!(),
            MessageKind::Inform => inform_from_message(message),
        }
    }
}

// We require here that message is named log and that it's kind is Inform
fn inform_from_message(message: Message) -> Result<Log, KatcpError> {
    let level = match message
        .arguments
        .get(0)
        .ok_or(KatcpError::MissingArgument)?
        .as_str()
    {
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
    let time = UNIX_EPOCH
        + Duration::from_secs_f32(
            message
                .arguments
                .get(1)
                .ok_or(KatcpError::MissingArgument)?
                .parse()
                .map_err(|_| KatcpError::BadArgument)?,
        );
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
    fn into_message(self, kind: MessageKind, id: Option<u32>) -> MessageResult {
        let (kind, args) = match self {
            log @ Log::Inform { .. } => message_from_inform(&log, id)?,
            Log::Reply { ret_code, level } => todo!(),
            Log::Request { level } => todo!(),
        };
        todo!()
    }
}

// We require here that log is indeed the inform variant
fn message_from_inform(
    log: &Log,
    id: Option<u32>,
) -> Result<(MessageKind, [String; 4]), KatcpError> {
    if let Log::Inform {
        level,
        timestamp,
        name,
        message,
    } = log
    {
        let level = match level {
            LogLevel::Off => "off",
            LogLevel::Fatal => "fatal",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
            LogLevel::All => "all",
        };
        let time = timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|_| KatcpError::Unknown)?
            .as_secs_f32()
            .to_string();
        Ok((
            MessageKind::Inform,
            [level.to_owned(), time, escape(&name), escape(&message)],
        ))
    } else {
        Err(KatcpError::BadArgument)
    }
}

// #[cfg(test)]
// mod log_tests {
//     use super::*;

//     #[test]
//     fn test_log() {
//         let now = SystemTime::now();
//         println!(
//             "{:#?}",
//             now.duration_since(UNIX_EPOCH).unwrap().as_secs_f32(I)
//         );
//         let msg_str = format!(
//             "#log warn {} device.foo.bar Something\\_was\\_kinda\\_wrong",
//             now.duration_since(UNIX_EPOCH).unwrap().as_secs_f32()
//         );
//         println!("{}", msg_str);
//         assert_eq!(
//             Log::new(
//                 None,
//                 LogLevel::Warn,
//                 now,
//                 "device.foo.bar",
//                 "Something was kinda wrong"
//             ),
//             msg_str.parse::<Message>().unwrap().try_into().unwrap()
//         );
//     }
// }
