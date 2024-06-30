# cube

[![Crates.io Version](https://img.shields.io/crates/v/cube_rs.svg)](https://crates.io/crates/cube_rs)

The universal GameCube file format tool.

Currently work in progress. This project is being developed in conjunction with [P2GZ](https://github.com/p2gz/p2gz) and will primarily support features needed for it at first, but aims to eventually support workflows for other games and hacks as well.

## Installation
Cube can be used either as a Crate or as a CLI tool. Please use the help commands for usage instructions as the tool is not currently stable and available functionality and defaults will change without warning.

### CLI
1. Download and install Rust and Cargo (rustup is recommended)
1. Run `cargo install cubetool`
1. Use as `cube extract file.szs` etc.

### Crate
`cargo add cube_rs`

## Features / Roadmap
- [x] SZS (archives)
- [x] RARC (archives)
- [ ] SARC (archives)
- [ ] BTI (images)
    - [x] Decoding
    - [ ] Encoding
- [x] Yaz0 (compression scheme, via [yaz0](https://crates.io/crates/yaz0)) 
- [ ] BMG (text dictionaries)
- [ ] BLO (menu screens)
- [ ] BMS (music and sounds)
- [ ] CND (Pikmin 2 specific(?) music config)
- [ ] ISO (disc images, via [gc-gcm](https://crates.io/crates/gc-gcm))
    - [x] Decoding
    - [ ] Encoding
