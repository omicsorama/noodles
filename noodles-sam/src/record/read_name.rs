//! SAM record read name.

use std::{
    convert::AsRef,
    error, fmt,
    str::{self, FromStr},
};

// § 1.4 The alignment section: mandatory fields (2020-07-19)
const MAX_LENGTH: usize = 254;

const MISSING: &[u8] = b"*";

/// A SAM record read name.
///
/// This is also called a query name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadName(Vec<u8>);

impl AsRef<str> for ReadName {
    fn as_ref(&self) -> &str {
        // SAFETY: The internal buffer is limited to ASCII graphic characters (i.e., '!'-'~').
        str::from_utf8(&self.0).unwrap()
    }
}

impl fmt::Display for ReadName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

/// An error returned when a raw SAM record read name fails to parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The input is empty.
    Empty,
    /// The input is invalid.
    Invalid,
}

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty input"),
            Self::Invalid => f.write_str("invalid input"),
        }
    }
}

impl FromStr for ReadName {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(ParseError::Empty)
        } else if !is_valid_name(s.as_bytes()) {
            Err(ParseError::Invalid)
        } else {
            Ok(Self(s.into()))
        }
    }
}

fn is_valid_name_char(b: u8) -> bool {
    b.is_ascii_graphic() && b != b'@'
}

// § 1.4 "The alignment section: mandatory fields" (2021-06-03): "`[!-?A-~]{1,254}`".
fn is_valid_name(buf: &[u8]) -> bool {
    buf != MISSING && buf.len() <= MAX_LENGTH && buf.iter().copied().all(is_valid_name_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt() -> Result<(), ParseError> {
        let read_name: ReadName = "r0".parse()?;
        assert_eq!(read_name.to_string(), "r0");
        Ok(())
    }

    #[test]
    fn test_from_str() {
        assert_eq!("r0".parse(), Ok(ReadName(b"r0".to_vec())));

        assert_eq!("".parse::<ReadName>(), Err(ParseError::Empty));
        assert_eq!("*".parse::<ReadName>(), Err(ParseError::Invalid));
        assert_eq!("r 0".parse::<ReadName>(), Err(ParseError::Invalid));
        assert_eq!("@r0".parse::<ReadName>(), Err(ParseError::Invalid));

        let s = "n".repeat(MAX_LENGTH + 1);
        assert_eq!(s.parse::<ReadName>(), Err(ParseError::Invalid));
    }
}
