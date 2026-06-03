//! Shared, fail-closed buffer allocation for the built-in UDP socket adapters.
//!
//! The socket adapters ([`udp_socket`](crate::network::udp_socket) and
//! [`tokio_socket`](crate::network::tokio_socket)) each keep a reused receive
//! and send buffer. Those buffers must satisfy two invariants:
//!
//! 1. **Non-empty.** A zero-length receive buffer silently drops every datagram
//!    (`recv_from` reads zero bytes), and a zero-length send buffer makes every
//!    encode fail as `BufferTooSmall`. A socket built from a `0` size is
//!    therefore permanently non-functional, so a `0` request is rejected up
//!    front with [`ErrorKind::InvalidInput`].
//! 2. **Fallibly allocated.** The size is caller-supplied and unbounded, and the
//!    global allocator *aborts* the process on failure. The reservation uses
//!    [`Vec::try_reserve_exact`] so an over-large request returns a recoverable
//!    error instead of taking the process down (cf. RUSTSEC-2022-0035).
//!
//! Concentrating both invariants in [`zeroed_buffer`] means every socket
//! constructor — `with_buffer_sizes`, `bind_to_port_with_buffer_sizes`,
//! `from_socket_with_buffer_sizes` — inherits them without re-implementing the
//! checks (and so cannot drift out of agreement).

use std::io::{Error, ErrorKind};

/// Allocates a zero-initialized buffer of exactly `size` bytes, rejecting a
/// zero size and reporting allocation failure as a recoverable error.
///
/// `name` identifies the buffer in the returned error message (e.g.
/// `"udp recv buffer"`).
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] if `size` is `0` (a zero-length socket
/// buffer can never send or receive), or [`ErrorKind::OutOfMemory`] if the
/// allocation cannot be reserved.
pub(crate) fn zeroed_buffer(size: usize, name: &'static str) -> Result<Vec<u8>, Error> {
    if size == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "{name} size must be non-zero (a zero-length socket buffer cannot send or receive)"
            ),
        ));
    }
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(size).map_err(|_err| {
        Error::new(
            ErrorKind::OutOfMemory,
            format!("failed to reserve {name} of {size} bytes"),
        )
    })?;
    // `resize` only initializes the capacity reserved fallibly (and non-zero) above.
    // alloc-bound: exact `size` was reserved via the `try_reserve_exact` above.
    buffer.resize(size, 0);
    Ok(buffer)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn zeroed_buffer_zero_size_is_rejected_as_invalid_input() {
        let err = zeroed_buffer(0, "test buffer").expect_err("zero size must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn zeroed_buffer_nonzero_size_allocates_zeroed() {
        let buffer = zeroed_buffer(8, "test buffer").expect("nonzero size must succeed");
        assert_eq!(buffer.len(), 8);
        assert!(buffer.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn zeroed_buffer_one_byte_is_allowed() {
        // The smallest non-empty buffer is valid: the invariant is non-empty,
        // not "large enough", so the minimum boundary is exercised explicitly.
        let buffer = zeroed_buffer(1, "test buffer").expect("one byte must succeed");
        assert_eq!(buffer.len(), 1);
    }
}
