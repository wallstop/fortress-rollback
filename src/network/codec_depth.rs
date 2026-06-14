//! Recursion-depth-limited serde deserialization for peer-controlled values.
//!
//! # The surface this closes (B-codec)
//!
//! The bounded decode in [`super::codec`] caps the total *bytes* a peer-supplied
//! blob may allocate, but it does **not** cap the decode's *call-stack depth*.
//! bincode decodes a recursive type — one transitively containing `Box<Self>`,
//! `Vec<Self>`, `Option<Box<Self>>`, etc. — by recursing once per level of
//! nesting, and a deeply-nested value can be encoded in far fewer bytes than the
//! byte cap (each level costs only a tag/length byte or two). Such input stays
//! under the byte cap yet can overflow the thread stack mid-decode — an
//! **uncatchable abort**, not a recoverable `Err`. A malicious peer can craft
//! exactly this blob for a hot-join [`Config::State`](crate::Config::State)
//! snapshot ([`Config::State`] is only `Serialize + DeserializeOwned`, so it may
//! be recursive).
//!
//! # What this does
//!
//! [`deserialize_depth_limited`] wraps any [`serde::Deserializer`] so that every
//! level of container nesting (`seq`/`tuple`/`map`/`struct`/`enum`/`option`/
//! newtype) increments a depth counter, and a value nested deeper than the
//! configured `limit` is **rejected with a recoverable error** before the stack
//! can overflow. It is a transparent forwarding adapter otherwise: scalars and
//! the value itself are decoded byte-for-byte identically to an unwrapped
//! decode, so legitimate (shallow) states are unaffected. This is a *reject*,
//! not a *grow* (no `unsafe` stack manipulation, no new dependency), matching
//! the library's bounded-allocation / fail-closed discipline.
//!
//! # Why only `Config::State`
//!
//! [`Config::Input`](crate::Config::Input) is bound `Copy`, and a `Copy + Sized`
//! type is **provably non-recursive** (recursion requires heap indirection —
//! `Box`/`Vec`/`String` — none of which are `Copy`; a direct `enum E { N([E;2]) }`
//! is infinite-size and rejected by the compiler). So the input decode path
//! cannot be driven into unbounded recursion by malicious bytes and needs no
//! depth guard; only the `State` path (`super::codec::decode_bounded`) is
//! wrapped.

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use std::fmt;

/// Maximum container-nesting depth permitted when decoding a peer-controlled
/// value. A value nested deeper is rejected with a recoverable error instead of
/// overflowing the stack.
///
/// 128 mirrors `serde_json`'s default recursion limit: comfortably deeper than
/// any realistic game state, yet far below the depth at which serde decoding
/// overflows a typical thread stack (the exact overflow depth is type-dependent,
/// since per-level stack cost varies by `T`, but is many times this limit), so a
/// rejected blob can never have come close to a crash.
pub(crate) const MAX_DECODE_DEPTH: usize = 128;

/// Deserializes `T` from `deserializer`, rejecting any value whose container
/// nesting exceeds `limit` with a recoverable error rather than overflowing the
/// stack. Behaviourally identical to a plain decode for any value nested at most
/// `limit` levels deep.
pub(crate) fn deserialize_depth_limited<'de, T, D>(
    deserializer: D,
    limit: usize,
) -> Result<T, D::Error>
where
    T: serde::Deserialize<'de>,
    D: Deserializer<'de>,
{
    T::deserialize(DepthDeserializer {
        inner: deserializer,
        depth: 0,
        limit,
    })
}

/// The recoverable error returned when nesting exceeds the limit. Built on the
/// cold path only.
fn too_deep<E: de::Error>(limit: usize) -> E {
    E::custom(format!(
        "decode recursion depth exceeded the maximum nesting limit of {limit}"
    ))
}

// ---------------------------------------------------------------------------
// Deserializer wrapper
// ---------------------------------------------------------------------------

/// Wraps a deserializer at nesting level `depth`. Scalars forward unchanged;
/// each composite forwards through a [`DepthVisitor`] that performs the depth
/// check and re-wraps children at `depth + 1`.
struct DepthDeserializer<D> {
    inner: D,
    depth: usize,
    limit: usize,
}

/// Forwards a scalar `deserialize_*` directly: scalar visits never recurse, so
/// the original visitor is passed through unchanged.
macro_rules! forward_scalar_deserialize {
    ($($m:ident),* $(,)?) => {$(
        #[inline]
        fn $m<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de> {
            self.inner.$m(visitor)
        }
    )*};
}

/// Forwards a composite `deserialize_*` (simple `(self, visitor)` signature)
/// through a depth-tracking [`DepthVisitor`].
macro_rules! forward_composite_deserialize {
    ($($m:ident),* $(,)?) => {$(
        #[inline]
        fn $m<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de> {
            self.inner.$m(DepthVisitor { inner: visitor, depth: self.depth, limit: self.limit })
        }
    )*};
}

impl<'de, D> Deserializer<'de> for DepthDeserializer<D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    forward_scalar_deserialize!(
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_unit,
        deserialize_identifier,
    );

    // `option`, `seq` and `map` use the simple `(self, visitor)` signature and
    // dispatch to recursive visits (`visit_some`/`visit_seq`/`visit_map`), so
    // they wrap the visitor. `any`/`ignored_any` are wrapped defensively: for a
    // non-self-describing format bincode errors on them, but if a future format
    // supported them they could recurse.
    forward_composite_deserialize!(
        deserialize_option,
        deserialize_seq,
        deserialize_map,
        deserialize_any,
        deserialize_ignored_any,
    );

    #[inline]
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // A unit struct carries no nested value (`visit_unit`); forward the
        // original visitor.
        self.inner.deserialize_unit_struct(name, visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.deserialize_newtype_struct(
            name,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }

    #[inline]
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.deserialize_tuple(
            len,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.deserialize_tuple_struct(
            name,
            len,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.deserialize_struct(
            name,
            fields,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.deserialize_enum(
            name,
            variants,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

// ---------------------------------------------------------------------------
// Visitor wrapper — performs the depth check and re-wraps children
// ---------------------------------------------------------------------------

/// Wraps a visitor for a value at nesting level `depth`. Scalar visits forward
/// unchanged; recursive visits (`some`/`newtype`/`seq`/`map`/`enum`) reject when
/// `depth >= limit` and otherwise re-wrap the child deserializer/accessor at
/// `depth + 1`.
struct DepthVisitor<V> {
    inner: V,
    depth: usize,
    limit: usize,
}

/// Forwards a scalar `visit_*` (`(self, v: $t)` signature) to the inner visitor.
macro_rules! forward_scalar_visit {
    ($($m:ident($t:ty)),* $(,)?) => {$(
        #[inline]
        fn $m<E>(self, v: $t) -> Result<Self::Value, E>
        where E: de::Error {
            self.inner.$m(v)
        }
    )*};
}

impl<'de, V> Visitor<'de> for DepthVisitor<V>
where
    V: Visitor<'de>,
{
    type Value = V::Value;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(formatter)
    }

    forward_scalar_visit!(
        visit_bool(bool),
        visit_i8(i8),
        visit_i16(i16),
        visit_i32(i32),
        visit_i64(i64),
        visit_i128(i128),
        visit_u8(u8),
        visit_u16(u16),
        visit_u32(u32),
        visit_u64(u64),
        visit_u128(u128),
        visit_f32(f32),
        visit_f64(f64),
        visit_char(char),
        visit_str(&str),
        visit_string(String),
        visit_bytes(&[u8]),
        visit_byte_buf(Vec<u8>),
    );

    #[inline]
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.inner.visit_borrowed_str(v)
    }

    #[inline]
    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.inner.visit_borrowed_bytes(v)
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.inner.visit_none()
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.inner.visit_unit()
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth >= self.limit {
            return Err(too_deep(self.limit));
        }
        self.inner.visit_some(DepthDeserializer {
            inner: deserializer,
            depth: self.depth + 1,
            limit: self.limit,
        })
    }

    #[inline]
    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth >= self.limit {
            return Err(too_deep(self.limit));
        }
        self.inner.visit_newtype_struct(DepthDeserializer {
            inner: deserializer,
            depth: self.depth + 1,
            limit: self.limit,
        })
    }

    #[inline]
    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        if self.depth >= self.limit {
            return Err(too_deep(self.limit));
        }
        self.inner.visit_seq(DepthSeqAccess {
            inner: seq,
            depth: self.depth + 1,
            limit: self.limit,
        })
    }

    #[inline]
    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if self.depth >= self.limit {
            return Err(too_deep(self.limit));
        }
        self.inner.visit_map(DepthMapAccess {
            inner: map,
            depth: self.depth + 1,
            limit: self.limit,
        })
    }

    #[inline]
    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        if self.depth >= self.limit {
            return Err(too_deep(self.limit));
        }
        self.inner.visit_enum(DepthEnumAccess {
            inner: data,
            depth: self.depth + 1,
            limit: self.limit,
        })
    }
}

// ---------------------------------------------------------------------------
// Access wrappers — propagate `depth + 1` to every child deserialize
// ---------------------------------------------------------------------------

/// Wraps a `SeqAccess`; every element deserializes at the child `depth`.
struct DepthSeqAccess<A> {
    inner: A,
    depth: usize,
    limit: usize,
}

impl<'de, A> SeqAccess<'de> for DepthSeqAccess<A>
where
    A: SeqAccess<'de>,
{
    type Error = A::Error;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.inner.next_element_seed(DepthSeed {
            inner: seed,
            depth: self.depth,
            limit: self.limit,
        })
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

/// Wraps a `MapAccess`; every key and value deserializes at the child `depth`.
struct DepthMapAccess<A> {
    inner: A,
    depth: usize,
    limit: usize,
}

impl<'de, A> MapAccess<'de> for DepthMapAccess<A>
where
    A: MapAccess<'de>,
{
    type Error = A::Error;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        self.inner.next_key_seed(DepthSeed {
            inner: seed,
            depth: self.depth,
            limit: self.limit,
        })
    }

    #[inline]
    fn next_value_seed<Va>(&mut self, seed: Va) -> Result<Va::Value, Self::Error>
    where
        Va: DeserializeSeed<'de>,
    {
        self.inner.next_value_seed(DepthSeed {
            inner: seed,
            depth: self.depth,
            limit: self.limit,
        })
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

/// Wraps an `EnumAccess`. The variant *selector* is an identifier (a scalar that
/// cannot recurse) and is deserialized directly; only the variant *payload*
/// (via [`DepthVariantAccess`]) is depth-tracked.
struct DepthEnumAccess<A> {
    inner: A,
    depth: usize,
    limit: usize,
}

impl<'de, A> EnumAccess<'de> for DepthEnumAccess<A>
where
    A: EnumAccess<'de>,
{
    type Error = A::Error;
    type Variant = DepthVariantAccess<A::Variant>;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (value, variant) = self.inner.variant_seed(seed)?;
        Ok((
            value,
            DepthVariantAccess {
                inner: variant,
                depth: self.depth,
                limit: self.limit,
            },
        ))
    }
}

/// Wraps a `VariantAccess`; the variant payload deserializes at the child
/// `depth`.
struct DepthVariantAccess<A> {
    inner: A,
    depth: usize,
    limit: usize,
}

impl<'de, A> VariantAccess<'de> for DepthVariantAccess<A>
where
    A: VariantAccess<'de>,
{
    type Error = A::Error;

    #[inline]
    fn unit_variant(self) -> Result<(), Self::Error> {
        self.inner.unit_variant()
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.inner.newtype_variant_seed(DepthSeed {
            inner: seed,
            depth: self.depth,
            limit: self.limit,
        })
    }

    #[inline]
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.tuple_variant(
            len,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }

    #[inline]
    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.inner.struct_variant(
            fields,
            DepthVisitor {
                inner: visitor,
                depth: self.depth,
                limit: self.limit,
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Seed wrapper — re-enters the depth-limited deserializer for each child
// ---------------------------------------------------------------------------

/// Wraps a `DeserializeSeed` so the child value is deserialized through a
/// [`DepthDeserializer`] at `depth`.
struct DepthSeed<S> {
    inner: S,
    depth: usize,
    limit: usize,
}

impl<'de, S> DeserializeSeed<'de> for DepthSeed<S>
where
    S: DeserializeSeed<'de>,
{
    type Value = S::Value;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.inner.deserialize(DepthDeserializer {
            inner: deserializer,
            depth: self.depth,
            limit: self.limit,
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    fn bincode_config() -> impl bincode::config::Config {
        bincode::config::standard().with_fixed_int_encoding()
    }

    fn encode<T: Serialize>(value: &T) -> Vec<u8> {
        bincode::serde::encode_to_vec(value, bincode_config()).expect("encode")
    }

    /// Decode through the depth-limited wrapper at `limit`.
    fn decode_depth_limited<T: DeserializeOwned>(
        bytes: &[u8],
        limit: usize,
    ) -> Result<T, bincode::error::DecodeError> {
        let mut decoder =
            bincode::serde::BorrowedSerdeDecoder::from_slice(bytes, bincode_config(), ());
        deserialize_depth_limited::<T, _>(decoder.as_deserializer(), limit)
    }

    /// Decode WITHOUT the wrapper — the control for non-vacuity (proves a
    /// rejection comes from the depth guard, not malformed bytes).
    fn decode_plain<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, bincode::error::DecodeError> {
        bincode::serde::decode_from_slice(bytes, bincode_config()).map(|(v, _)| v)
    }

    // A type exercising every serde container kind so a missing forward or
    // accessor bug in the wrapper would corrupt the round-trip.
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Inner {
        x: u32,
        next: Option<Box<Self>>, // recursive + option + newtype
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    enum Choice {
        Unit,
        Newtype(i64),
        Tuple(u8, bool),
        Struct { values: Vec<u8> },
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Rich {
        flag: bool,
        nums: Vec<i64>,
        name: String,
        maybe: Option<u32>,
        pair: (u8, i16),
        map: BTreeMap<String, u64>,
        raw: Vec<u8>,
        nested: Vec<Inner>,
        choices: Vec<Choice>,
        unit: (),
    }

    fn sample_rich() -> Rich {
        let mut map = BTreeMap::new();
        map.insert("a".to_owned(), 1u64);
        map.insert("bb".to_owned(), 22u64);
        Rich {
            flag: true,
            nums: vec![-1, 0, 7, i64::MAX, i64::MIN],
            name: "fortress".to_owned(),
            maybe: Some(99),
            pair: (3, -300),
            map,
            raw: vec![0, 1, 2, 250, 255],
            nested: vec![
                Inner { x: 1, next: None },
                Inner {
                    x: 2,
                    next: Some(Box::new(Inner { x: 3, next: None })),
                },
            ],
            choices: vec![
                Choice::Unit,
                Choice::Newtype(-5),
                Choice::Tuple(9, true),
                Choice::Struct {
                    values: vec![4, 5, 6],
                },
            ],
            unit: (),
        }
    }

    /// The wrapper is transparent: any value within the depth limit decodes
    /// byte-for-byte identically to a plain decode (catches a missing `visit_*`
    /// forward or a broken access wrapper).
    #[test]
    fn depth_limited_decode_is_transparent_for_diverse_types() {
        let value = sample_rich();
        let bytes = encode(&value);

        let via_wrapper: Rich =
            decode_depth_limited(&bytes, MAX_DECODE_DEPTH).expect("decode within limit");
        assert_eq!(
            via_wrapper, value,
            "wrapper must not alter the decoded value"
        );

        // And identical to a plain (unwrapped) decode.
        let via_plain: Rich = decode_plain(&bytes).expect("plain decode");
        assert_eq!(
            via_wrapper, via_plain,
            "wrapper must match an unwrapped decode"
        );
    }

    /// Scalars and an empty/shallow value are unaffected.
    #[test]
    fn depth_limited_decode_handles_scalars_and_empty_containers() {
        for limit in [1usize, 2, MAX_DECODE_DEPTH] {
            let n = encode(&123_456_789u64);
            assert_eq!(decode_depth_limited::<u64>(&n, limit).unwrap(), 123_456_789);

            let empty: Vec<u32> = Vec::new();
            let e = encode(&empty);
            assert_eq!(decode_depth_limited::<Vec<u32>>(&e, limit).unwrap(), empty);
        }
    }

    // A linearly-recursive type so we can dial nesting depth precisely.
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    enum Nest {
        Leaf(u32),
        Node(Box<Self>),
    }

    fn build_nest(nodes: usize) -> Nest {
        let mut n = Nest::Leaf(7);
        for _ in 0..nodes {
            n = Nest::Node(Box::new(n));
        }
        n
    }

    /// The guard rejects nesting at/above the limit with a recoverable error and
    /// accepts nesting below it — and is NON-VACUOUS: the rejected blob decodes
    /// fine WITHOUT the guard, so the `Err` is the guard, not malformed bytes.
    #[test]
    fn depth_limited_decode_rejects_beyond_limit_accepts_below() {
        let limit = 5usize;

        // `build_nest(d)` reaches serde nesting depth `d`; the guard fires when
        // depth reaches `limit`, so `d < limit` is accepted and `d >= limit` is
        // rejected.
        let shallow = encode(&build_nest(limit - 1));
        assert_eq!(
            decode_depth_limited::<Nest>(&shallow, limit).unwrap(),
            build_nest(limit - 1),
            "nesting just below the limit must decode"
        );

        let deep = encode(&build_nest(limit));
        let err = decode_depth_limited::<Nest>(&deep, limit);
        assert!(
            err.is_err(),
            "nesting at the limit must be rejected with an error, not accepted"
        );

        // Non-vacuity: the same bytes decode fine without the guard.
        assert_eq!(
            decode_plain::<Nest>(&deep).unwrap(),
            build_nest(limit),
            "control: the rejected blob is well-formed (rejection is the guard)"
        );

        // A much deeper blob is also rejected (not an abort) — the whole point.
        let very_deep = encode(&build_nest(limit + 500));
        assert!(
            decode_depth_limited::<Nest>(&very_deep, limit).is_err(),
            "far-too-deep nesting must be a recoverable Err, never a stack-overflow abort"
        );
    }

    // Recursion through a MAP (value type is `Self`) — exercises the
    // `DepthMapAccess` path, distinct from the seq/enum path `Nest` uses.
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
    struct MapNest(BTreeMap<u8, Self>);

    fn build_map_nest(depth: usize) -> MapNest {
        let mut n = MapNest::default();
        for _ in 0..depth {
            let mut m = BTreeMap::new();
            m.insert(0u8, n);
            n = MapNest(m);
        }
        n
    }

    // Recursion through a STRUCT-VARIANT payload — exercises
    // `DepthVariantAccess::struct_variant`, distinct again.
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    enum StructVariantNest {
        Leaf,
        Node { child: Box<Self> },
    }

    fn build_struct_variant_nest(depth: usize) -> StructVariantNest {
        let mut n = StructVariantNest::Leaf;
        for _ in 0..depth {
            n = StructVariantNest::Node { child: Box::new(n) };
        }
        n
    }

    /// The guard also bounds recursion that flows through the MAP and
    /// STRUCT-VARIANT accessors (not just the seq/enum path the other tests use),
    /// each proven non-vacuous by a successful unguarded control decode. This
    /// locks in `DepthMapAccess` and `DepthVariantAccess::struct_variant` against
    /// a future refactor that forgot to re-wrap one of those child paths.
    #[test]
    fn depth_limit_covers_map_and_struct_variant_recursion() {
        let limit = 5usize;

        let deep_map = encode(&build_map_nest(limit + 10));
        assert!(
            decode_depth_limited::<MapNest>(&deep_map, limit).is_err(),
            "map-value recursion past the limit must be rejected (DepthMapAccess)"
        );
        assert!(
            decode_plain::<MapNest>(&deep_map).is_ok(),
            "control: the deep map blob is well-formed (rejection is the guard)"
        );

        let deep_sv = encode(&build_struct_variant_nest(limit + 10));
        assert!(
            decode_depth_limited::<StructVariantNest>(&deep_sv, limit).is_err(),
            "struct-variant payload recursion past the limit must be rejected (DepthVariantAccess)"
        );
        assert!(
            decode_plain::<StructVariantNest>(&deep_sv).is_ok(),
            "control: the deep struct-variant blob is well-formed (rejection is the guard)"
        );

        // And a shallow value of each still round-trips unchanged under the
        // production limit. (Each logical level of these types costs ~2 serde
        // nesting levels — the newtype-struct/enum wrapper plus the map/struct —
        // so the guard is conservatively stricter than the logical depth; use
        // the generous production limit here to show shallow values are fine.)
        let shallow_map = encode(&build_map_nest(2));
        assert_eq!(
            decode_depth_limited::<MapNest>(&shallow_map, MAX_DECODE_DEPTH).unwrap(),
            build_map_nest(2),
        );
        let shallow_sv = encode(&build_struct_variant_nest(2));
        assert_eq!(
            decode_depth_limited::<StructVariantNest>(&shallow_sv, MAX_DECODE_DEPTH).unwrap(),
            build_struct_variant_nest(2),
        );
    }

    /// End-to-end through the production entry point `codec::decode_bounded`
    /// (which uses [`MAX_DECODE_DEPTH`]): a deeply-recursive `Config::State`-shaped
    /// blob is rejected with a recoverable error instead of aborting, while a
    /// realistically-nested one decodes.
    #[test]
    fn decode_bounded_rejects_deeply_nested_state() {
        use crate::network::codec;

        let ok = encode(&build_nest(MAX_DECODE_DEPTH - 1));
        assert_eq!(
            codec::decode_bounded::<Nest>(&ok).expect("within MAX_DECODE_DEPTH"),
            build_nest(MAX_DECODE_DEPTH - 1),
        );

        let too_deep = encode(&build_nest(MAX_DECODE_DEPTH + 50));
        assert!(
            codec::decode_bounded::<Nest>(&too_deep).is_err(),
            "decode_bounded must reject a state nested past MAX_DECODE_DEPTH"
        );
        // Non-vacuity: well-formed bytes (a plain decode succeeds).
        assert_eq!(
            decode_plain::<Nest>(&too_deep).unwrap(),
            build_nest(MAX_DECODE_DEPTH + 50),
        );
    }
}
