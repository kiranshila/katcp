use crate::protocol::{Message, MessageKind, MessageResult};

/// The trait that specific katcp messages should implement
pub trait KatcpMessage: TryFrom<Message> {
    fn into_message(self, kind: MessageKind, id: Option<u32>) -> MessageResult;
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
