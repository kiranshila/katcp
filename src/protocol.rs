//! The implementation of the protocol itself (no message specific details)
//!
//! You usually shouldn't have to interact with things from here and use the message types directly. However, you
//! can if you want to
//! ## Examples
//!
//! Serialization and deserialization is handled through the core [`Message`] type. Most of the standard rust conversion methods should work
//! and error appropriately.
//!
//! ### Deserialization
//!
//! If you have a string that represents a katcp message, you can convert directly into the [`Message`] struct.
//!
//! ```
//! use std::str::FromStr;
//!
//! use katcp::protocol::Message;
//!
//! let msg_str = "?set-unknown-paramer[123] 6.1 true my-attribute";
//! // Both of these are equivalent
//! let msg_a: Message = msg_str.try_into().unwrap();
//! let msg_b = Message::from_str(msg_str).unwrap();
//! ```
//!
//! If you are working on a stream of messages, you can invoke the parser directly. The parser is written with the [nom](https://github.com/Geal/nom)
//! parser combinator library, so the top level [`message`] can be used with that directly.
//!
//! ```
//! use katcp::protocol::{message, Message};
//! use nom::{multi::many1, IResult};
//!
//! fn parse_many_messages(input: &str) -> IResult<&str, Vec<Message>> {
//!     many1(message)(input)
//! }
//! ```
//!
//! ### Serialization
//!
//! If you have a constructed [`Message`], you can call anything that uses `Display` to serialize.
//! Note: the serialization function does *not* check validity, that is performed with the standard [`Message::new`]
//! consstructor. The `Display` methods will assume a constructed message is valid. If you want to skip these validation steps
//! there is the [`Message::new_unchecked`], which is marked `unsafe`.
//!
//! ```
//! use katcp::protocol::{Message, MessageKind};
//!
//! let msg = Message::new(MessageKind::Inform, "foo-bar", None, vec![
//!     "param-1", "param-2",
//! ])
//! .unwrap(); // Panic on bad arguments
//! let msg_str = format!("{}", msg);
//! ```

use core::{fmt::Display, str::FromStr};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char, digit0, none_of, one_of},
    combinator::{eof, map_res, opt, recognize},
    error::Error,
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
/// The kind of katcp message. The docs call this the type, but we want to scoot
/// around the fact that `type` is a reserved keyword.
pub enum MessageKind {
    /// Request (?) messages will always be acknowledged by a reply
    Request,
    /// Reply (!) messages are sent in response to a `Request`
    Reply,
    /// Inform (#) messages can be sent asynchronously and do not invoke a reply
    Inform,
}

#[derive(Debug, PartialEq, Eq)]
/// The core raw message type of katcp
pub struct Message {
    /// The message kind
    pub(crate) kind: MessageKind,
    /// The message name
    pub(crate) name: String,
    /// The optional, positive message id
    pub(crate) id: Option<u32>,
    /// The (potentially empty) vector of message arguments
    /// In this structure, these will always be strings. It
    /// is left to consumers to define the serde into the
    /// appropriate types.
    pub(crate) arguments: Vec<String>,
}

#[derive(Debug, PartialEq)]
/// The core Error type for this crate
pub enum KatcpError {
    ParseError(nom::Err<Error<String>>),
    BadArgument,
    MissingArgument,
    IncorrectType,
    Message(String),
    Unknown,
}

pub type MessageResult = Result<Message, KatcpError>;

impl Message {
    /// A constructor for message that will create owned copies of the string-slice arguments
    /// # Safety
    /// This constructor does *not* validate that the `name` and `arguments` are valid and as
    /// such the serialized result may be wrong. It is up to the user to verify that the name
    /// and arguments match the spec
    pub unsafe fn new_unchecked<T: AsRef<str>, U: AsRef<str>>(
        kind: MessageKind,
        name: T,
        id: Option<u32>,
        arguments: Vec<U>,
    ) -> Self {
        Self {
            kind,
            name: name.as_ref().into(),
            id,
            arguments: arguments.iter().map(|s| s.as_ref().into()).collect(),
        }
    }

    /// A constructor for message that will create owned copies of the string-slice arguments
    pub fn new<T: AsRef<str>, U: AsRef<str>>(
        kind: MessageKind,
        name: T,
        id: Option<u32>,
        arguments: Vec<U>,
    ) -> Result<Self, KatcpError> {
        // Check name
        if let Err(e) = crate::protocol::name(name.as_ref()) {
            return Err(KatcpError::ParseError(own_nom_err(e)));
        }
        for argument in arguments.iter() {
            if let Err(e) = crate::protocol::argument(argument.as_ref()) {
                return Err(KatcpError::ParseError(own_nom_err(e)));
            }
        }
        // Safety: this is after we've thrown parser results for validation of name
        // and arguments, so we're good to go here
        unsafe { Ok(Self::new_unchecked(kind, name, id, arguments)) }
    }

    /// Kind getter
    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    /// Name getter
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Id getter
    pub fn id(&self) -> Option<u32> {
        self.id
    }

    /// Arguments getter
    pub fn arguments(&self) -> Vec<String> {
        self.arguments.clone()
    }
}

fn own_nom_err(e: nom::Err<Error<&str>>) -> nom::Err<Error<String>> {
    match e {
        nom::Err::Incomplete(i) => nom::Err::Incomplete(i),
        nom::Err::Error(Error { input, code }) => nom::Err::Error(Error {
            input: input.to_owned(),
            code,
        }),
        nom::Err::Failure(Error { input, code }) => nom::Err::Failure(Error {
            input: input.to_owned(),
            code,
        }),
    }
}

fn kind(input: &str) -> IResult<&str, MessageKind> {
    let (remaining, typ) = one_of("!#?")(input)?;
    Ok((remaining, match typ {
        '?' => MessageKind::Request,
        '!' => MessageKind::Reply,
        '#' => MessageKind::Inform,
        _ => unreachable!(),
    }))
}

fn whitespace(input: &str) -> IResult<&str, &str> {
    recognize(many1(one_of(" \t")))(input)
}

fn name(input: &str) -> IResult<&str, &str> {
    recognize(pair(alpha1, many0(alt((alphanumeric1, tag("-"))))))(input)
}

fn id(input: &str) -> IResult<&str, u32> {
    map_res(
        delimited(
            char('['),
            recognize(tuple((one_of("123456789"), digit0))),
            char(']'),
        ),
        str::parse,
    )(input)
}

fn escape(input: &str) -> IResult<&str, &str> {
    recognize(pair(char('\\'), one_of("\\_0nret@")))(input)
}

fn eol(input: &str) -> IResult<&str, &str> {
    recognize(one_of("\n\r"))(input)
}

fn plain(input: &str) -> IResult<&str, &str> {
    recognize(many1(none_of("\\ \0\n\r\t")))(input)
}

fn argument(input: &str) -> IResult<&str, &str> {
    recognize(many1(alt((escape, plain))))(input)
}

/// The parser combinator for messages. One could write a grammar that utilizes this parser with nom.
pub fn message(input: &str) -> IResult<&str, Message> {
    let (remaining, (kind, name, id, arguments, _, _)) = tuple((
        kind,
        name,
        opt(id),
        many0(preceded(whitespace, argument)),
        opt(whitespace),
        alt((eol, eof)),
    ))(input)?;

    // Safety: this is after we've unwrapped the parser result, so any parser errors will have been
    // thrown already, so we can guarantee that this message will be valid
    Ok((remaining, unsafe {
        Message::new_unchecked(kind, name, id, arguments)
    }))
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_msg_type() {
        assert_eq!(Ok(("", MessageKind::Reply)), kind("!"));
        assert_eq!(Ok(("", MessageKind::Inform)), kind("#"));
        assert_eq!(Ok(("", MessageKind::Request)), kind("?"));
    }

    #[test]
    fn test_name() {
        assert_eq!(Ok(("", "set-rate")), name("set-rate"));
        assert_eq!(Ok(("", "foobar")), name("foobar"));
        assert_eq!(Ok(("", "f00-bar")), name("f00-bar"));
    }

    #[test]
    fn test_msg_id() {
        assert_eq!(Ok(("", 123)), id("[123]"));
        assert_eq!(Ok(("", 100)), id("[100]"));
        assert_eq!(Ok(("", 9)), id("[9]"));
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(Ok(("", " ")), whitespace(" "));
        assert_eq!(Ok(("", "    ")), whitespace("    "));
        assert_eq!(Ok(("", "\t    \t")), whitespace("\t    \t"));
    }

    #[test]
    fn test_escaped() {
        assert_eq!(Ok(("", r"\\")), escape(r"\\"));
        assert_eq!(Ok(("", r"\_")), escape(r"\_"));
        assert_eq!(Ok(("", r"\0")), escape(r"\0"));
        assert_eq!(Ok(("", r"\n")), escape(r"\n"));
        assert_eq!(Ok(("", r"\r")), escape(r"\r"));
        assert_eq!(Ok(("", r"\e")), escape(r"\e"));
        assert_eq!(Ok(("", r"\t")), escape(r"\t"));
        assert_eq!(Ok(("", r"\@")), escape(r"\@"));
    }

    #[test]
    fn test_eol() {
        assert_eq!(Ok(("", "\n")), eol("\n"));
        assert_eq!(Ok(("", "\r")), eol("\r"));
    }

    #[test]
    fn test_plain() {
        assert_eq!(Ok(("", "6.1")), plain("6.1"));
        assert_eq!(Ok(("", "invalid")), plain("invalid"));
        assert_eq!(Ok(("\\_request.", "Unknown")), plain("Unknown\\_request."));
    }

    #[test]
    fn test_argument() {
        assert_eq!(Ok(("", "6.1")), argument("6.1"));
        assert_eq!(Ok(("", "invalid")), argument("invalid"));
        assert_eq!(
            Ok(("", "Unknown\\_request.")),
            argument("Unknown\\_request.")
        );
    }

    #[test]
    fn test_message() {
        assert_eq!(
            Message::new(MessageKind::Request, "set-rate", None, vec!["5.1"]).unwrap(),
            message("?set-rate 5.1").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Request, "set-rate", None, vec!["5.1"]).unwrap(),
            message("?set-rate 5.1\n").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Reply, "set-rate", None, vec!["ok"]).unwrap(),
            message("!set-rate ok").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Request, "set-unknown-parameter", None, vec![
                "6.1"
            ])
            .unwrap(),
            message("?set-unknown-parameter 6.1").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Reply, "set-unknown-parameter", None, vec![
                "invalid",
                r"Unknown\_request."
            ])
            .unwrap(),
            message(r"!set-unknown-parameter invalid Unknown\_request.")
                .unwrap()
                .1
        );
        assert_eq!(
            Message::new(MessageKind::Reply, "set-rate", None, vec![
                "fail",
                r"Hardware\_did\_not\_respond."
            ])
            .unwrap(),
            message(r"!set-rate fail Hardware\_did\_not\_respond.")
                .unwrap()
                .1
        );
        assert_eq!(
            Message::new(MessageKind::Request, "set-rate", Some(123), vec!["4.1"]).unwrap(),
            message("?set-rate[123] 4.1").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Reply, "set-rate", Some(123), vec!["ok"]).unwrap(),
            message("!set-rate[123] ok").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageKind::Request,
                "sensor-list",
                None,
                Vec::<String>::new()
            )
            .unwrap(),
            message("?sensor-list").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageKind::Request,
                "sensor-list",
                Some(420),
                Vec::<String>::new()
            )
            .unwrap(),
            message("?sensor-list[420]").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Inform, "sensor-list", None, vec![
                "drive.enable-azim",
                r"Azimuth\_drive\_enable\_signal\_status",
                r"\@",
                "boolean"
            ])
            .unwrap(),
            message(
                r"#sensor-list drive.enable-azim Azimuth\_drive\_enable\_signal\_status \@ boolean"
            )
            .unwrap()
            .1
        );
        assert_eq!(
            Message::new(
                MessageKind::Inform,
                "sensor-list",
                None,
                vec![
                    "drive.enable-elev",
                    r"Elevation\_drive\_enable\_signal\_status",
                    r"\@",
                    "boolean"

                ]
            ).unwrap(),
            message(
                r"#sensor-list drive.enable-elev Elevation\_drive\_enable\_signal\_status \@ boolean"
            )
            .unwrap()
            .1
        );
        assert_eq!(
            Message::new(MessageKind::Inform, "sensor-list", None, vec![
                "drive.dc-voltage-elev",
                r"Drive\_bus\_voltage",
                "V",
                "float",
                "0.0",
                "900.0"
            ])
            .unwrap(),
            message(r"#sensor-list drive.dc-voltage-elev Drive\_bus\_voltage V float 0.0 900.0")
                .unwrap()
                .1
        );
        assert_eq!(
            Message::new(MessageKind::Reply, "sensor-list", None, vec!["ok", "3"]).unwrap(),
            message(r"!sensor-list ok 3").unwrap().1
        );
        assert_eq!(
            Message::new(MessageKind::Inform, "internet-box", None, vec![
                "address",
                "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:4000"
            ])
            .unwrap(),
            message(r"#internet-box address [2001:0db8:85a3:0000:0000:8a2e:0370:7334]:4000 ")
                .unwrap()
                .1
        );
    }
}

impl FromStr for Message {
    type Err = KatcpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match message(s) {
            Ok((_, m)) => Ok(m),
            Err(e) => Err(KatcpError::ParseError(own_nom_err(e))),
        }
    }
}

impl TryFrom<&str> for Message {
    type Error = KatcpError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

#[cfg(test)]
mod deserialization_tests {
    use super::*;

    #[test]
    fn deserialization() {
        let msg = Message::new(MessageKind::Inform, "foo-bar", Some(123), vec![
            "foo", "bar",
        ])
        .unwrap();
        let msg_str = "#foo-bar[123] foo bar";
        // FromStr
        assert_eq!(msg, Message::from_str(msg_str).unwrap());
        assert_eq!(msg, msg_str.parse().unwrap());
        // TryInto
        assert_eq!(msg, msg_str.try_into().unwrap());
    }
}

// Serialization
impl Display for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let type_char = match self.kind {
            MessageKind::Request => '?',
            MessageKind::Reply => '!',
            MessageKind::Inform => '#',
        };
        let id_str = match self.id {
            Some(id) => format!("[{}]", id),
            None => "".to_owned(),
        };
        let mut args_str = "".to_owned();
        self.arguments.iter().for_each(|argument| {
            args_str.push(' ');
            args_str.push_str(argument);
        });
        writeln!(f, "{}{}{}{}", type_char, self.name, id_str, args_str)
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn serialization() {
        let msg = Message::new(MessageKind::Inform, "foo-bar", Some(123), vec![
            "foo", "bar",
        ])
        .unwrap();
        let msg_str = "#foo-bar[123] foo bar\n";
        assert_eq!(msg_str, msg.to_string());
    }
}

#[cfg(test)]
mod there_and_back_tests {
    use super::*;

    #[test]
    fn struct_and_back() {
        let msg = Message::new(MessageKind::Inform, "foo-bar", Some(123), vec![
            "foo", "bar",
        ])
        .unwrap();
        assert_eq!(Message::from_str(&msg.to_string()).unwrap(), msg);
    }

    #[test]
    fn string_and_back() {
        let msg_str = "#foo-bar[123] foo bar\n";
        assert_eq!(Message::from_str(msg_str).unwrap().to_string(), msg_str);
    }
}
