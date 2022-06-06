use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

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

/// Serializes the implemented type into an argument string
/// Implemented for all fundamental katcp types as well as any user-defined types
pub trait ToKatcpArgument {
    /// Create a katcp message argument (String) from a self
    fn to_argument(&self) -> String;
}

/// Deserializes an argument string into the implemented type
/// Implemented for all fundamental katcp types as well as any user-defined types
pub trait FromKatcpArgument
where
    Self: Sized,
{
    type Err; // Not Error as to not clash with Self being an enum with an `Error` variant
    /// Create a self from a katcp message argument (String), potentially erroring
    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err>;
}

/// A trait for serializing more complex types that return the full argument vector
pub trait ToKatcpArguments {
    fn to_arguments(&self) -> Vec<String>;
}

/// A trait for deserializing more complex types that consume an iterator of arguments
pub trait FromKatcpArguments
where
    Self: Sized,
{
    type Err;
    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err>;
}

// Marker Traits
pub trait KatcpArgument: ToKatcpArgument + FromKatcpArgument {}
pub trait KatcpArguments: ToKatcpArguments + FromKatcpArguments {}
impl<T> KatcpArgument for T where T: ToKatcpArgument + FromKatcpArgument {}
impl<T> KatcpArguments for T where T: ToKatcpArguments + FromKatcpArguments {}

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
/// Return codes that form the first parameter of replys
pub enum RetCode {
    /// Request successfully processed. Further arguments are request-specific
    Ok,
    /// Request malformed. Second argument is a human-readable description of the error
    Invalid,
    /// Valid request that could not be processed. Second argument is a human-readable description of the error.
    Fail,
}

impl ToKatcpArgument for u32 {
    fn to_argument(&self) -> String {
        self.to_string()
    }
}

impl FromKatcpArgument for u32 {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        s.as_ref().parse().map_err(|_| KatcpError::BadArgument)
    }
}

impl ToKatcpArgument for i32 {
    fn to_argument(&self) -> String {
        self.to_string()
    }
}

impl FromKatcpArgument for i32 {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        s.as_ref().parse().map_err(|_| KatcpError::BadArgument)
    }
}

impl ToKatcpArgument for bool {
    fn to_argument(&self) -> String {
        (if *self { "1" } else { "0" }).to_owned()
    }
}

impl FromKatcpArgument for bool {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        match s.as_ref() {
            "1" => Ok(true),
            "0" => Ok(false),
            _ => Err(KatcpError::BadArgument),
        }
    }
}

impl ToKatcpArgument for f32 {
    fn to_argument(&self) -> String {
        format!("{}", self)
    }
}

impl FromKatcpArgument for f32 {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        s.as_ref().parse().map_err(|_| KatcpError::BadArgument)
    }
}

/// Katcp addresses optionally have a port, so we need a sum type for the two native rust
/// types [`IpAddr`] and [`SocketAddr`], depending on whether we have a port
#[derive(Debug, PartialEq, Eq)]
pub enum KatcpAddress {
    Ip(IpAddr),
    Socket(SocketAddr),
}

impl ToKatcpArgument for KatcpAddress {
    fn to_argument(&self) -> String {
        match self {
            KatcpAddress::Ip(addr) => match addr {
                IpAddr::V4(addr) => addr.to_string(),
                IpAddr::V6(addr) => format!("[{}]", addr),
            },
            KatcpAddress::Socket(addr) => match addr {
                SocketAddr::V4(addr) => addr.to_string(),
                SocketAddr::V6(addr) => addr.to_string(),
            },
        }
    }
}

impl FromKatcpArgument for KatcpAddress {
    type Err = KatcpError;

    fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
        if s.as_ref().is_empty() {
            Err(KatcpError::BadArgument)
        } else if let Ok(addr) = s.as_ref().parse() {
            Ok(Self::Socket(addr))
        } else if let Ok(addr) = s.as_ref().parse() {
            Ok(Self::Ip(addr))
        } else if s.as_ref().starts_with('[') && s.as_ref().ends_with(']') {
            let slice = &s.as_ref()[1..s.as_ref().len() - 1];
            if let Ok(addr) = slice.parse() {
                Ok(Self::Ip(addr))
            } else {
                Err(KatcpError::BadArgument)
            }
        } else {
            Err(KatcpError::BadArgument)
        }
    }
}

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

#[derive(KatcpDiscrete, Debug, PartialEq, Eq)]
/// The datatypes that KATCP supports
pub enum ArgumentType {
    /// Represented by i32
    Integer,
    /// Represented by f32
    Float,
    Boolean,
    /// Represented by chrono::DateTime<Utc>
    Timestamp,
    /// Represented by an enum
    Discrete,
    /// Represented by KatcpAddress
    Address,
    /// Will always be escaped and unescaped during serde
    String,
}

#[derive(Debug, PartialEq)]
/// The sum type of a vector of one of the primitive [`ArgumentType`]s
pub enum ArgumentVec {
    Integer(Vec<i32>),
    Float(Vec<f32>),
    Boolean(Vec<bool>),
    Timestamp(Vec<DateTime<Utc>>),
    String(Vec<String>),
    Discrete(Vec<String>),
    Address(Vec<KatcpAddress>),
}

impl ArgumentVec {
    pub fn to_string(&self) -> String {
        match self {
            ArgumentVec::Integer(_) => "integer",
            ArgumentVec::Float(_) => "float",
            ArgumentVec::Boolean(_) => "boolean",
            ArgumentVec::Timestamp(_) => "timestamp",
            ArgumentVec::String(_) => "string",
            ArgumentVec::Discrete(_) => "discrete",
            ArgumentVec::Address(_) => "address",
        }
        .to_owned()
    }
}

impl ToKatcpArguments for ArgumentVec {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            Self::Integer(v) => v.iter().map(|e| e.to_argument()).collect(),
            Self::Float(v) => v.iter().map(|e| e.to_argument()).collect(),
            Self::Boolean(v) => v.iter().map(|e| e.to_argument()).collect(),
            Self::Timestamp(v) => v.iter().map(|e| e.to_argument()).collect(),
            Self::String(v) => v.iter().map(|e| e.to_argument()).collect(),
            Self::Discrete(v) => v.iter().map(|e| e.to_argument()).collect(),
            Self::Address(v) => v.iter().map(|e| e.to_argument()).collect(),
        }
    }
}

pub fn from_argument_vec(
    ty: &ArgumentType,
    strings: &mut impl Iterator<Item = String>,
) -> Result<ArgumentVec, KatcpError> {
    Ok(match ty {
        ArgumentType::Boolean => ArgumentVec::Boolean(
            strings
                .map(bool::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ArgumentType::Integer => ArgumentVec::Integer(
            strings
                .map(i32::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ArgumentType::Float => ArgumentVec::Float(
            strings
                .map(f32::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ArgumentType::Timestamp => ArgumentVec::Timestamp(
            strings
                .map(DateTime::<Utc>::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ArgumentType::Discrete => ArgumentVec::Discrete(
            strings
                .map(String::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ArgumentType::Address => ArgumentVec::Address(
            strings
                .map(KatcpAddress::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ArgumentType::String => ArgumentVec::String(
            strings
                .map(String::from_argument)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    })
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

    #[test]
    fn test_int() {
        let pos_int = 12345;
        let neg_int = -12345;
        assert_eq!(pos_int, u32::from_argument(pos_int.to_argument()).unwrap());
        assert_eq!(neg_int, i32::from_argument(neg_int.to_argument()).unwrap());
    }

    #[test]
    fn test_bool() {
        let a = true;
        let b = false;
        assert_eq!(a, bool::from_argument(a.to_argument()).unwrap());
        assert_eq!(b, bool::from_argument(b.to_argument()).unwrap());
    }

    #[test]
    fn test_float() {
        let a = -1.234e-05;
        let b = 1.7;
        let c = 100.0;
        assert_eq!(a, f32::from_argument(a.to_argument()).unwrap());
        assert_eq!(b, f32::from_argument(b.to_argument()).unwrap());
        assert_eq!(c, f32::from_argument(c.to_argument()).unwrap());
    }

    #[test]
    fn test_addr() {
        let v4_socket = "192.168.1.1:4000";
        let v4_ip = "127.0.0.1";
        let v6_socket = "[2001:db8:85a3::8a2e:370:7334]:4000";
        let v6_ip = "[::1]";
        assert_eq!(
            v4_socket,
            KatcpAddress::from_argument(v4_socket)
                .unwrap()
                .to_argument()
        );
        assert_eq!(
            v6_socket,
            KatcpAddress::from_argument(v6_socket)
                .unwrap()
                .to_argument()
        );
        assert_eq!(
            v4_ip,
            KatcpAddress::from_argument(v4_ip).unwrap().to_argument()
        );
        assert_eq!(
            v6_ip,
            KatcpAddress::from_argument(v6_ip).unwrap().to_argument()
        );
    }
}
