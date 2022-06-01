use crate::protocol::{KatcpError, Message, MessageResult};
use std::{fmt::Display, str::FromStr};

/// The trait that specific katcp messages should implement
pub trait KatcpMessage: TryFrom<Message> {
    fn into_message(self, id: Option<u32>) -> MessageResult;
}

#[derive(Debug, PartialEq, Eq)]
/// Return codes that form the first parameter of [`KatcpMethod::Reply`]
pub enum RetCode {
    /// Request successfully processed. Further arguments are request-specific
    Ok,
    /// Request malformed. Second argument is a human-readable description of the error
    Invalid,
    /// Valid request that could not be processed. Second argument is a human-readable description of the error.
    Fail,
}

impl FromStr for RetCode {
    type Err = KatcpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = match s {
            "ok" => RetCode::Ok,
            "invalid" => RetCode::Invalid,
            "fail" => RetCode::Fail,
            _ => return Err(KatcpError::BadArgument),
        };
        Ok(level)
    }
}

impl Display for RetCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self {
            RetCode::Ok => "ok",
            RetCode::Invalid => "invalid",
            RetCode::Fail => "fail",
        };
        write!(f, "{}", level)
    }
}
