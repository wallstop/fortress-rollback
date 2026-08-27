//! Fallible JSON serialization shared by public telemetry helpers.

use serde::Serialize;
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::string::FromUtf8Error;

/// An error returned by a fallible JSON convenience method.
///
/// The variants retain their original error values so callers can inspect the
/// complete source chain through [`Error::source`].
///
/// This enum is exhaustive: each variant identifies a distinct stage of the
/// two-pass output pipeline.
#[derive(Debug)]
pub enum JsonSerializationError {
    /// Serde rejected the value or the JSON writer failed.
    Serialization {
        /// The original `serde_json` error.
        source: serde_json::Error,
    },
    /// The exact JSON output buffer could not be reserved.
    Allocation {
        /// The number of output bytes requested after the counting pass.
        requested_bytes: usize,
        /// The original allocation error.
        source: TryReserveError,
    },
    /// The JSON serializer unexpectedly produced bytes that were not UTF-8.
    ///
    /// JSON emitted by `serde_json` is UTF-8, so this variant indicates an
    /// internal contract violation rather than invalid caller data.
    InvalidUtf8 {
        /// The original UTF-8 conversion error.
        source: FromUtf8Error,
    },
}

impl fmt::Display for JsonSerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization { source } => {
                write!(f, "JSON serialization failed: {source}")
            },
            Self::Allocation {
                requested_bytes,
                source,
            } => write!(
                f,
                "failed to reserve {requested_bytes} bytes for JSON output: {source}"
            ),
            Self::InvalidUtf8 { source } => {
                write!(f, "JSON serializer produced invalid UTF-8: {source}")
            },
        }
    }
}

impl Error for JsonSerializationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialization { source } => Some(source),
            Self::Allocation { source, .. } => Some(source),
            Self::InvalidUtf8 { source } => Some(source),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum JsonStyle {
    Compact,
    Pretty,
}

#[derive(Default)]
struct CountingWriter {
    bytes: usize,
}

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(buf.len())
            .ok_or_else(|| io::Error::from(io::ErrorKind::FileTooLarge))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ReservedVecWriter<'a> {
    output: &'a mut Vec<u8>,
}

impl Write for ReservedVecWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let remaining = self.output.capacity().saturating_sub(self.output.len());
        if buf.len() > remaining {
            return Err(io::Error::from(io::ErrorKind::WriteZero));
        }
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn serialize_json<T: Serialize + ?Sized>(
    value: &T,
    style: JsonStyle,
) -> Result<String, JsonSerializationError> {
    serialize_json_with_reserve(value, style, |output, bytes| {
        output.try_reserve_exact(bytes)
    })
}

fn serialize_json_with_reserve<T, R>(
    value: &T,
    style: JsonStyle,
    reserve: R,
) -> Result<String, JsonSerializationError>
where
    T: Serialize + ?Sized,
    R: FnOnce(&mut Vec<u8>, usize) -> Result<(), TryReserveError>,
{
    let mut counter = CountingWriter::default();
    match style {
        JsonStyle::Compact => serde_json::to_writer(&mut counter, value),
        JsonStyle::Pretty => serde_json::to_writer_pretty(&mut counter, value),
    }
    .map_err(|source| JsonSerializationError::Serialization { source })?;

    let mut output = Vec::new();
    reserve(&mut output, counter.bytes).map_err(|source| JsonSerializationError::Allocation {
        requested_bytes: counter.bytes,
        source,
    })?;

    let mut writer = ReservedVecWriter {
        output: &mut output,
    };
    match style {
        JsonStyle::Compact => serde_json::to_writer(&mut writer, value),
        JsonStyle::Pretty => serde_json::to_writer_pretty(&mut writer, value),
    }
    .map_err(|source| JsonSerializationError::Serialization { source })?;

    String::from_utf8(output).map_err(|source| JsonSerializationError::InvalidUtf8 { source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::Error as _;

    struct RejectSerialization;

    impl Serialize for RejectSerialization {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(S::Error::custom("injected JSON failure"))
        }
    }

    #[test]
    fn compact_and_pretty_output_match_serde_json() {
        let value = ["quoted: \"", "newline:\n"];
        assert_eq!(
            serialize_json(&value, JsonStyle::Compact).unwrap(),
            serde_json::to_string(&value).unwrap()
        );
        assert_eq!(
            serialize_json(&value, JsonStyle::Pretty).unwrap(),
            serde_json::to_string_pretty(&value).unwrap()
        );
    }

    #[test]
    fn serializer_failure_retains_source() {
        let error = serialize_json(&RejectSerialization, JsonStyle::Compact)
            .expect_err("injected serializer failure must be observable");
        assert!(matches!(
            error,
            JsonSerializationError::Serialization { .. }
        ));
        assert!(error.source().is_some());
        assert!(error.to_string().contains("injected JSON failure"));
    }

    #[test]
    fn allocation_failure_is_not_valid_empty_output() {
        let error = serialize_json_with_reserve(&[1_u8, 2, 3], JsonStyle::Compact, |_, _| {
            Vec::<u8>::new().try_reserve(usize::MAX)
        })
        .expect_err("injected allocation refusal must be observable");
        assert!(matches!(
            error,
            JsonSerializationError::Allocation {
                requested_bytes: 7,
                ..
            }
        ));
        assert!(error.source().is_some());
    }
}
