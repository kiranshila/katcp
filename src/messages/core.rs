//! The requests and informs defined in this section deal with connecting to a device, halting it or restarting it
//! and querying it for some basic information about itself. KATCP devices are required to implement all of the
//! messages in this section.

use katcp_derive::KatcpMessage;

use crate::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone)]
/// Requesting a Halt should trigger a software halt
/// It is expected to close the connection and put the
/// software and hardware into a state where it is safe to power down. The reply message should be sent just
/// before the halt occurs
pub enum Halt {
    Request,
    Reply { ret_code: RetCode, message: Option<String> },
}

// We have to do a manual implementation here because katcp chose to make their argument grammar not context free
impl TryFrom<Message> for Halt {
    type Error = KatcpError;
    fn try_from(message: Message) -> Result<Self, Self::Error> {
        if message.name != "halt" {
            return Err(KatcpError::IncorrectType);
        }
        match message.kind {
            MessageKind::Request => Ok(Halt::Request),
            MessageKind::Reply => {
                let ret_code = RetCode::from_argument(
                    message.arguments.get(0).ok_or(KatcpError::MissingArgument)?,
                )?;
                let message = if !matches!(ret_code, RetCode::Ok) {
                    Some(String::from_argument(
                        message.arguments.get(1).ok_or(KatcpError::MissingArgument)?,
                    )?)
                } else {
                    None
                };
                Ok(Halt::Reply { ret_code, message })
            }
            MessageKind::Inform => unimplemented!(),
        }
    }
}

impl KatcpMessage for Halt {
    fn into_message(self, id: Option<u32>) -> MessageResult {
        // Safety: No args, no safety concerns
        match self {
            Halt::Request => Ok(unsafe {
                Message::new_unchecked(MessageKind::Request, "halt", id, Vec::<String>::new())
            }),
            Halt::Reply { ret_code, message } => Ok(if matches!(ret_code, RetCode::Ok) {
                unsafe {
                    Message::new_unchecked(MessageKind::Reply, "halt", id, vec![
                        ret_code.to_argument()
                    ])
                }
            } else {
                // Safety: message.to_argument() escapes, so we're good there
                unsafe {
                    Message::new_unchecked(MessageKind::Reply, "halt", id, vec![
                        ret_code.to_argument(),
                        message.to_argument(),
                    ])
                }
            }),
        }
    }
}

impl TryFrom<&str> for Halt {
    type Error = KatcpError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let message: Message = s.try_into()?;
        message.try_into()
    }
}

#[derive(KatcpMessage, Debug, PartialEq, Eq)]
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
    Reply {
        ret_code: RetCode,
        message: Option<String>,
    },
}

#[derive(KatcpMessage, Debug, PartialEq, Eq)]
/// Requesting a restart should trigger a software reset. It is expected to close the connection, reload the
/// software and begin execution again, preferably without changing the hardware configuration (if possible).
/// It would end with the device being ready to accept new connections again. The reply should be sent before
/// the connection to the current client is closed.
pub enum Restart {
    Inform {},
    Request {},
    Reply { ret_code: RetCode },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_halt() {
        let halt_request = Halt::Request;
        let halt_invalid =
            Halt::Reply { ret_code: RetCode::Invalid, message: Some("You messed up".to_owned()) };
        let halt_ok = Halt::Reply { ret_code: RetCode::Ok, message: None };

        assert_eq!(halt_request, "?halt".try_into().unwrap());
        assert_eq!("?halt\n", halt_request.into_message(None).unwrap().to_string());

        assert_eq!(halt_ok, "!halt ok".try_into().unwrap());
        assert_eq!("!halt ok\n", halt_ok.into_message(None).unwrap().to_string());

        assert_eq!(halt_invalid, r"!halt invalid You\_messed\_up".try_into().unwrap());
        assert_eq!(
            "!halt invalid You\\_messed\\_up\n",
            halt_invalid.into_message(None).unwrap().to_string()
        );
    }
}
