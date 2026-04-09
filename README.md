# winrarkey

A compact Rust CLI for generating WinRAR license artifacts from a username and license mode

## Overview

This project implements the WinRAR registration data flow described in:

- `HOW_DOES_IT_WORK.md`
- `VERIFY_Point_G.md`

The current codebase includes:

- finite field arithmetic over `GF((2^15)^17)`
- elliptic curve point operations
- WinRAR-style private key derivation
- WinRAR-style signature generation
- `rarreg.key` text generation

## Features

- Clean CLI based on `clap`
- Modular Rust structure instead of a single `main.rs`
- Focused Chinese comments on key logic
- Built-in tests for core curve and key derivation checks

## Project Structure

```text
src/
├── main.rs       # CLI entrypoint
├── lib.rs        # top-level workflow
├── cli.rs        # command-line arguments
└── crypto.rs     # field, ECC, signing, register data generation
```

## Requirements

- Rust toolchain

## Build

```powershell
cargo build --release
```

## Usage

Generate `rarreg.key`

```powershell
cargo run -- --user Github
```

Generate `rarreg.key` with explicit license type

```powershell
cargo run -- --user Github --license-name "Single PC usage license"
```

Show CLI help

```powershell
cargo run -- --help
```

## Notes

- Output file name is fixed as `rarreg.key`
- Input text is currently processed as UTF-8 bytes
- If exact ANSI behavior is needed, only the encoding layer needs to be adjusted

## Development

Format the code

```powershell
cargo fmt
```

Run tests

```powershell
cargo test
```

## Reference Documents

- `HOW_DOES_IT_WORK.md`
- `VERIFY_Point_G.md`
- [bitcookies/winrar-keygen](https://github.com/bitcookies/winrar-keygen)
