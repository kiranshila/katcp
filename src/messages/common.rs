use chrono::{DateTime, TimeZone, Utc};
use katcp_derive::KatcpDiscrete;

use crate::{
    protocol::{KatcpError, Message, MessageResult},
    utils::{escape, unescape},
};

/// The trait that specific katcp messages should implement
pub trait KatcpMessage: TryFrom<Message> {
    fn to_message(&self, id: Option<u32>) -> MessageResult;
}

/// The trait that is implemented for all the fundamental katcp types
/// as well any user defined types such as (C-like) enums
pub trait ToKatcpArgument {
    /// Create a katcp message argument (String) from a self
    fn to_argument(&self) -> String;
}

pub trait FromKatcpArgument
where
    Self: Sized,
{
    type Err; // Not Error as to not clash with Self being an enum with an `Error` variant
    /// Create a self from a katcp message argument (String), potentially erroring
    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err>;
}

pub trait KatcpArgument: ToKatcpArgument + FromKatcpArgument {}

// Default KatcpArgument - "Trait Marker"
impl<T> KatcpArgument for T where T: ToKatcpArgument + FromKatcpArgument {}

// ---- Implementations for the "core" KatcpTypes

// str
impl ToKatcpArgument for str {
    fn to_argument(&self) -> String {
        escape(self)
    }
}

impl ToKatcpArgument for String {
    fn to_argument(&self) -> String {
        escape(self)
    }
}

impl FromKatcpArgument for String {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        Ok(unescape(s.as_ref()))
    }
}

// DateTime<Utc>
impl ToKatcpArgument for DateTime<Utc> {
    fn to_argument(&self) -> String {
        let secs = self.timestamp() as f64;
        let nano = self.timestamp_subsec_nanos();
        let frac = nano as f64 / 1e9;
        format!("{}", secs + frac)
    }
}

impl FromKatcpArgument for DateTime<Utc> {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        let fractional: f64 = s.as_ref().parse().map_err(|_| KatcpError::BadArgument)?;
        let secs = fractional as i64;
        let nanos = (fractional.fract() * 1e9) as u32;
        Ok(Utc.timestamp(secs, nanos))
    }
}

// Option
impl<T> ToKatcpArgument for Option<T>
where
    T: ToKatcpArgument,
{
    fn to_argument(&self) -> String {
        match self {
            Some(v) => v.to_argument(),
            None => r"\@".to_owned(),
        }
    }
}

impl<E, T> FromKatcpArgument for Option<T>
where
    T: FromKatcpArgument<Err = E>,
{
    type Err = E;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        match s.as_ref() {
            r"\@" => Ok(None),
            _ => Ok(Some(T::from_argument(s)?)),
        }
    }
}

// Return Code
#[derive(KatcpDiscrete, Debug, PartialEq, Eq, Copy, Clone)]
/// Return codes that form the first parameter of [`KatcpMethod::Reply`]
pub enum RetCode {
    /// Request successfully processed. Further arguments are request-specific
    Ok,
    /// Request malformed. Second argument is a human-readable description of the error
    Invalid,
    /// Valid request that could not be processed. Second argument is a human-readable description of the error.
    Fail,
}

// TODO integer, float, boolean, address

/// Convienence method for round-trip testing
pub fn roundtrip_test<T, E>(message: T)
where
    E: std::fmt::Debug,
    T: KatcpMessage + PartialEq + std::fmt::Debug + TryFrom<Message, Error = E>,
{
    let raw = message.to_message(None).unwrap();
    let s = raw.to_string();
    // Print the middle, we're using this in tests, so we'll only see it on fails
    println!("Katcp Payload:\n{}", s);
    let raw_test: Message = (s.as_str()).try_into().unwrap();
    let message_test = raw_test.try_into().unwrap();
    assert_eq!(message, message_test)
}

#[cfg(test)]
mod test_arguments {
    use super::*;

    #[test]
    fn test_string() {
        let s = "This is a message with spaces\n";
        assert_eq!(s, String::from_argument(s.to_argument()).unwrap());
    }

    #[test]
    fn test_timestamp() {
        let ts = Utc.timestamp(42069, 42069000);
        assert_eq!(
            ts,
            DateTime::<Utc>::from_argument(ts.to_argument()).unwrap()
        );
    }

    #[test]
    fn test_option() {
        let s = Some("\tFoo a bar\n".to_owned());
        assert_eq!(s, Option::<String>::from_argument(s.to_argument()).unwrap())
    }

    #[test]
    fn test_ret_code() {
        let code = RetCode::Invalid;
        assert_eq!(code, RetCode::from_argument(code.to_argument()).unwrap())
    }
}
