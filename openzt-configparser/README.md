# bf-configparser
<!-- [![Build Status](https://github.com/QEDK/configparser-rs/actions/workflows/rust.yaml/badge.svg)](https://github.com/QEDK/configparser-rs/actions/workflows/rust.yaml) [![Crates.io](https://img.shields.io/crates/l/configparser?color=black)](LICENSE-MIT) [![Crates.io](https://img.shields.io/crates/v/configparser?color=black)](https://crates.io/crates/configparser) [![Released API docs](https://docs.rs/configparser/badge.svg)](https://docs.rs/configparser) [![Maintenance](https://img.shields.io/maintenance/yes/2024)](https://github.com/QEDK/configparser-rs)-->
[![Build Status](https://github.com/openztcc/bf-configparser/actions/workflows/rust.yaml/badge.svg)](https://github.com/QEDK/configparser-rs/actions/workflows/rust.yaml) [![Crates.io](https://img.shields.io/crates/l/bf-configparser?color=black)](LICENSE-MIT) [![Maintenance](https://img.shields.io/maintenance/yes/2024)](https://github.com/QEDK/configparser-rs)

This is a Rust library for parsing and writing configuration files based on `INI` like file formats used in [Zoo Tycoon (2001)](https://en.wikipedia.org/wiki/Zoo_Tycoon_(2001_video_game)). 

It is forked from [configparser-rs](https://github.com/QEDK/configparser-rs), the main differences are:
 - Duplicate keys create a list of values instead of overwriting the previous value
 - Breaks api compatibility with Python's configparser, `getbool()` and other non-snake case functions are removed
 - Adds `get_vec()` function to get a vector of values
 - Adds generic `get_parse()` and `get_vec_parse` functions, replacing all `get*()` functions