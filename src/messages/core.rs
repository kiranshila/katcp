//! Messages dealing with connecting to a device, halting it or restarting it and querying basic information

use std::{collections::HashSet, fmt::Display};

use katcp_derive::{KatcpDiscrete, KatcpMessage};
use rustc_version;

use crate::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone)]
/// A Reply type that contains no data in the Ok branch or a message in the error branch
pub enum GenericReply {
    Ok,
    Error { ret_code: RetCode, message: String },
}

impl ToKatcpArguments for GenericReply {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            Self::Ok => vec![(RetCode::Ok).to_argument()],
            Self::Error { ret_code, message } => {
                vec![ret_code.to_argument(), message.clone().to_argument()]
            }
        }
    }
}

impl FromKatcpArguments for GenericReply {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let ret_code = RetCode::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        Ok(match ret_code {
            RetCode::Ok => Self::Ok,
            _ => Self::Error {
                ret_code,
                message: String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
            },
        })
    }
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Requesting a Halt should trigger a software halt
/// It is expected to close the connection and put the
/// software and hardware into a state where it is safe to power down. The reply message should be sent just
/// before the halt occurs
pub enum Halt {
    Request,
    Reply(GenericReply),
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// A Reply type that is an integer in the Ok branch or a message in the error branch
pub enum IntReply {
    Ok { num: u32 },
    Error { ret_code: RetCode, message: String },
}

impl ToKatcpArguments for IntReply {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            Self::Ok { num } => {
                vec![(RetCode::Ok).to_argument(), num.to_argument()]
            }
            Self::Error { ret_code, message } => {
                vec![ret_code.to_argument(), message.clone().to_argument()]
            }
        }
    }
}

impl FromKatcpArguments for IntReply {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let ret_code = RetCode::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let num_or_msg = strings.next().ok_or(KatcpError::MissingArgument)?;
        Ok(match ret_code {
            RetCode::Ok => Self::Ok {
                num: u32::from_argument(num_or_msg)?,
            },
            _ => Self::Error {
                ret_code,
                message: String::from_argument(num_or_msg)?,
            },
        })
    }
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// The core help message type
pub enum Help {
    /// Although the description is not intended to be machine readable, the preferred convention for describing
    /// the parameters and return values is to use a syntax like that seen on the right-hand side of a BNF produc-
    /// tion (as commonly seen in the usage strings of UNIX command-line utilities and the synopsis sections
    /// of man pages). Brackets ([]) surround optional arguments, vertical bars (|) separate choices, and ellipses
    /// (...) can be repeated.
    Inform {
        name: String,
        description: String,
    },
    /// Before sending a reply, the help request will send a number of #help inform messages. If no name
    /// parameter is sent the help request will return one inform message for each request available on the device.
    /// If a name parameter is specified, only an inform message for that request will be sent. On success the
    /// first reply parameter after the status code will contain the number of help inform messages generated by
    /// this request. If the name parameter does not correspond to a request on the device, a reply with a failure
    /// code and message should be sent
    Request {
        name: Option<String>,
    },
    Reply(IntReply),
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Requesting a restart should trigger a software reset. It is expected to close the connection, reload the
/// software and begin execution again, preferably without changing the hardware configuration (if possible).
/// It would end with the device being ready to accept new connections again. The reply should be sent before
/// the connection to the current client is closed.
pub enum Restart {
    Request,
    Reply(GenericReply),
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Requesting a watchdog may be sent by the client occasionally to check that the connection to the
/// device is still active. The device should respond with a success reply if it receives the watchdog request
pub enum Watchdog {
    Request,
    Reply(GenericReply),
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Before sending a reply the ?version-list command will send a
/// series of #version-list informs. The list of informs should include all of the roles and components
/// returned via #version-connect but may contain additional roles or components.
pub enum VersionList {
    Inform {
        /// the name of the role or component the version information applies to
        name: String,
        /// a string identifying the version of the component. Individual components may define the structure
        /// of this argument as they choose. In the absence of other information clients should treat it as
        /// an opaque string
        version: String,
        /// a unique identifier for a particular instance of a component. This is either the "build-state" or "serial-number"
        uuid: String,
    },
    Request,
    /// The [`VersionList`] reply's Ok branch will contain the number of informs that were sent
    Reply(IntReply),
}

// Async informs, these only have `Inform` fields

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Sent to the client by the device shortly before the client is disconnected. In the case where a client is being
/// disconnected because a new client has connected, the message should include
/// the IP number and port of the new client for tracking purposes.
pub enum Disconnect {
    Inform { message: String },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// Flags from [VersionConnect] that indicate the device's features
pub enum ProtocolFlags {
    /// the server supports multiple clients. Absence of this flag indicates that only a single client is supported
    MultiClient,
    /// the server supports message identifiers
    MessageIds,
    /// the server provides request timeout hints
    TimeoutHints,
    /// the server supports setting sensor sampling in bulk
    BulkSampling,
}

impl Display for ProtocolFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ProtocolFlags::MultiClient => "M",
            ProtocolFlags::MessageIds => "I",
            ProtocolFlags::TimeoutHints => "T",
            ProtocolFlags::BulkSampling => "B",
        })
    }
}

impl TryFrom<String> for ProtocolFlags {
    type Error = KatcpError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "M" => Ok(Self::MultiClient),
            "I" => Ok(Self::MessageIds),
            "T" => Ok(Self::TimeoutHints),
            "B" => Ok(Self::BulkSampling),
            _ => Err(KatcpError::BadArgument),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// The three different types of [`VersionConnect`] inform messages
pub enum VersionConnectInform {
    /// The version of katcp and the options it supports
    KatcpProtocol {
        major: u32,
        minor: u32,
        flags: HashSet<ProtocolFlags>,
    },
    /// Specifies the specific katcp library that the device is using
    KatcpLibrary {
        version: String,
        build_state: String,
    },
    /// Specifies API version and build state
    KatcpDevice {
        api_version: String,
        device: KatcpAddress,
        build_state: String,
    },
    /// Fallback for the other custom messages
    Custom {
        name: String,
        version: String,
        info: Option<String>,
    },
}

impl ToKatcpArguments for VersionConnectInform {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            VersionConnectInform::KatcpProtocol {
                major,
                minor,
                flags,
            } => {
                let flags = flags
                    .iter()
                    .map(|f| f.to_string())
                    .reduce(|current, next| current + &next);
                let flag_str = flags.map_or("".to_owned(), |s| format!("-{}", s));
                vec![
                    "katcp-protocol".to_owned(),
                    format!("{}.{}{}", major, minor, flag_str),
                ]
            }
            VersionConnectInform::KatcpLibrary {
                version,
                build_state,
            } => vec![
                "katcp-library".to_owned(),
                version.to_argument(),
                build_state.to_argument(),
            ],
            VersionConnectInform::KatcpDevice {
                api_version,
                device,
                build_state,
            } => vec![
                "katcp-device".to_owned(),
                api_version.to_argument(),
                device.to_argument(),
                build_state.to_argument(),
            ],
            VersionConnectInform::Custom {
                name,
                version,
                info,
            } => {
                let mut prelude = vec![name.to_argument(), version.to_argument()];
                if let Some(s) = info {
                    prelude.push(s.to_argument());
                }
                prelude
            }
        }
    }
}

impl FromKatcpArguments for VersionConnectInform {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let inform_type = strings.next().ok_or(KatcpError::MissingArgument)?;
        match inform_type.as_str() {
            "katcp-protocol" => {
                let version_str = strings.next().ok_or(KatcpError::MissingArgument)?;
                let (major, minor_and_flags) =
                    version_str.split_once('.').ok_or(KatcpError::BadArgument)?;
                let major = major.parse().map_err(|_| KatcpError::BadArgument)?;
                let split = minor_and_flags.split_once('-');
                let (minor, flags) = if let Some((minor, flagset)) = split {
                    let flags = flagset
                        .chars()
                        .map(|c| c.to_string().try_into())
                        .collect::<Result<HashSet<_>, _>>()?;
                    (minor.parse().map_err(|_| KatcpError::BadArgument)?, flags)
                } else {
                    (
                        minor_and_flags // if let didn't match, so minor_and_flags is only minor
                            .parse()
                            .map_err(|_| KatcpError::BadArgument)?,
                        HashSet::new(),
                    )
                };
                Ok(Self::KatcpProtocol {
                    major,
                    minor,
                    flags,
                })
            }
            "katcp-library" => Ok(Self::KatcpLibrary {
                version: String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
                build_state: String::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
            }),
            "katcp-device" => Ok(Self::KatcpDevice {
                api_version: String::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
                device: KatcpAddress::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
                build_state: String::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
            }),
            _ => Ok(Self::Custom {
                name: inform_type,
                version: String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
                info: strings.next().map(String::from_argument).transpose()?,
            }),
        }
    }
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Sent to the client when it connects. These inform messages use the same argument format as [`VersionList`]
/// and all roles and components declared via [`VersionConnect`] should be included in the informs sent in
/// response to [`VersionList`].
pub enum VersionConnect {
    Inform(VersionConnectInform),
}

impl VersionConnect {
    /// Returns a [`VersionConnect`] of `name:`[`VersionConnectName::KatcpLibrary`] for this rust library
    pub fn library() -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let target = rustc_version::version().unwrap();
        Self::Inform(VersionConnectInform::KatcpLibrary {
            version: format!("katcp-{}", version),
            build_state: format!("rustc-{}", target),
        })
    }
}

#[derive(KatcpDiscrete, Debug, PartialEq, Eq, Clone)]
/// On specific [`InterfaceChanged`] informs, these specify how precisely the interface was changed
pub enum ChangeSpecificationAction {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// The sum type of the different [`InterfaceChanged`] informs
pub enum InterfaceChangeInform {
    SensorList,
    RequestList,
    Sensor {
        name: String,
        action: ChangeSpecificationAction,
    },
    Request {
        name: String,
        action: ChangeSpecificationAction,
    },
}

impl ToKatcpArguments for InterfaceChangeInform {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            Self::SensorList => vec!["sensor-list".to_owned()],
            Self::RequestList => vec!["request-list".to_owned()],
            Self::Sensor { name, action } => vec![
                "sensor".to_owned(),
                name.to_argument(),
                action.to_argument(),
            ],
            Self::Request { name, action } => vec![
                "request".to_owned(),
                name.to_argument(),
                action.to_argument(),
            ],
        }
    }
}
impl FromKatcpArguments for InterfaceChangeInform {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let inform_type = strings.next().ok_or(KatcpError::MissingArgument)?;
        match inform_type.as_str() {
            "sensor-list" => Ok(Self::SensorList),
            "request-list" => Ok(Self::RequestList),
            "sensor" => Ok(Self::Sensor {
                name: String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
                action: ChangeSpecificationAction::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
            }),
            "request" => Ok(Self::Request {
                name: String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
                action: ChangeSpecificationAction::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
            }),
            _ => Err(KatcpError::BadArgument),
        }
    }
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Only required for dynamic devices, i.e. devices that may change their katcp interface during a connection.
/// Sent to the client by the device to indicate that the katcp interface has changed. Passing no arguments
/// with the inform implies that the whole katcp interface may have changed. The optional parameters allow
/// more fine grained specification of what changed:
pub enum InterfaceChanged {
    Inform(InterfaceChangeInform),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::common::roundtrip_test;

    #[test]
    fn test_halt() {
        roundtrip_test(Halt::Request);
        roundtrip_test(Halt::Reply(GenericReply::Ok));
        roundtrip_test(Halt::Reply(GenericReply::Error {
            ret_code: RetCode::Fail,
            message: "You Messed Up".to_owned(),
        }));
    }

    #[test]
    fn test_help() {
        roundtrip_test(Help::Request { name: None });
        roundtrip_test(Help::Request {
            name: Some("my_special_message".to_owned()),
        });
        roundtrip_test(Help::Reply(IntReply::Ok { num: 10 }));
        roundtrip_test(Help::Reply(IntReply::Error {
            ret_code: RetCode::Fail,
            message: "Something went wrong".to_owned(),
        }));
    }

    #[test]
    fn test_restart() {
        roundtrip_test(Restart::Request);
        roundtrip_test(Restart::Reply(GenericReply::Ok));
        roundtrip_test(Restart::Reply(GenericReply::Error {
            ret_code: RetCode::Fail,
            message: "You Messed Up".to_owned(),
        }));
    }

    #[test]
    fn test_watchdog() {
        roundtrip_test(Watchdog::Request);
        roundtrip_test(Watchdog::Reply(GenericReply::Ok));
    }

    #[test]
    fn test_version_list() {
        roundtrip_test(VersionList::Request);
        roundtrip_test(VersionList::Inform {
            name: "my-special-device".to_owned(),
            version: "0.1.2.3rev10".to_owned(),
            uuid: "asdb132b34j".to_owned(),
        });
        roundtrip_test(VersionList::Reply(IntReply::Ok { num: 300 }));
        roundtrip_test(VersionList::Reply(IntReply::Error {
            ret_code: RetCode::Invalid,
            message: "Please fix me\nThis is bad".to_owned(),
        }))
    }

    #[test]
    fn test_disconnect() {
        roundtrip_test(Disconnect::Inform {
            message: "New client connected from 192.168.1.100:24500".to_owned(),
        });
    }

    #[test]
    fn test_version_connect() {
        roundtrip_test(VersionConnect::library());
        roundtrip_test(VersionConnect::Inform(
            VersionConnectInform::KatcpProtocol {
                major: 5,
                minor: 1,
                flags: HashSet::from([ProtocolFlags::MultiClient, ProtocolFlags::BulkSampling]),
            },
        ));
        roundtrip_test(VersionConnect::Inform(
            VersionConnectInform::KatcpProtocol {
                major: 5,
                minor: 0,
                flags: HashSet::new(),
            },
        ));
        roundtrip_test(VersionConnect::Inform(VersionConnectInform::Custom {
            name: "kernel".to_owned(),
            version: "4.4.9-v7+".to_owned(),
            info: Some("#884 SMP Fri May 6 17:28:59 BST 2016".to_owned()),
        }));
    }

    #[test]
    fn test_interface_changed() {
        roundtrip_test(InterfaceChanged::Inform(InterfaceChangeInform::SensorList));
        roundtrip_test(InterfaceChanged::Inform(InterfaceChangeInform::RequestList));
        roundtrip_test(InterfaceChanged::Inform(InterfaceChangeInform::Sensor {
            name: "name.of.fancy.sensor".to_owned(),
            action: ChangeSpecificationAction::Added,
        }));
        roundtrip_test(InterfaceChanged::Inform(InterfaceChangeInform::Request {
            name: "name.of.fancy.sensor".to_owned(),
            action: ChangeSpecificationAction::Removed,
        }));
    }
}
