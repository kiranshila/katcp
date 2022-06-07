# katcp

[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![docs](https://img.shields.io/docsrs/katcp?logo=rust&style=flat-square)](https://docs.rs/katcp/latest/katcp/index.html)
[![rustc](https://img.shields.io/badge/rustc-1.59+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![build status](https://img.shields.io/github/workflow/status/GReX-Telescope/katcp/CI/main?style=flat-square&logo=github)](https://github.com/GReX-Telescope/katcp/actions)
[![Codecov](https://img.shields.io/codecov/c/github/GReX-Telescope/katcp?style=flat-square)](https://app.codecov.io/gh/GReX-Telescope/katcp)

This crate provides a rust implementation of the [KATCP](https://katcp-python.readthedocs.io/en/latest/_downloads/361189acb383a294be20d6c10c257cb4/NRF-KAT7-6.0-IFCE-002-Rev5-1.pdf)
monitor and control protocol, as described by the Karoo Array Telescope (KAT) project from the Square Kilometer Array (SKA) South Africa group.

### Description

From the official specification:

> Broadly speaking, KATCP consists of newline-separated text messages sent asynchronously over a TCP/IP
> stream. There are three categories of messages: requests, replies and informs. Request messages expect some
> sort of acknowledgement. Reply messages acknowledge requests. Inform messages require no acknowledgement
> Inform messages are of two types: those sent synchronously as part of a reply and those sent asynchronously.

The details of orchestrating a client or server for this protocol is not the goal of this crate. Rather, this crate
only provides the core `protocol::Message` type and the core message formats. It is up to the user of this crate to implement a client or server. This is to allow this library to be small and portable and not to have to make any assumptions about the eventual implementation.

### License

katcp is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
