//! Messages for implementing multi-client support
//!
//! KATCP-compliant devices may either support multiple simultaneous clients or only a single client. The multi-client
//! option is the preferred option for devices capable of implementing it as it provides a means of monitoring
//! a device while it is being controlled on a separate connection.
//!
//! Multi-client devices need not make any arrangements to share control – they may simply accept commands
//! from all clients. Clients should arrange to handle shared control among themselves. It is expected that usually
//! a single client will have primary control and that other clients will only monitor the device, although this
//! arrangement is not required by KATCP.
//! To assist clients in tracking what other clients are connected a [`ClientList`] request is provided so the current
//! list of connected clients can be retrieved. A [`ClientConnected`] inform is sent to each connected client when
//! a new client is accepted.
//!
//! Replies to requests should be sent only to the client that made the request. Inform messages generated as part
//! of a reply should also only be sent to the client that made the request.
//!
//! Whether asynchronous informs are sent to multiple clients is determined by the type of inform. Of the core messages implemented
//! in this crate, only [`crate::messages::log::Log`] and [`ClientConnected`] are sent to multiple clients. All others are sent
//! to the single client associated with the event that triggered the inform. For `build_state`, `version` and
//! [`crate::messages::core::Disconnect`], it is the client connecting or being disconnected. For [`crate::messages::sensors::SensorStatus`] it is the client that configured the
//! sensor sampling strategy. The [`crate::messages::log::Log`] informs should be sent to all clients. The [`ClientConnected`]
//! informs should be sent to all clients except the one that has just connected.
//!
//! Devices should maintain one sensor sampling strategy per sensor per client and send sampled values only to
//! the client that set up the sampling strategy.

use katcp_derive::KatcpMessage;

use crate::prelude::*;

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// Messages for getting information about all the connected clients
pub enum ClientList {
    /// Before sending a reply, the client-list request will send a client-list inform
    /// message containing the address of a client for each client connected to the device,
    /// including the client making the request.
    Request,
    Inform {
        addr: KatcpAddress,
    },
    Reply(IntReply),
}

#[derive(KatcpMessage, Debug, PartialEq, Eq, Clone)]
/// The inform messsage sent on new connections
pub enum ClientConnected {
    /// A description of the new client. It should include the address and port the new client connected from
    Inform { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::common::roundtrip_test;

    #[test]
    fn test_client_list() {
        roundtrip_test(ClientList::Request);
        roundtrip_test(ClientList::Reply(IntReply::Ok { num: 3 }));
        roundtrip_test(ClientList::Inform {
            addr: KatcpAddress::from_argument("192.168.4.10:8081").unwrap(),
        });
        roundtrip_test(ClientList::Inform {
            addr: KatcpAddress::from_argument("[::1]").unwrap(),
        });
    }

    #[test]
    fn test_client_connected() {
        roundtrip_test(ClientConnected::Inform {
            message: "Welcome! You're connected".to_owned(),
        })
    }
}
