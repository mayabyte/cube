# cube
The universal GameCube file format tool.

Currently work in progress. This project is being developed in conjunction with [P2GZ](https://github.com/p2gz/p2gz) and will primarily support features needed for it at first, but aims to eventually support workflows for other games and hacks as well.

## Installation
Cube can be used either as a Crate or as a CLI tool. Please use the help commands for usage instructions as the tool is not currently stable and available functionality and defaults will change without warning.

### CLI
1. Download and install Rust and Cargo (rustup is recommended)
1. Run `cargo install cubetool`
1. Use as `cube extract file.szs` etc.

### Crate
Add `cube_rs = "0.1.1" to your Cargo.toml.

## Features / Roadmap
- [ ] Extraction
    - [x] SZS (archives)
    - [x] RARC (archives)
    - [ ] SARC (archives)
    - [x] BTI (images)
    - [x] Yaz0 (via `yaz0` crate) 
    - [ ] BMG (text dictionaries)
    - [ ] BLO (menu screens)
    - [ ] BMS (music and sounds)
    - [ ] CND (Pikmin 2 specific(?) music config)
    - [ ] ISO (disc image)
- [ ] Packing / Encoding
    - [x] SZS (archives)
    - [x] RARC (archives)
    - [ ] SARC (archives)
    - [ ] BTI (images)
    - [x] Yaz0 (compression scheme)
    - [ ] BMG (text dictionaries)
    - [ ] BLO (menu screens)
    - [ ] BMS (music and sounds)
    - [ ] CND (Pikmin 2 specific(?) music config)
    - [ ] ISO (disc image)
