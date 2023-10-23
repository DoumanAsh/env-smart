//! Improved version of `env!` macro from std.
//!
//! ## Syntax:
//!
//! - Standard `env!` - If plain string specified then behavior is the same as standard [env](https://doc.rust-lang.org/std/macro.env.html) macro
//! - Simplified formatting - Allows to format string using multiple variables enveloped into `{}` brackets. Note that bracket escaping is not supported
//!
//!
//!## Sources
//!
//!Macro fetches environment variables in following order:
//!
//!- Use `.env` file from root where build is run. Duplicate values are not allowed.
//!- Use current environment where proc macro runs. It will not override `.env` variables
//!
//! ## Usage
//!
//! ```rust
//! use env_smart::env;
//!
//! static USER_AGENT: &str = env!("{CARGO_PKG_NAME}-{CARGO_PKG_VERSION}");
//!
//! assert_eq!(USER_AGENT, "env-smart-1.0.0-alpha.2");
//!
//! static TEST: &str = env!("test-{CARGO_PKG_NAME}-{CARGO_PKG_VERSION}");
//!
//! assert_eq!(TEST, "test-env-smart-1.0.0-alpha.2");
//!
//! assert_eq!(env!("{CARGO_PKG_NAME}"), "env-smart");
//!
//! assert_eq!(env!("CARGO_PKG_NAME"), "env-smart");
//!
//! #[cfg(not(windows))]
//! assert_ne!(env!("PWD"), "PWD");
//! ```

#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

use proc_macro::{TokenStream, TokenTree};

use core::mem;
use core::cell::UnsafeCell;

use std::fs;
use std::io::{self, BufRead};
use std::collections::{hash_map, HashMap};
use std::sync::Once;

mod format;

const QUOTE: char = '"';

#[cold]
#[inline(never)]
fn compile_error(error: &str) -> TokenStream {
    format!("compile_error!(\"{error}\")").parse().unwrap()
}

fn read_envs() -> Result<HashMap<String, String>, TokenStream> {
    let mut envs = HashMap::default();

    match fs::File::open(".env") {
        Ok(file) => {
            let file = io::BufReader::new(file);
            for line in file.lines() {
                match line {
                    Ok(line) => {
                        let mut split = line.splitn(2, '=');
                        let key = split.next().unwrap();
                        let value = match split.next() {
                            Some(value) => value,
                            None => return Err(compile_error(&format!(".env file has '{key}' without value"))),
                        };

                        if envs.insert(key.to_owned(), value.to_owned()).is_some() {
                            return Err(compile_error(&format!(".env file has multiple instances of '{key}'")))
                        }
                    },
                    Err(error) => {
                        let error = format!(".env: Read fail: {error}");
                        return Err(compile_error(&error));
                    }
                }
            }
        }
        Err(error) => match error.kind() {
            io::ErrorKind::NotFound => (),
            _ => {
                let error = format!(".env: Cannot open: {error}");
                return Err(compile_error(&error));
            },
        }
    };

    for (key, value) in std::env::vars() {
        match envs.entry(key) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(value);
            },
            hash_map::Entry::Occupied(_) => (),
        }
    }

    Ok(envs)
}

//Like imagine using lock for one time initialization
type State = UnsafeCell<mem::MaybeUninit<Result<HashMap<String, String>, TokenStream>>>;
struct Cache(State);
unsafe impl Sync for Cache {}

//This implementation may or may not in future, but at the current moment we can freely rely on
//execution context to be shared between all instances of macro call
fn read_cached_envs() -> &'static Result<HashMap<String, String>, TokenStream> {
    static STATE: Cache = Cache(State::new(mem::MaybeUninit::uninit()));
    static LOCK: Once = Once::new();

    LOCK.call_once(|| {
        unsafe {
            *STATE.0.get() = mem::MaybeUninit::new(read_envs());
        }
    });

    unsafe {
        &*(STATE.0.get() as *const _)
    }
}

struct Args {
    input: String,
}

impl Args {
    pub fn from_tokens(input: TokenStream) -> Result<Self, TokenStream> {
        const EXPECTED_STRING: &str = "Expected string literal";
        let mut args = input.into_iter();

        let input = match args.next() {
            Some(TokenTree::Literal(lit)) => {
                let quoted = lit.to_string();
                let result = quoted.trim_matches(QUOTE);
                if result.len() + 2 != quoted.len() {
                    return Err(compile_error(EXPECTED_STRING));
                }
                result.to_owned()
            },
            Some(unexpected) => return Err(compile_error(&format!("{EXPECTED_STRING}, got {:?}", unexpected))),
            None => return Err(compile_error("Missing input string")),
        };

        Ok(Self {
            input,
        })
    }
}

#[proc_macro]
///Inserts env variable
pub fn env(input: TokenStream) -> TokenStream {
    let args = match Args::from_tokens(input) {
        Ok(args) => args,
        Err(error) => return error,
    };
    let envs = match read_cached_envs() {
        Ok(envs) => envs,
        Err(error) => return error.clone(),
    };

    let mut output = String::new();
    let mut formatter = format::Format::new(args.input.as_str(), &envs);

    let mut plain_len = 0;
    let mut args_len = 0;

    output.push(QUOTE);
    while let Some(part) = formatter.next() {
        match part {
            Ok(part) => match part {
                format::Part::Plain(plain) => {
                    plain_len += 1;
                    output.push_str(plain);
                }
                format::Part::Argument(plain) => {
                    args_len += 1;
                    output.push_str(plain);
                }
            },
            Err(error) => {
                return compile_error(&format!("Format string error {error}"));
            }
        }
    }

    if args_len == 0 {
        debug_assert_eq!(plain_len, 1);
        match std::env::var(&output[1..]) {
            Ok(value) => {
                output.clear();
                output.push(QUOTE);
                output.push_str(&value);
            },
            Err(_) => return compile_error(&format!("env:{}: Cannot fetch env value", &output[1..])),
        }
    }

    output.push(QUOTE);

    output.parse().expect("valid literal string syntax")
}
