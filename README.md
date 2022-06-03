# katcp

This crate provides a pure-rust implementation of the
[katcp](https://pythonhosted.org/katcp/) protocol, as developed for the [Karoo
Array Telescope](http://ska.ac.za/) project. This will provide the parser and
structures for the protocol following the most recent
[v5.1](https://katcp-python.readthedocs.io/en/latest/_downloads/361189acb383a294be20d6c10c257cb4/NRF-KAT7-6.0-IFCE-002-Rev5-1.pdf) revision.

Our version number here will reflect the katcp revision in major and minor, with
the patch field left for this library's patches.

## Todo!

* no_std
* KatcpDiscrete derive macro