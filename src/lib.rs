// https://github.com/lemire/validateutf8-experiments

mod core;
mod error;

pub use self::core::{validate_next, AsByte};
pub use self::error::UtfError;
