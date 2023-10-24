# env-smart

[![Rust](https://github.com/DoumanAsh/env-smart/actions/workflows/rust.yml/badge.svg)](https://github.com/DoumanAsh/env-smart/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/env-smart.svg)](https://crates.io/crates/env-smart)
[![Documentation](https://docs.rs/env-smart/badge.svg)](https://docs.rs/crate/env-smart/)

Improved version of `env!` macro from std.

## Syntax:

- Standard `env!` - If plain string specified then behavior is the same as standard [env](https://doc.rust-lang.org/std/macro.env.html) macro
- Simplified formatting - Allows to format string using multiple variables enveloped into `{}` brackets. Note that bracket escaping is not supported

## Sources

Macro fetches environment variables in following order:

- Use `.env` file from root where build is run. Duplicate values are not allowed.
- Use current environment where proc macro runs. It will not override `.env` variables

## Usage

```rust
use env_smart::env;

static USER_AGENT: &str = env!("{CARGO_PKG_NAME}-{CARGO_PKG_VERSION}");

assert_eq!(USER_AGENT, "env-smart-1.0.0-alpha.2");
```
