# katcp

[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![rustc](https://img.shields.io/badge/rustc-1.54+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![build status](https://img.shields.io/github/workflow/status/GReX-Telescope/katcp/CI/main?style=flat-square&logo=github)](https://github.com/GReX-Telescope/katcp/actions)

This crate provides a rust implementation of the [KATCP](https://katcp-python.readthedocs.io/en/latest/_downloads/361189acb383a294be20d6c10c257cb4/NRF-KAT7-6.0-IFCE-002-Rev5-1.pdf)
monitor and control protocol, as described by the Karoo Array Telescope (KAT) project from the Square Kilometer Array (SKA) South Africa group.

### Description

From the official specification:

> Broadly speaking, KATCP consists of newline-separated text messages sent asynchronously over a TCP/IP
> stream. There are three categories of messages: requests, replies and informs. Request messages expect some
> sort of acknowledgement. Reply messages acknowledge requests. Inform messages require no acknowledgement
> Inform messages are of two types: those sent synchronously as part of a reply and those sent asynchronously.

The details of orchestrating a client or server for this protocol is not the goal of this crate. Rather, this crate
only provides the core [`protocol::Message`] type and the required message formats. It is up to the user of this crate how to design
a client or server. This is to allow this library to be small and portable and not to have to make any assumptions about
the eventual implementation.

### Examples

Serialization and deserialization is handled through the core [`protocol::Message`] type. Most of the standard rust conversion methods should work
and error appropriately.

#### Deserialization

If you have a string that represents a katcp message, you can convert directly into the [`protocol::Message`] struct.

```rust
use katcp::protocol::Message;
use std::str::FromStr;

let msg_str = "?set-unknown-paramer[123] 6.1 true my-attribute";
// Both of these are equivalent
let msg_a: Message = msg_str.try_into().unwrap();
let msg_b = Message::from_str(msg_str).unwrap();
```

If you are working on a stream of messages, you can invoke the parser directly. The parser is written with the [nom](https://github.com/Geal/nom)
parser combinator library, so the top level `protocol::message` can be used with that directly.

```rust
use katcp::protocol::{message, Message};
use nom::{multi::many1, IResult};

fn parse_many_messages(input: &str) -> IResult<&str, Vec<Message>> {
    many1(message)(input)
}
```

#### Serialization

If you have a constructed `protocol::Message`, you can call anything that uses `Display` to serialize.
Note: the serialization function does _not_ check validity, that is performed with the standard `protocol::Message::new`
constructor. The `Display` methods will assume a constructed message is valid. If you want to skip these validation steps
there is the `protocol::Message::new_unchecked`, which is marked `unsafe`.

```rust
use katcp::protocol::{Message,MessageKind};

let msg = Message::new(MessageKind::Inform,"foo-bar",None,vec!["param-1","param-2"]).unwrap(); // Panic on bad arguments
let msg_str = format!("{}",msg);
```
