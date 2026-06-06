//! `ProofVec`: a growable vector that is a real [`Vec`] in production but a
//! stack-allocated, fixed-capacity inline buffer under Kani.
//!
//! # Why this exists
//!
//! Kani proofs that build the rollback data structures ([`InputQueue`],
//! [`SavedStates`], [`SyncLayer`]) were observed to drive CBMC to ~20 GB of SAT
//! memory and OOM the CI runner. Controlled measurement isolated the cause to
//! CBMC modeling Rust's heap-`Vec` machinery: every `push` / `clone` / drop
//! reaches `RawVec` → `Layout::array::<T>(count)` (a SAT-hard 64-bit
//! `size_of::<T>() * count` multiply) plus the capacity-overflow, `grow_one`,
//! and `deallocate` paths. That cost is *per Vec operation*, independent of the
//! element count — so shrinking queue lengths or unwind bounds does not help.
//!
//! Replacing the backing store with a stack `[Option<T>; CAP]` removes the heap
//! entirely: CBMC models a single fixed-size object with no allocator circuit,
//! collapsing the propositional-reduction phase. It is 100% safe (no `unsafe`),
//! so it upholds the crate's `#![forbid(unsafe_code)]` guarantee.
//!
//! `cfg(kani)` is inactive in every normal build (`cargo build`, `cargo test`,
//! `cargo clippy`, loom, release), so production keeps the real `Vec` with
//! identical behavior and zero overhead — `ProofVec<T>` is literally `Vec<T>`.
//!
//! [`InputQueue`]: crate::input_queue::InputQueue
//! [`SavedStates`]: crate::sync_layer::SavedStates
//! [`SyncLayer`]: crate::sync_layer::SyncLayer

/// A growable vector. In production this is exactly [`Vec<T>`]; under Kani it is
/// the fixed-capacity `InlineVec`. The two expose the same subset of the `Vec`
/// API that the rollback containers use (`push`, `get`, `get_mut`, `len`,
/// `first`, `clone`, `iter`, `iter_mut`), so call sites are identical.
#[cfg(not(kani))]
pub(crate) type ProofVec<T> = Vec<T>;

#[cfg(kani)]
pub(crate) type ProofVec<T> = InlineVec<T, KANI_INLINE_CAP>;

/// Compile-time capacity for every Kani inline vector.
///
/// It must be at least the largest element count any Kani harness builds:
/// - input-queue length (`<= INPUT_QUEUE_LENGTH`, 7 under Kani),
/// - players per `SyncLayer` (`<= 2` in the proofs),
/// - saved-state cells (`<= max_prediction + 1`, `<= 4` in the proofs).
///
/// `INPUT_QUEUE_LENGTH` dominates, so we reuse it: the capacity then tracks the
/// queue-length constant automatically (kept at 7, not 8, so the `CAP`-element
/// array drop loop — see `INPUT_QUEUE_LENGTH`'s docs — fits `--default-unwind 8`
/// for proofs without an explicit unwind). A `push` past `CAP` trips the
/// `kani::assert` in [`InlineVec::push`] (a proof-setup bug, surfaced loudly).
#[cfg(kani)]
pub(crate) const KANI_INLINE_CAP: usize = crate::input_queue::INPUT_QUEUE_LENGTH;

/// Fixed-capacity, stack-allocated vector used **only** under `#[cfg(kani)]` as
/// the backing store for [`ProofVec`]. See the module docs for the rationale.
///
/// Elements live in `[Option<T>; CAP]`; `slots[0..len]` are always `Some` and
/// `slots[len..]` are always `None`. No `unsafe`, no heap allocation.
///
/// `pub` (not `pub(crate)`) only because it backs the `pub` `SavedStates::states`
/// field; this type is `#[cfg(kani)]`-gated, so it never appears in any real
/// (non-Kani) build's public API.
#[cfg(kani)]
#[derive(Debug)]
pub struct InlineVec<T, const CAP: usize> {
    slots: [Option<T>; CAP],
    len: usize,
}

// `InlineVec` is `Copy` exactly when its elements are. Every `Config::Input` is
// `Copy` (a trait requirement), so the only `InlineVec` that is ever cloned —
// `InputQueue::inputs`, whose elements are `PlayerInput<Config::Input>` — always
// qualifies. A `Copy`-based clone is a single memcpy with NO initializer loop,
// whereas `#[derive(Clone)]`'s element-wise `[Option<T>; CAP]` clone runs a
// CAP-iteration loop whose unwinding assertion fails under CI's
// `--default-unwind 8` once `CAP >= 8`. (`SavedStates`/`SyncLayer` are not
// `Clone`, so their `InlineVec` fields never need a `Clone` impl.)
#[cfg(kani)]
impl<T: Copy, const CAP: usize> Copy for InlineVec<T, CAP> {}

#[cfg(kani)]
impl<T: Copy, const CAP: usize> Clone for InlineVec<T, CAP> {
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(kani)]
impl<T, const CAP: usize> Default for InlineVec<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(kani)]
impl<T, const CAP: usize> InlineVec<T, CAP> {
    /// Creates an empty inline vector.
    pub(crate) fn new() -> Self {
        Self {
            // `[const { None }; CAP]` is a compile-time-constant array: it has no
            // initializer loop for CBMC to unwind. `core::array::from_fn` would
            // run a `CAP`-iteration loop whose unwinding assertion fails under
            // CI's `--default-unwind 8` once `CAP >= 8`. The const-repeat form
            // also needs no `T: Copy`/`Default` bound.
            slots: [const { None }; CAP],
            len: 0,
        }
    }

    /// Appends an element. Mirrors [`Vec::push`].
    pub(crate) fn push(&mut self, value: T) {
        // Every Kani harness is sized so `len < CAP` (see `KANI_INLINE_CAP`);
        // an overflow is a proof-setup bug, so surface it as a failed check
        // rather than silently dropping the element.
        kani::assert(
            self.len < CAP,
            "InlineVec capacity exceeded under Kani (raise KANI_INLINE_CAP)",
        );
        if let Some(slot) = self.slots.get_mut(self.len) {
            *slot = Some(value);
            self.len += 1;
        }
    }

    /// Returns the number of live elements. Mirrors [`Vec::len`].
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Returns a reference to the first element, or `None`. Mirrors
    /// [`Vec::first`] / `[T]::first`.
    pub(crate) fn first(&self) -> Option<&T> {
        self.get(0)
    }

    /// Returns a reference to the element at `index`, or `None`. Mirrors
    /// [`Vec::get`].
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            self.slots.get(index).and_then(Option::as_ref)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the element at `index`, or `None`. Mirrors
    /// [`Vec::get_mut`].
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len {
            self.slots.get_mut(index).and_then(Option::as_mut)
        } else {
            None
        }
    }

    /// Iterates over the live elements in order. Mirrors [`Vec::iter`].
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().take(self.len).filter_map(Option::as_ref)
    }

    /// Mutably iterates over the live elements in order. Mirrors
    /// [`Vec::iter_mut`].
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots
            .iter_mut()
            .take(self.len)
            .filter_map(Option::as_mut)
    }
}
