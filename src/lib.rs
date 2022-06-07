//! This crate provides a rust implementation of the [KATCP](https://katcp-python.readthedocs.io/en/latest/_downloads/361189acb383a294be20d6c10c257cb4/NRF-KAT7-6.0-IFCE-002-Rev5-1.pdf)
//! monitor and control protocol, as described by the Karoo Array Telescope (KAT) project from the Square Kilometer Array (SKA) South Africa group.
//!
//! ## Description
//!
//! From the official specification:
//! > Broadly speaking, KATCP consists of newline-separated text messages sent asynchronously over a TCP/IP
//! > stream. There are three categories of messages: requests, replies and informs. Request messages expect some
//! > sort of acknowledgement. Reply messages acknowledge requests. Inform messages require no acknowledgement
//! > Inform messages are of two types: those sent synchronously as part of a reply and those sent asynchronously.
//!
//! The details of orchestrating a client or server for this protocol is not the goal of this crate. Rather, this crate
//! only provides the core [Message](protocol::Message) type and the required message formats. It is up to the user of this crate how to design
//! a client or server. This is to allow this library to be small and portable and not to have to make any assumptions about
//! the eventual implementation.
//!
//! ## Messages
//!
//! Usually, you will interact with specific message types, these are organized in the same way they are presented in the spec, but will be reiteraeted here:
//!
//!
//! |                         Core                         |                 Log                 |                       Sensors                       |                        Multi-Client                        |
//! |------------------------------------------------------|-------------------------------------|-----------------------------------------------------|------------------------------------------------------------|
//! |             [Halt](messages::core::Halt)             |      [Log](messages::log::Log)      |     [SensorList](messages::sensors::SensorList)     |      [ClientList](messages::multi_client::ClientList)      |
//! |             [Help](messages::core::Help)             | [LogLevel](messages::log::LogLevel) | [SensorSampling](messages::sensors::SensorSampling) | [ClientConnected](messages::multi_client::ClientConnected) |
//! |          [Restart](messages::core::Restart)          |                                     |    [SensorValue](messages::sensors::SensorValue)    |                                                            |
//! |         [Watchdog](messages::core::Watchdog)         |                                     |   [SensorStatus](messages::sensors::SensorStatus)   |                                                            |
//! |      [VersionList](messages::core::VersionList)      |                                     |                                                     |                                                            |
//! |       [Disconnect](messages::core::Disconnect)       |                                     |                                                     |                                                            |
//! |   [VersionConnect](messages::core::VersionConnect)   |                                     |                                                     |                                                            |
//! | [InterfaceChanged](messages::core::InterfaceChanged) |                                     |                                                     |                                                            |

pub mod messages;
pub mod prelude;
pub mod protocol;
mod utils;
