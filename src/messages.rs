use crate::protocol::{KatcpError, Message, MessageMethod};
use std::time::Instant;

pub trait KatcpMessage: TryFrom<Message> + TryInto<Message> {
    fn name(&self) -> &str;
    fn method(&self) -> MessageMethod;
    fn id(&self) -> Option<u32>;
}

impl KatcpMessage for Message {
    fn name(&self) -> &str {
        &self.name
    }

    fn method(&self) -> MessageMethod {
        self.method
    }

    fn id(&self) -> Option<u32> {
        self.id
    }
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

pub struct Log {
    id: Option<u32>,
    level: LogLevel,
    timestamp: Instant,
    name: String,
    message: String,
}

impl TryFrom<Message> for Log {
    type Error = KatcpError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        todo!()
        // let level = match (message.arguments.get(1)?) {
        //     "off" => LogLevel::Off,
        //     "fatal" => LogLevel::Fatal,
        //     "error" => LogLevel::Error,
        //     "warn" => LogLevel::Warn,
        //     "info" => LogLevel::Info,
        //     "debug" => LogLevel::Debug,
        //     "trace" => LogLevel::Trace,
        //     "all" => LogLevel::All,
        //     _ => return Err(KatcpError::DeserializationError),
        // };
    }
}

impl TryInto<Message> for Log {
    type Error = KatcpError;

    fn try_into(self) -> Result<Message, Self::Error> {
        let level = match self.level {
            LogLevel::Off => "off",
            LogLevel::Fatal => "fatal",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
            LogLevel::All => "all",
        };
        let time = (self.timestamp.elapsed().as_millis() as u32).to_string();
        // Safety: A constructed message is going to be valid
        Ok(unsafe {
            Message::new_unchecked(
                MessageMethod::Inform,
                "log",
                self.id,
                &[level, &time, &self.name, &self.message],
            )
        })
    }
}

impl KatcpMessage for Log {
    fn name(&self) -> &str {
        "log"
    }

    fn method(&self) -> MessageMethod {
        MessageMethod::Inform
    }

    fn id(&self) -> Option<u32> {
        self.id
    }
}
