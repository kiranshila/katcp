use core::{borrow::Borrow, fmt::Display, str::FromStr};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char, digit0, none_of, one_of},
    combinator::{eof, map_res, opt, recognize},
    error::Error,
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, tuple},
    Finish, IResult,
};

#[derive(Debug, PartialEq, Eq)]
/// The type of a katcp message
pub enum MessageType {
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
    /// The message type
    pub msg_type: MessageType,
    /// The message name
    pub name: String,
    /// The optional, positive message id
    pub msg_id: Option<u32>,
    /// The (potentially empty) vector of message arguments
    /// In this structure, these will always be strings. It
    /// is left to consumers to define the serde into the
    /// appropriate types.
    pub arguments: Vec<String>,
}

impl Message {
    /// A constructor for message that will create owned copies of the string-slice arguments
    /// # Safety
    /// This constructor does *not* validate that the `name` and `arguments` are valid and as
    /// such the serialized result may be wrong. It is up to the user to verify that the name
    /// and arguments match the spec
    pub unsafe fn new_unchecked<T: AsRef<str>, U: AsRef<str>>(
        msg_type: MessageType,
        name: T,
        msg_id: Option<u32>,
        arguments: &[U],
    ) -> Self {
        Self {
            msg_type,
            name: name.as_ref().into(),
            msg_id,
            arguments: arguments.iter().map(|s| s.as_ref().into()).collect(),
        }
    }

    /// A constructor for message that will create owned copies of the string-slice arguments
    pub fn new<T: AsRef<str>, U: AsRef<str>>(
        msg_type: MessageType,
        name: T,
        msg_id: Option<u32>,
        arguments: &[U],
    ) -> Result<Self, nom::Err<Error<String>>> {
        // Check name
        if let Err(e) = crate::protocol::name(name.as_ref()) {
            return Err(own_nom_err(e));
        }
        for argument in arguments {
            if let Err(e) = crate::protocol::argument(argument.as_ref()) {
                return Err(own_nom_err(e));
            }
        }
        // Safety: this is after we've thrown parser results for validation of name
        // and arguments, so we're good to go here
        unsafe { Ok(Self::new_unchecked(msg_type, name, msg_id, arguments)) }
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

fn msg_type(input: &str) -> IResult<&str, MessageType> {
    let (remaining, typ) = one_of("!#?")(input)?;
    Ok((
        remaining,
        match typ {
            '?' => MessageType::Request,
            '!' => MessageType::Reply,
            '#' => MessageType::Inform,
            _ => unreachable!(),
        },
    ))
}

fn whitespace(input: &str) -> IResult<&str, &str> {
    recognize(many1(one_of(" \t")))(input)
}

fn name(input: &str) -> IResult<&str, &str> {
    recognize(pair(alpha1, many0(alt((alphanumeric1, tag("-"))))))(input)
}

fn msg_id(input: &str) -> IResult<&str, u32> {
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

pub fn message(input: &str) -> IResult<&str, Message> {
    let (remaining, (msg_type, name, msg_id, arguments, _, _)) = tuple((
        msg_type,
        name,
        opt(msg_id),
        many0(preceded(whitespace, argument)),
        opt(whitespace),
        alt((eol, eof)),
    ))(input)?;

    // Safety: this is after we've unwrapped the parser result, so any parser errors will have been
    // thrown already, so we can guarantee that this message will be valid
    Ok((remaining, unsafe {
        Message::new_unchecked(msg_type, name, msg_id, &arguments)
    }))
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_msg_type() {
        assert_eq!(Ok(("", MessageType::Reply)), msg_type("!"));
        assert_eq!(Ok(("", MessageType::Inform)), msg_type("#"));
        assert_eq!(Ok(("", MessageType::Request)), msg_type("?"));
    }

    #[test]
    fn test_name() {
        assert_eq!(Ok(("", "set-rate")), name("set-rate"));
        assert_eq!(Ok(("", "foobar")), name("foobar"));
        assert_eq!(Ok(("", "f00-bar")), name("f00-bar"));
    }

    #[test]
    fn test_msg_id() {
        assert_eq!(Ok(("", 123)), msg_id("[123]"));
        assert_eq!(Ok(("", 100)), msg_id("[100]"));
        assert_eq!(Ok(("", 9)), msg_id("[9]"));
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
            Message::new(MessageType::Request, "set-rate", None, &["5.1"]).unwrap(),
            message("?set-rate 5.1").unwrap().1
        );
        assert_eq!(
            Message::new(MessageType::Request, "set-rate", None, &["5.1"]).unwrap(),
            message("?set-rate 5.1\n").unwrap().1
        );
        assert_eq!(
            Message::new(MessageType::Reply, "set-rate", None, &["ok"]).unwrap(),
            message("!set-rate ok").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageType::Request,
                "set-unknown-parameter",
                None,
                &["6.1"]
            )
            .unwrap(),
            message("?set-unknown-parameter 6.1").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageType::Reply,
                "set-unknown-parameter",
                None,
                &["invalid", r"Unknown\_request."]
            )
            .unwrap(),
            message(r"!set-unknown-parameter invalid Unknown\_request.")
                .unwrap()
                .1
        );
        assert_eq!(
            Message::new(
                MessageType::Reply,
                "set-rate",
                None,
                &["fail", r"Hardware\_did\_not\_respond."]
            )
            .unwrap(),
            message(r"!set-rate fail Hardware\_did\_not\_respond.")
                .unwrap()
                .1
        );
        assert_eq!(
            Message::new(MessageType::Request, "set-rate", Some(123), &["4.1"]).unwrap(),
            message("?set-rate[123] 4.1").unwrap().1
        );
        assert_eq!(
            Message::new(MessageType::Reply, "set-rate", Some(123), &["ok"]).unwrap(),
            message("!set-rate[123] ok").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageType::Request,
                "sensor-list",
                None,
                &Vec::<String>::new()
            )
            .unwrap(),
            message("?sensor-list").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageType::Request,
                "sensor-list",
                Some(420),
                &Vec::<String>::new()
            )
            .unwrap(),
            message("?sensor-list[420]").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageType::Inform,
                "sensor-list",
                None,
                &[
                    "drive.enable-azim",
                    r"Azimuth\_drive\_enable\_signal\_status",
                    r"\@",
                    "boolean"
                ]
            )
            .unwrap(),
            message(
                r"#sensor-list drive.enable-azim Azimuth\_drive\_enable\_signal\_status \@ boolean"
            )
            .unwrap()
            .1
        );
        assert_eq!(
            Message::new(
                MessageType::Inform,
                "sensor-list",
                None,
                &[
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
            Message::new(
                MessageType::Inform,
                "sensor-list",
                None,
                &[
                    "drive.dc-voltage-elev",
                    r"Drive\_bus\_voltage",
                    "V",
                    "float",
                    "0.0",
                    "900.0"
                ]
            )
            .unwrap(),
            message(r"#sensor-list drive.dc-voltage-elev Drive\_bus\_voltage V float 0.0 900.0")
                .unwrap()
                .1
        );
        assert_eq!(
            Message::new(MessageType::Reply, "sensor-list", None, &["ok", "3"]).unwrap(),
            message(r"!sensor-list ok 3").unwrap().1
        );
        assert_eq!(
            Message::new(
                MessageType::Inform,
                "internet-box",
                None,
                &["address", "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:4000"]
            )
            .unwrap(),
            message(r"#internet-box address [2001:0db8:85a3:0000:0000:8a2e:0370:7334]:4000 ")
                .unwrap()
                .1
        );
    }
}

impl FromStr for Message {
    type Err = Error<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match message(s).finish() {
            Ok((_, message)) => Ok(message),
            Err(Error { input, code }) => Err(Error {
                input: input.to_owned(),
                code,
            }),
        }
    }
}

// impl<T> TryFrom<T> for Message
// where
//     T: Borrow<str>,
// {
//     type Error = Error<String>;

//     fn try_from(value: T) -> Result<Self, Self::Error> {
//         value.borrow().from_str()
//     }
// }

#[cfg(test)]
mod deserialization_tests {
    use super::*;

    #[test]
    fn deserialization() {
        let msg = Message::new(MessageType::Inform, "foo-bar", Some(123), &["foo", "bar"]).unwrap();
        let msg_str = "#foo-bar[123] foo bar";
        assert_eq!(msg, Message::from_str(msg_str).unwrap());
        assert_eq!(msg, msg_str.parse().unwrap());
    }
}

// Serialization
impl Display for Message {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let type_char = match self.msg_type {
            MessageType::Request => '?',
            MessageType::Reply => '!',
            MessageType::Inform => '#',
        };
        let id_str = match self.msg_id {
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
        let msg = Message::new(MessageType::Inform, "foo-bar", Some(123), &["foo", "bar"]).unwrap();
        let msg_str = "#foo-bar[123] foo bar\n";
        assert_eq!(msg_str, msg.to_string());
    }
}

#[cfg(test)]
mod there_and_back_tests {
    use super::*;

    #[test]
    fn struct_and_back() {
        let msg = Message::new(MessageType::Inform, "foo-bar", Some(123), &["foo", "bar"]).unwrap();
        assert_eq!(Message::from_str(&msg.to_string()).unwrap(), msg);
    }

    #[test]
    fn string_and_back() {
        let msg_str = "#foo-bar[123] foo bar\n";
        assert_eq!(Message::from_str(msg_str).unwrap().to_string(), msg_str);
    }
}
