//! Structured telemetry pipeline for specification violations.
//!
//! This module provides an observable system for tracking specification violations
//! and internal invariant failures. Instead of just logging with `tracing::warn!`,
//! violations are structured data that can be:
//!
//! - Logged via tracing (default behavior, backward compatible)
//! - Collected programmatically for testing
//! - Sent to custom observers (metrics, alerting, etc.)
//!
//! # Example
//!
//! ```
//! use fortress_rollback::telemetry::{ViolationSeverity, ViolationKind, CollectingObserver};
//! use std::sync::Arc;
//!
//! // Create a collecting observer for tests
//! let observer = Arc::new(CollectingObserver::new());
//!
//! // Check violations after some operations
//! assert!(observer.violations().is_empty(), "unexpected violations");
//! ```

use crate::network::network_stats::NetworkStats;
use crate::sync::Mutex;
use crate::{Frame, PlayerHandle};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Custom serializer for `Option<Frame>` that outputs clean integers or null.
///
/// - `None` → `null`
/// - `Some(Frame::NULL)` → `null`
/// - `Some(Frame(n))` where n >= 0 → `n`
mod frame_serializer {
    use crate::Frame;
    use serde::Serializer;

    #[allow(clippy::ref_option)]
    pub fn serialize<S>(frame: &Option<Frame>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match frame {
            None => serializer.serialize_none(),
            Some(f) if f.is_null() => serializer.serialize_none(),
            Some(f) => serializer.serialize_i32(f.as_i32()),
        }
    }
}

/// Severity of a specification violation.
///
/// Severities are ordered from least to most severe, allowing filtering
/// and comparison operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    /// Unexpected but recoverable - operation continued with fallback.
    ///
    /// Example: A minor timing issue that was automatically corrected.
    Warning,
    /// Serious issue - operation may have degraded behavior.
    ///
    /// Example: Frame mismatch after load that may affect simulation.
    Error,
    /// Critical invariant broken - state may be corrupted.
    ///
    /// Example: Input queue corruption that could cause desync.
    Critical,
}

impl ViolationSeverity {
    /// Returns a string representation suitable for logging/metrics labels.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

impl std::fmt::Display for ViolationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Categories of specification violations.
///
/// Each category corresponds to a major subsystem of the library,
/// making it easy to filter and route violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationKind {
    /// Frame synchronization invariant violated.
    ///
    /// Examples:
    /// - Frame counter mismatch after load
    /// - Unexpected frame value during resimulation
    FrameSync,
    /// Input queue invariant violated.
    ///
    /// Examples:
    /// - Gap in input sequence
    /// - Confirmation of already-confirmed input
    InputQueue,
    /// State save/load invariant violated.
    ///
    /// Examples:
    /// - Loading non-existent state
    /// - State checksum mismatch
    StateManagement,
    /// Network protocol invariant violated.
    ///
    /// Examples:
    /// - Unexpected message type
    /// - Protocol state machine error
    NetworkProtocol,
    /// Checksum or desync detection issue.
    ///
    /// Examples:
    /// - Local/remote checksum mismatch
    /// - Unable to compute checksum
    ChecksumMismatch,
    /// Configuration constraint violated.
    ///
    /// Examples:
    /// - Invalid parameter combination
    /// - Constraint violation at runtime
    Configuration,
    /// Internal logic error (should never happen).
    ///
    /// These violations indicate bugs in the library itself.
    InternalError,
    /// Runtime invariant check failed.
    ///
    /// These violations indicate that a type's invariants were broken,
    /// which could lead to undefined behavior or incorrect results.
    /// Only checked in debug builds or when `paranoid` feature is enabled.
    Invariant,
    /// Synchronization protocol issues.
    ///
    /// Examples:
    /// - Excessive sync retries due to packet loss
    /// - Sync duration exceeding expected time
    /// - Repeated sync failures before connection established
    Synchronization,
    /// Arithmetic overflow detected.
    ///
    /// This indicates that a frame counter or similar value would overflow.
    /// In practice, this should never happen (at 60 FPS, i32::MAX takes ~1.14 years),
    /// but detecting it helps catch bugs in edge cases or adversarial inputs.
    ArithmeticOverflow,
}

impl ViolationKind {
    /// Returns a string representation suitable for logging/metrics labels.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FrameSync => "frame_sync",
            Self::InputQueue => "input_queue",
            Self::StateManagement => "state_management",
            Self::NetworkProtocol => "network_protocol",
            Self::ChecksumMismatch => "checksum_mismatch",
            Self::Configuration => "configuration",
            Self::InternalError => "internal_error",
            Self::Invariant => "invariant",
            Self::Synchronization => "synchronization",
            Self::ArithmeticOverflow => "arithmetic_overflow",
        }
    }
}

impl std::fmt::Display for ViolationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A recorded specification violation.
///
/// Contains all relevant context for diagnosing and responding to
/// a violation of expected behavior or invariants.
///
/// # Serialization
///
/// This type implements `serde::Serialize` for structured JSON output.
/// The frame field is serialized as `null` for [`Frame::NULL`], or as an
/// integer for valid frames.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{SpecViolation, ViolationSeverity, ViolationKind};
/// use fortress_rollback::Frame;
///
/// let violation = SpecViolation::new(
///     ViolationSeverity::Warning,
///     ViolationKind::FrameSync,
///     "frame mismatch",
///     "sync.rs:42",
/// ).with_frame(Frame::new(100))
///  .with_context("expected", "50")
///  .with_context("actual", "100");
///
/// // Serialize to JSON
/// let json = serde_json::to_string(&violation)?;
/// assert!(json.contains(r#""severity":"warning""#));
/// assert!(json.contains(r#""kind":"frame_sync""#));
/// assert!(json.contains(r#""frame":100"#));
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpecViolation {
    /// The severity level of this violation.
    pub severity: ViolationSeverity,
    /// The category/subsystem where the violation occurred.
    pub kind: ViolationKind,
    /// Human-readable description of what went wrong.
    pub message: String,
    /// Source location where the violation was detected (file:line).
    pub location: &'static str,
    /// The game frame at which the violation occurred, if applicable.
    ///
    /// Serialized as an integer for valid frames, or `null` for `None`/[`Frame::NULL`].
    #[serde(serialize_with = "frame_serializer::serialize")]
    pub frame: Option<Frame>,
    /// Additional structured context as key-value pairs.
    ///
    /// This can include values like player handles, expected vs actual
    /// values, or other diagnostic information.
    pub context: BTreeMap<String, String>,
}

impl SpecViolation {
    /// Creates a new specification violation.
    ///
    /// Marked `#[cold]` because violations should be rare in normal operation.
    /// `#[inline(never)]` prevents error-path code from polluting caller instruction cache.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn new(
        severity: ViolationSeverity,
        kind: ViolationKind,
        message: impl Into<String>,
        location: &'static str,
    ) -> Self {
        Self {
            severity,
            kind,
            message: message.into(),
            location,
            frame: None,
            context: BTreeMap::new(),
        }
    }

    /// Sets the frame at which this violation occurred.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Adds a context key-value pair.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Serializes this violation to a JSON string.
    ///
    /// This is a convenience method for programmatic access to violation data.
    /// Returns `None` if serialization fails (which should not happen for
    /// well-formed violations).
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::telemetry::{SpecViolation, ViolationSeverity, ViolationKind};
    /// use fortress_rollback::Frame;
    ///
    /// let violation = SpecViolation::new(
    ///     ViolationSeverity::Warning,
    ///     ViolationKind::FrameSync,
    ///     "test",
    ///     "test.rs:1",
    /// ).with_frame(Frame::new(42));
    ///
    /// # #[cfg(feature = "json")]
    /// # {
    /// if let Some(json) = violation.to_json() {
    ///     assert!(json.contains(r#""frame":42"#));
    /// }
    /// # }
    /// ```
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// Serializes this violation to a pretty-printed JSON string.
    ///
    /// Like [`to_json`](Self::to_json), but with indentation for readability.
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json_pretty(&self) -> Option<String> {
        serde_json::to_string_pretty(self).ok()
    }
}

impl std::fmt::Display for SpecViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}/{}] {} (at {}",
            self.severity, self.kind, self.message, self.location
        )?;
        if let Some(frame) = self.frame {
            write!(f, ", frame={frame}")?;
        }
        if !self.context.is_empty() {
            write!(f, ", context={:?}", self.context)?;
        }
        write!(f, ")")
    }
}

/// Trait for observing specification violations.
///
/// Implement this trait to create custom observers that can react to
/// violations in various ways (logging, metrics, alerting, etc.).
///
/// # Thread Safety
///
/// When the `sync-send` feature is enabled, observers must be `Send + Sync`
/// to allow sharing across threads.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{ViolationObserver, SpecViolation};
///
/// struct MetricsObserver {
///     // Your metrics implementation
/// }
///
/// impl ViolationObserver for MetricsObserver {
///     fn on_violation(&self, violation: &SpecViolation) {
///         // Increment a counter, send to monitoring system, etc.
///         println!("Violation: {}", violation);
///     }
/// }
/// ```
#[cfg(feature = "sync-send")]
pub trait ViolationObserver: Send + Sync {
    /// Called when a specification violation is detected.
    ///
    /// This method should be relatively quick to execute, as it may be
    /// called during time-critical operations.
    fn on_violation(&self, violation: &SpecViolation);
}

#[cfg(not(feature = "sync-send"))]
/// Trait for observing specification violations.
///
/// Implement this trait to create custom observers that can react to
/// violations in various ways (logging, metrics, alerting, etc.).
pub trait ViolationObserver {
    /// Called when a specification violation is detected.
    fn on_violation(&self, violation: &SpecViolation);
}

/// Built-in observer that logs violations via the `tracing` crate.
///
/// This is the default observer that maintains backward compatibility
/// with the previous `tracing::warn!` behavior, but with improved
/// structured output for machine parseability.
///
/// # Log Levels
///
/// - `Warning` severity → `tracing::warn!`
/// - `Error` severity → `tracing::error!`
/// - `Critical` severity → `tracing::error!` with additional context
///
/// # Structured Output
///
/// All fields are output as structured tracing fields:
/// - `severity` - The severity level as a string (`warning`, `error`, `critical`)
/// - `kind` - The violation category as a string (e.g., `frame_sync`, `input_queue`)
/// - `location` - Source file and line number where the violation was detected
/// - `frame` - The frame number as an integer, or "null" if not applicable
/// - `context` - A compact representation of context key-value pairs
///
/// This structured output is compatible with JSON logging formatters
/// (like `tracing-subscriber`'s JSON layer) and log aggregation systems.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TracingObserver;

impl TracingObserver {
    /// Creates a new tracing observer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Formats the frame as a displayable value.
    /// Returns the frame number for valid frames, or "null" for None/NULL frames.
    fn format_frame(frame: Option<Frame>) -> String {
        match frame {
            None => "null".to_string(),
            Some(f) if f.is_null() => "null".to_string(),
            Some(f) => f.as_i32().to_string(),
        }
    }
}

impl ViolationObserver for TracingObserver {
    fn on_violation(&self, violation: &SpecViolation) {
        let severity = violation.severity.as_str();
        let kind = violation.kind.as_str();
        let location = violation.location;
        let frame_str = Self::format_frame(violation.frame);

        // Format context as a compact key=value string for compatibility
        // with systems that don't support dynamic field expansion
        let context_str = if violation.context.is_empty() {
            "{}".to_string()
        } else {
            let pairs: Vec<String> = violation
                .context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        };

        match violation.severity {
            ViolationSeverity::Warning => {
                tracing::warn!(
                    severity,
                    kind,
                    location,
                    frame = %frame_str,
                    context = %context_str,
                    "{}",
                    violation.message
                );
            },
            ViolationSeverity::Error => {
                tracing::error!(
                    severity,
                    kind,
                    location,
                    frame = %frame_str,
                    context = %context_str,
                    "{}",
                    violation.message
                );
            },
            ViolationSeverity::Critical => {
                tracing::error!(
                    severity = "critical",
                    kind,
                    location,
                    frame = %frame_str,
                    context = %context_str,
                    "{}",
                    violation.message
                );
            },
        }
    }
}

/// Built-in observer that collects violations for testing.
///
/// This observer stores all violations in a thread-safe vector,
/// allowing tests to assert on the violations that occurred during
/// an operation.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{CollectingObserver, ViolationKind, ViolationObserver, SpecViolation, ViolationSeverity};
///
/// let observer = CollectingObserver::new();
///
/// // Simulate a violation being reported
/// observer.on_violation(&SpecViolation::new(
///     ViolationSeverity::Warning,
///     ViolationKind::FrameSync,
///     "test violation",
///     "test.rs:1",
/// ));
///
/// // Check that the violation was collected
/// assert_eq!(observer.violations().len(), 1);
/// assert!(observer.has_violation(ViolationKind::FrameSync));
/// ```
#[derive(Debug, Default)]
pub struct CollectingObserver {
    violations: Mutex<Vec<SpecViolation>>,
}

impl CollectingObserver {
    /// Creates a new collecting observer with an empty violation list.
    #[must_use]
    pub fn new() -> Self {
        Self {
            violations: Mutex::new(Vec::new()),
        }
    }

    /// Returns a copy of all collected violations.
    #[cfg(not(loom))]
    #[must_use]
    pub fn violations(&self) -> Vec<SpecViolation> {
        self.violations.lock().clone()
    }

    /// Returns a copy of all collected violations (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn violations(&self) -> Vec<SpecViolation> {
        self.violations.lock().unwrap().clone()
    }

    /// Returns the number of collected violations.
    #[cfg(not(loom))]
    #[must_use]
    pub fn len(&self) -> usize {
        self.violations.lock().len()
    }

    /// Returns the number of collected violations (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.violations.lock().unwrap().len()
    }

    /// Returns true if no violations have been collected.
    #[cfg(not(loom))]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.violations.lock().is_empty()
    }

    /// Returns true if no violations have been collected (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.violations.lock().unwrap().is_empty()
    }

    /// Checks if any violation of the specified kind has been collected.
    #[cfg(not(loom))]
    #[must_use]
    pub fn has_violation(&self, kind: ViolationKind) -> bool {
        self.violations.lock().iter().any(|v| v.kind == kind)
    }

    /// Checks if any violation of the specified kind has been collected (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn has_violation(&self, kind: ViolationKind) -> bool {
        self.violations
            .lock()
            .unwrap()
            .iter()
            .any(|v| v.kind == kind)
    }

    /// Checks if any violation with the specified severity has been collected.
    #[cfg(not(loom))]
    #[must_use]
    pub fn has_severity(&self, severity: ViolationSeverity) -> bool {
        self.violations
            .lock()
            .iter()
            .any(|v| v.severity == severity)
    }

    /// Checks if any violation with the specified severity has been collected (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn has_severity(&self, severity: ViolationSeverity) -> bool {
        self.violations
            .lock()
            .unwrap()
            .iter()
            .any(|v| v.severity == severity)
    }

    /// Returns all violations matching the specified kind.
    #[cfg(not(loom))]
    #[must_use]
    pub fn violations_of_kind(&self, kind: ViolationKind) -> Vec<SpecViolation> {
        self.violations
            .lock()
            .iter()
            .filter(|v| v.kind == kind)
            .cloned()
            .collect()
    }

    /// Returns all violations matching the specified kind (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn violations_of_kind(&self, kind: ViolationKind) -> Vec<SpecViolation> {
        self.violations
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.kind == kind)
            .cloned()
            .collect()
    }

    /// Returns all violations at or above the specified severity.
    #[cfg(not(loom))]
    #[must_use]
    pub fn violations_at_severity(&self, min_severity: ViolationSeverity) -> Vec<SpecViolation> {
        self.violations
            .lock()
            .iter()
            .filter(|v| v.severity >= min_severity)
            .cloned()
            .collect()
    }

    /// Returns all violations at or above the specified severity (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn violations_at_severity(&self, min_severity: ViolationSeverity) -> Vec<SpecViolation> {
        self.violations
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.severity >= min_severity)
            .cloned()
            .collect()
    }

    /// Clears all collected violations.
    #[cfg(not(loom))]
    pub fn clear(&self) {
        self.violations.lock().clear();
    }

    /// Clears all collected violations (loom version).
    #[cfg(loom)]
    pub fn clear(&self) {
        self.violations.lock().unwrap().clear();
    }
}

#[cfg(not(loom))]
impl ViolationObserver for CollectingObserver {
    fn on_violation(&self, violation: &SpecViolation) {
        self.violations.lock().push(violation.clone());
    }
}

#[cfg(loom)]
impl ViolationObserver for CollectingObserver {
    fn on_violation(&self, violation: &SpecViolation) {
        self.violations.lock().unwrap().push(violation.clone());
    }
}

/// A composite observer that forwards violations to multiple observers.
///
/// Useful when you want to both log violations and collect them for testing,
/// or when you have multiple monitoring systems.
#[derive(Default)]
pub struct CompositeObserver {
    observers: Vec<Arc<dyn ViolationObserver>>,
}

impl CompositeObserver {
    /// Creates a new composite observer with no child observers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    /// Adds an observer to the composite.
    pub fn add(&mut self, observer: Arc<dyn ViolationObserver>) {
        self.observers.push(observer);
    }

    /// Creates a composite observer from a list of observers.
    #[must_use]
    pub fn from_observers(observers: Vec<Arc<dyn ViolationObserver>>) -> Self {
        Self { observers }
    }
}

impl ViolationObserver for CompositeObserver {
    fn on_violation(&self, violation: &SpecViolation) {
        for observer in &self.observers {
            observer.on_violation(violation);
        }
    }
}

impl std::fmt::Debug for CompositeObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeObserver")
            .field("num_observers", &self.observers.len())
            .finish()
    }
}

/// Macro for reporting specification violations with location tracking.
///
/// This macro creates a [`SpecViolation`] with the current file and line,
/// and reports it to the global observer (if set) or to a provided observer.
///
/// # Syntax
///
/// ```text
/// report_violation!(severity, kind, "message");
/// report_violation!(severity, kind, "message with {}", format_args);
/// ```
///
/// # Example
///
/// ```
/// use fortress_rollback::{report_violation, telemetry::{ViolationSeverity, ViolationKind}};
///
/// let expected = 10;
/// let actual = 15;
///
/// // Simple usage
/// report_violation!(ViolationSeverity::Warning, ViolationKind::FrameSync,
///     "frame mismatch: expected={}, actual={}", expected, actual);
/// ```
///
/// # Kani (Formal Verification)
///
/// Under `cfg(kani)`, this macro evaluates its arguments (to suppress unused
/// import/variable warnings) but skips `format!()` and tracing. The formatting
/// and tracing infrastructure create massive symbolic state space for CBMC,
/// causing proof timeouts. Since this macro only performs logging (no state
/// mutation), skipping reporting under Kani does not affect correctness verification.
#[macro_export]
macro_rules! report_violation {
    // Under Kani, report_violation is a no-op to avoid CBMC state explosion
    // from format!() and tracing infrastructure. Kani proofs verify correctness
    // properties, not logging behavior.

    // Basic: severity, kind, message (no format args)
    ($severity:expr, $kind:expr, $msg:literal) => {{
        #[cfg(not(kani))]
        {
            use $crate::telemetry::ViolationObserver as _;
            let violation = $crate::telemetry::SpecViolation::new(
                $severity,
                $kind,
                $msg,
                concat!(file!(), ":", line!()),
            );
            $crate::telemetry::TracingObserver.on_violation(&violation);
        }
        // Under Kani, evaluate severity and kind to suppress unused import warnings
        // for ViolationSeverity/ViolationKind, but avoid format!() and tracing
        // which cause CBMC state space explosion.
        #[cfg(kani)]
        {
            let _ = ($severity, $kind);
        }
    }};

    // With format args: severity, kind, format, args...
    ($severity:expr, $kind:expr, $fmt:literal, $($arg:tt)+) => {{
        #[cfg(not(kani))]
        {
            use $crate::telemetry::ViolationObserver as _;
            let violation = $crate::telemetry::SpecViolation::new(
                $severity,
                $kind,
                format!($fmt, $($arg)+),
                concat!(file!(), ":", line!()),
            );
            $crate::telemetry::TracingObserver.on_violation(&violation);
        }
        // Under Kani, evaluate severity, kind, and all format arguments to suppress
        // unused import/variable warnings, but avoid format!() and tracing which
        // cause CBMC state space explosion.
        #[cfg(kani)]
        {
            let _ = ($severity, $kind, $($arg)+);
        }
    }};
}

/// Safely adds a value to a Frame, reporting a violation if overflow would occur.
///
/// Returns the result of checked addition, or the saturated value if overflow occurs.
/// When overflow is detected, a violation is reported with the ArithmeticOverflow kind.
///
/// # Example
///
/// ```ignore
/// use fortress_rollback::{Frame, safe_frame_add};
///
/// let frame = Frame::new(100);
/// let result = safe_frame_add!(frame, 50, "advancing game frame");
/// assert_eq!(result, Frame::new(150));
/// ```
#[macro_export]
macro_rules! safe_frame_add {
    ($frame:expr, $delta:expr, $context:expr) => {{
        let frame: $crate::Frame = $frame;
        let delta: i32 = $delta;
        match frame.checked_add(delta) {
            Some(result) => result,
            None => {
                $crate::report_violation!(
                    $crate::telemetry::ViolationSeverity::Error,
                    $crate::telemetry::ViolationKind::ArithmeticOverflow,
                    "Frame overflow in {}: {} + {} would overflow",
                    $context,
                    frame,
                    delta
                );
                frame.saturating_add(delta)
            },
        }
    }};
}

/// Safely subtracts a value from a Frame, reporting a violation if overflow would occur.
///
/// Returns the result of checked subtraction, or the saturated value if overflow occurs.
/// When overflow is detected, a violation is reported with the ArithmeticOverflow kind.
///
/// # Example
///
/// ```ignore
/// use fortress_rollback::{Frame, safe_frame_sub};
///
/// let frame = Frame::new(100);
/// let result = safe_frame_sub!(frame, 50, "rolling back frame");
/// assert_eq!(result, Frame::new(50));
/// ```
#[macro_export]
macro_rules! safe_frame_sub {
    ($frame:expr, $delta:expr, $context:expr) => {{
        let frame: $crate::Frame = $frame;
        let delta: i32 = $delta;
        match frame.checked_sub(delta) {
            Some(result) => result,
            None => {
                $crate::report_violation!(
                    $crate::telemetry::ViolationSeverity::Error,
                    $crate::telemetry::ViolationKind::ArithmeticOverflow,
                    "Frame underflow in {}: {} - {} would underflow",
                    $context,
                    frame,
                    delta
                );
                frame.saturating_sub(delta)
            },
        }
    }};
}

/// Asserts that no violations have been collected.
///
/// # Panics
///
/// Panics if the observer contains any violations, printing them for debugging.
///
/// # Example
///
/// ```
/// use fortress_rollback::{assert_no_violations, telemetry::CollectingObserver};
///
/// let observer = CollectingObserver::new();
/// // ... run some operations ...
/// assert_no_violations!(observer);
/// ```
#[macro_export]
macro_rules! assert_no_violations {
    ($observer:expr) => {{
        let violations = $observer.violations();
        assert!(
            violations.is_empty(),
            "Expected no violations, but found {}:\n{:#?}",
            violations.len(),
            violations
        );
    }};

    ($observer:expr, $msg:expr) => {{
        let violations = $observer.violations();
        assert!(
            violations.is_empty(),
            "{}\nExpected no violations, but found {}:\n{:#?}",
            $msg,
            violations.len(),
            violations
        );
    }};
}

/// Asserts that a violation of the specified kind was collected.
///
/// # Panics
///
/// Panics if no violation of the specified kind was found.
///
/// # Example
///
/// ```
/// use fortress_rollback::{assert_violation, telemetry::{CollectingObserver, ViolationKind, ViolationObserver, SpecViolation, ViolationSeverity}};
///
/// let observer = CollectingObserver::new();
/// observer.on_violation(&SpecViolation::new(
///     ViolationSeverity::Warning,
///     ViolationKind::FrameSync,
///     "test",
///     "test.rs:1",
/// ));
/// assert_violation!(observer, ViolationKind::FrameSync);
/// ```
#[macro_export]
macro_rules! assert_violation {
    ($observer:expr, $kind:expr) => {{
        assert!(
            $observer.has_violation($kind),
            "Expected violation of kind {:?}, but found: {:#?}",
            $kind,
            $observer.violations()
        );
    }};

    ($observer:expr, $kind:expr, $msg:expr) => {{
        assert!(
            $observer.has_violation($kind),
            "{}\nExpected violation of kind {:?}, but found: {:#?}",
            $msg,
            $kind,
            $observer.violations()
        );
    }};
}

/// Reports a violation to an optional observer, falling back to [`TracingObserver`] if `None`.
///
/// This function is used internally by sessions to report violations through their
/// configured observer, while maintaining backward compatibility with the default
/// tracing-based logging.
///
/// # Arguments
///
/// * `observer` - Optional reference to a violation observer
/// * `violation` - The violation to report
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{
///     report_to_observer, CollectingObserver, SpecViolation, ViolationKind, ViolationSeverity
/// };
/// use std::sync::Arc;
///
/// let observer = Arc::new(CollectingObserver::new());
/// let violation = SpecViolation::new(
///     ViolationSeverity::Warning,
///     ViolationKind::FrameSync,
///     "test message",
///     "test.rs:1",
/// );
///
/// // Report to custom observer
/// report_to_observer(Some(&observer), &violation);
/// assert_eq!(observer.len(), 1);
///
/// // Report with no observer (uses TracingObserver)
/// report_to_observer(None::<&Arc<CollectingObserver>>, &violation);
/// ```
#[cold]
#[inline(never)]
pub fn report_to_observer<O: ViolationObserver + ?Sized>(
    observer: Option<&Arc<O>>,
    violation: &SpecViolation,
) {
    match observer {
        Some(obs) => obs.on_violation(violation),
        None => TracingObserver.on_violation(violation),
    }
}

/// Macro for reporting specification violations through a session's observer.
///
/// This macro is similar to [`report_violation!`], but allows specifying an
/// optional observer. If the observer is `None`, it falls back to the default
/// [`TracingObserver`].
///
/// # Syntax
///
/// ```text
/// report_violation_to!(observer, severity, kind, "message");
/// report_violation_to!(observer, severity, kind, "message with {}", format_args);
/// ```
///
/// # Example
///
/// ```
/// use fortress_rollback::{report_violation_to, telemetry::{ViolationSeverity, ViolationKind, CollectingObserver, ViolationObserver}};
/// use std::sync::Arc;
///
/// let observer: Option<Arc<dyn ViolationObserver>> = Some(Arc::new(CollectingObserver::new()));
///
/// report_violation_to!(&observer, ViolationSeverity::Warning, ViolationKind::FrameSync,
///     "frame mismatch: expected={}, actual={}", 10, 15);
/// ```
#[macro_export]
macro_rules! report_violation_to {
    // Basic: observer, severity, kind, message (no format args)
    ($observer:expr, $severity:expr, $kind:expr, $msg:literal) => {{
        let violation = $crate::telemetry::SpecViolation::new(
            $severity,
            $kind,
            $msg,
            concat!(file!(), ":", line!()),
        );
        $crate::telemetry::report_to_observer($observer.as_ref(), &violation);
    }};

    // With format args: observer, severity, kind, format, args...
    ($observer:expr, $severity:expr, $kind:expr, $fmt:literal, $($arg:tt)+) => {{
        let violation = $crate::telemetry::SpecViolation::new(
            $severity,
            $kind,
            format!($fmt, $($arg)+),
            concat!(file!(), ":", line!()),
        );
        $crate::telemetry::report_to_observer($observer.as_ref(), &violation);
    }};
}

// ==========================================
// Runtime Invariant Checking
// ==========================================

/// Result of an invariant check.
///
/// Contains information about what invariant was violated and any
/// additional context for debugging.
///
/// # Serialization
///
/// This type implements `serde::Serialize` for structured JSON output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InvariantViolation {
    /// Name of the type whose invariant was violated.
    pub type_name: &'static str,
    /// Description of the violated invariant.
    pub invariant: String,
    /// Additional diagnostic context.
    pub details: Option<String>,
}

impl InvariantViolation {
    /// Creates a new invariant violation.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn new(type_name: &'static str, invariant: impl Into<String>) -> Self {
        Self {
            type_name,
            invariant: invariant.into(),
            details: None,
        }
    }

    /// Adds additional details to the violation.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Adds checksum mismatch details to the violation.
    ///
    /// This is a specialized method for desync detection that encapsulates
    /// the formatting internally, keeping the call site clean.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_checksum_mismatch(
        mut self,
        frame: Frame,
        player_handle: PlayerHandle,
        local_checksum: u128,
        remote_checksum: u128,
    ) -> Self {
        use std::fmt::Write;
        let mut details = String::new();
        // Ignore write error since we're writing to a String
        let _ = write!(
            details,
            "Desync at frame {} with player {}: local={:#x}, remote={:#x}",
            frame,
            player_handle.as_usize(),
            local_checksum,
            remote_checksum
        );
        self.details = Some(details);
        self
    }

    /// Adds input queue index and nested violation details.
    ///
    /// This method encapsulates the formatting pattern for input queue invariant
    /// violations, providing a consistent API across call sites.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_input_queue_index(mut self, index: usize, nested_violation: String) -> Self {
        use std::fmt::Write;
        let mut details = String::new();
        // Ignore write error since we're writing to a String
        let _ = write!(details, "input_queue[{}]: {}", index, nested_violation);
        self.details = Some(details);
        self
    }

    /// Adds a single field name and value as details.
    ///
    /// This method encapsulates the common pattern of reporting a single
    /// field's value when an invariant is violated.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_field_value(mut self, field: &str, value: impl std::fmt::Display) -> Self {
        use std::fmt::Write;
        let mut details = String::new();
        // Ignore write error since we're writing to a String
        let _ = write!(details, "{}={}", field, value);
        self.details = Some(details);
        self
    }

    /// Adds bounds violation details showing the actual value and valid range.
    ///
    /// This method encapsulates the common pattern of reporting when a value
    /// is outside its expected bounds.
    #[cold]
    #[inline(never)]
    #[must_use]
    pub fn with_bounds_violation(
        mut self,
        name: &str,
        actual: impl std::fmt::Display,
        min: impl std::fmt::Display,
        max: impl std::fmt::Display,
    ) -> Self {
        use std::fmt::Write;
        let mut details = String::new();
        // Ignore write error since we're writing to a String
        let _ = write!(
            details,
            "{}={}, valid_range=[{}, {}]",
            name, actual, min, max
        );
        self.details = Some(details);
        self
    }

    /// Serializes this violation to a JSON string.
    ///
    /// Returns `None` if serialization fails (which should not happen for
    /// well-formed violations).
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// Serializes this violation to a pretty-printed JSON string.
    #[cfg(feature = "json")]
    #[must_use]
    pub fn to_json_pretty(&self) -> Option<String> {
        serde_json::to_string_pretty(self).ok()
    }
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.type_name, self.invariant)?;
        if let Some(details) = &self.details {
            write!(f, " ({})", details)?;
        }
        Ok(())
    }
}

/// Trait for types that maintain internal invariants.
///
/// Types implementing this trait can have their invariants checked at runtime
/// during debug builds or when the `paranoid` feature is enabled.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{InvariantChecker, InvariantViolation};
///
/// struct BoundedCounter {
///     value: u32,
///     max: u32,
/// }
///
/// impl InvariantChecker for BoundedCounter {
///     fn check_invariants(&self) -> Result<(), InvariantViolation> {
///         if self.value > self.max {
///             return Err(InvariantViolation::new(
///                 "BoundedCounter",
///                 "value exceeds maximum",
///             ).with_details(format!("value={}, max={}", self.value, self.max)));
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait InvariantChecker {
    /// Checks that all invariants of this type are satisfied.
    ///
    /// Returns `Ok(())` if all invariants hold, or an `InvariantViolation`
    /// describing the first broken invariant.
    fn check_invariants(&self) -> Result<(), InvariantViolation>;
}

/// Macro for conditionally checking invariants in debug builds.
///
/// This macro expands to an invariant check in debug builds but compiles
/// to nothing in release builds, unless the `paranoid` feature is enabled.
///
/// # Syntax
///
/// ```text
/// debug_check_invariants!(expr);
/// debug_check_invariants!(expr, "context message");
/// ```
///
/// # Example
///
/// ```ignore
/// use fortress_rollback::{debug_check_invariants, telemetry::InvariantChecker};
///
/// fn process<T: InvariantChecker>(item: &T) {
///     // Check invariants at entry in debug builds
///     debug_check_invariants!(item, "before processing");
///
///     // ... do work ...
///
///     // Check invariants at exit in debug builds
///     debug_check_invariants!(item, "after processing");
/// }
/// ```
#[macro_export]
#[cfg(any(debug_assertions, feature = "paranoid"))]
macro_rules! debug_check_invariants {
    ($expr:expr) => {{
        use $crate::telemetry::InvariantChecker as _;
        if let Err(violation) = $expr.check_invariants() {
            $crate::report_violation!(
                $crate::telemetry::ViolationSeverity::Critical,
                $crate::telemetry::ViolationKind::Invariant,
                "{}",
                violation
            );
        }
    }};

    ($expr:expr, $context:expr) => {{
        use $crate::telemetry::InvariantChecker as _;
        if let Err(violation) = $expr.check_invariants() {
            $crate::report_violation!(
                $crate::telemetry::ViolationSeverity::Critical,
                $crate::telemetry::ViolationKind::Invariant,
                "{} [context: {}]",
                violation,
                $context
            );
        }
    }};
}

/// No-op version for release builds without `paranoid` feature.
#[macro_export]
#[cfg(not(any(debug_assertions, feature = "paranoid")))]
macro_rules! debug_check_invariants {
    ($expr:expr) => {{}};
    ($expr:expr, $context:expr) => {{}};
}

/// Macro for checking invariants and panicking if violated (debug only).
///
/// Unlike [`debug_check_invariants!`], this macro will panic if an invariant
/// is violated, making it suitable for critical invariants where continuing
/// would cause undefined behavior.
///
/// # Example
///
/// ```ignore
/// use fortress_rollback::{assert_invariants, telemetry::InvariantChecker};
///
/// fn critical_operation<T: InvariantChecker>(item: &mut T) {
///     assert_invariants!(item); // Panics if invariant broken
///     // ... proceed knowing invariants hold ...
/// }
/// ```
#[macro_export]
#[cfg(any(debug_assertions, feature = "paranoid"))]
macro_rules! assert_invariants {
    ($expr:expr) => {{
        use $crate::telemetry::InvariantChecker as _;
        if let Err(violation) = $expr.check_invariants() {
            panic!("Invariant violation: {}", violation);
        }
    }};

    ($expr:expr, $context:expr) => {{
        use $crate::telemetry::InvariantChecker as _;
        if let Err(violation) = $expr.check_invariants() {
            panic!("Invariant violation ({}): {}", $context, violation);
        }
    }};
}

/// No-op version for release builds without `paranoid` feature.
///
/// # Note
///
/// This macro is a no-op in release builds to avoid the overhead of invariant
/// checking in production. If you need to check invariants in production without
/// panicking, use [`try_check_invariants!`] instead, which returns a `Result`.
#[macro_export]
#[cfg(not(any(debug_assertions, feature = "paranoid")))]
macro_rules! assert_invariants {
    ($expr:expr) => {{}};
    ($expr:expr, $context:expr) => {{}};
}

/// Macro for checking invariants and returning a Result.
///
/// Unlike [`assert_invariants!`], this macro does not panic. Instead, it returns
/// `Ok(())` if invariants hold, or `Err(violation_message)` if they are violated.
/// This is suitable for production code where you want to handle invariant
/// violations gracefully without panicking.
///
/// Unlike [`debug_check_invariants!`], this macro is not gated behind debug_assertions
/// and will always execute in both debug and release builds.
///
/// # Example
///
/// ```ignore
/// use fortress_rollback::{try_check_invariants, telemetry::InvariantChecker};
///
/// fn validate_state<T: InvariantChecker>(item: &T) -> Result<(), String> {
///     try_check_invariants!(item)?;
///     Ok(())
/// }
///
/// fn process_with_validation<T: InvariantChecker>(item: &T) -> Result<(), String> {
///     // Check invariants and handle failure gracefully
///     if let Err(violation) = try_check_invariants!(item) {
///         // Log and recover instead of panicking
///         eprintln!("Warning: invariant violation: {}", violation);
///         return Err(violation);
///     }
///     // Continue processing...
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! try_check_invariants {
    ($expr:expr) => {{
        #[allow(unused_imports)]
        use $crate::telemetry::InvariantChecker as _;
        $expr.check_invariants()
    }};

    ($expr:expr, $context:expr) => {{
        #[allow(unused_imports)]
        use $crate::telemetry::InvariantChecker as _;
        $expr
            .check_invariants()
            .map_err(|violation| format!("{} [context: {}]", violation, $context))
    }};
}

// ==========================================
// Session Telemetry
// ==========================================

/// Observer for session performance telemetry.
///
/// Follows the same pattern as [`ViolationObserver`] — implement this trait
/// and pass it via [`SessionBuilder::with_telemetry()`] to receive structured
/// performance data during a P2P session.
///
/// All methods have default no-op implementations. Override only the events
/// you care about.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::SessionTelemetry;
/// use fortress_rollback::{Frame, PlayerHandle};
/// use fortress_rollback::NetworkStats;
///
/// struct MyTelemetry;
///
/// impl SessionTelemetry for MyTelemetry {
///     fn on_rollback(&self, depth: usize, frame: Frame) {
///         println!("Rollback of {depth} frames at {frame}");
///     }
/// }
/// ```
///
/// [`SessionBuilder::with_telemetry()`]: crate::sessions::builder::SessionBuilder::with_telemetry
#[cfg(feature = "sync-send")]
pub trait SessionTelemetry: Send + Sync {
    /// Called when a rollback occurs.
    ///
    /// `depth` is the number of frames rolled back, `frame` is the frame
    /// that was loaded (the target of the rollback).
    fn on_rollback(&self, depth: usize, frame: Frame) {
        let _ = (depth, frame);
    }

    /// Called when a predicted input turns out to be wrong for a player.
    fn on_prediction_miss(&self, player: PlayerHandle, frame: Frame) {
        let _ = (player, frame);
    }

    /// Called periodically with network statistics for a peer.
    fn on_network_stats(&self, player: PlayerHandle, stats: &NetworkStats) {
        let _ = (player, stats);
    }

    /// Called each time the session advances a frame.
    fn on_frame_advance(&self, frame: Frame) {
        let _ = frame;
    }
}

/// Observer for session performance telemetry.
///
/// Follows the same pattern as [`ViolationObserver`] — implement this trait
/// and pass it via [`SessionBuilder::with_telemetry()`] to receive structured
/// performance data during a P2P session.
///
/// All methods have default no-op implementations. Override only the events
/// you care about.
///
/// [`SessionBuilder::with_telemetry()`]: crate::sessions::builder::SessionBuilder::with_telemetry
#[cfg(not(feature = "sync-send"))]
pub trait SessionTelemetry {
    /// Called when a rollback occurs.
    ///
    /// `depth` is the number of frames rolled back, `frame` is the frame
    /// that was loaded (the target of the rollback).
    fn on_rollback(&self, depth: usize, frame: Frame) {
        let _ = (depth, frame);
    }

    /// Called when a predicted input turns out to be wrong for a player.
    fn on_prediction_miss(&self, player: PlayerHandle, frame: Frame) {
        let _ = (player, frame);
    }

    /// Called periodically with network statistics for a peer.
    fn on_network_stats(&self, player: PlayerHandle, stats: &NetworkStats) {
        let _ = (player, stats);
    }

    /// Called each time the session advances a frame.
    fn on_frame_advance(&self, frame: Frame) {
        let _ = frame;
    }
}

/// Structured telemetry event for collecting and inspecting.
///
/// Each variant corresponds to a method on [`SessionTelemetry`], capturing
/// all the arguments for later inspection.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{TelemetryEvent, CollectingTelemetry, SessionTelemetry};
/// use fortress_rollback::Frame;
///
/// let telemetry = CollectingTelemetry::new();
/// telemetry.on_frame_advance(Frame::new(10));
///
/// let events = telemetry.events();
/// assert_eq!(events.len(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryEvent {
    /// A rollback occurred.
    Rollback {
        /// Number of frames rolled back.
        depth: usize,
        /// The frame that was loaded.
        frame: Frame,
    },
    /// A predicted input was incorrect.
    PredictionMiss {
        /// The player whose prediction was wrong.
        player: PlayerHandle,
        /// The frame at which the misprediction occurred.
        frame: Frame,
    },
    /// Network statistics received for a peer.
    NetworkStatsUpdate {
        /// The player the stats are for.
        player: PlayerHandle,
        /// The network statistics snapshot.
        stats: NetworkStats,
    },
    /// A frame was advanced.
    FrameAdvance {
        /// The frame that was just advanced to.
        frame: Frame,
    },
}

impl std::fmt::Display for TelemetryEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rollback { depth, frame } => {
                write!(f, "Rollback({depth} frames to {frame})")
            },
            Self::PredictionMiss { player, frame } => {
                write!(f, "PredictionMiss(player={player}, frame={frame})")
            },
            Self::NetworkStatsUpdate { player, stats } => {
                write!(f, "NetworkStatsUpdate(player={player}, {stats})")
            },
            Self::FrameAdvance { frame } => write!(f, "FrameAdvance({frame})"),
        }
    }
}

/// Built-in telemetry observer that collects events for testing.
///
/// This observer stores all telemetry events in a thread-safe vector,
/// allowing tests to assert on events that occurred during a session.
///
/// # Example
///
/// ```
/// use fortress_rollback::telemetry::{CollectingTelemetry, SessionTelemetry};
/// use fortress_rollback::Frame;
///
/// let telemetry = CollectingTelemetry::new();
///
/// // Simulate telemetry events
/// telemetry.on_rollback(3, Frame::new(10));
/// telemetry.on_frame_advance(Frame::new(13));
///
/// assert_eq!(telemetry.len(), 2);
/// assert_eq!(telemetry.rollbacks().len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct CollectingTelemetry {
    events: Mutex<Vec<TelemetryEvent>>,
}

impl CollectingTelemetry {
    /// Creates a new collecting telemetry observer with an empty event list.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Returns a copy of all collected events.
    #[cfg(not(loom))]
    #[must_use]
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().clone()
    }

    /// Returns a copy of all collected events (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Returns the number of collected events.
    #[cfg(not(loom))]
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.lock().len()
    }

    /// Returns the number of collected events (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Returns true if no events have been collected.
    #[cfg(not(loom))]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.lock().is_empty()
    }

    /// Returns true if no events have been collected (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    /// Returns all rollback events.
    #[cfg(not(loom))]
    #[must_use]
    pub fn rollbacks(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::Rollback { .. }))
            .copied()
            .collect()
    }

    /// Returns all rollback events (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn rollbacks(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::Rollback { .. }))
            .copied()
            .collect()
    }

    /// Returns all prediction miss events.
    #[cfg(not(loom))]
    #[must_use]
    pub fn prediction_misses(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::PredictionMiss { .. }))
            .copied()
            .collect()
    }

    /// Returns all prediction miss events (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn prediction_misses(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::PredictionMiss { .. }))
            .copied()
            .collect()
    }

    /// Returns all network stats update events.
    #[cfg(not(loom))]
    #[must_use]
    pub fn network_stats_updates(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::NetworkStatsUpdate { .. }))
            .copied()
            .collect()
    }

    /// Returns all network stats update events (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn network_stats_updates(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::NetworkStatsUpdate { .. }))
            .copied()
            .collect()
    }

    /// Returns all frame advance events.
    #[cfg(not(loom))]
    #[must_use]
    pub fn frame_advances(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::FrameAdvance { .. }))
            .copied()
            .collect()
    }

    /// Returns all frame advance events (loom version).
    #[cfg(loom)]
    #[must_use]
    pub fn frame_advances(&self) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| matches!(e, TelemetryEvent::FrameAdvance { .. }))
            .copied()
            .collect()
    }

    /// Clears all collected events.
    #[cfg(not(loom))]
    pub fn clear(&self) {
        self.events.lock().clear();
    }

    /// Clears all collected events (loom version).
    #[cfg(loom)]
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[cfg(not(loom))]
impl SessionTelemetry for CollectingTelemetry {
    fn on_rollback(&self, depth: usize, frame: Frame) {
        self.events
            .lock()
            .push(TelemetryEvent::Rollback { depth, frame });
    }

    fn on_prediction_miss(&self, player: PlayerHandle, frame: Frame) {
        self.events
            .lock()
            .push(TelemetryEvent::PredictionMiss { player, frame });
    }

    fn on_network_stats(&self, player: PlayerHandle, stats: &NetworkStats) {
        self.events.lock().push(TelemetryEvent::NetworkStatsUpdate {
            player,
            stats: *stats,
        });
    }

    fn on_frame_advance(&self, frame: Frame) {
        self.events
            .lock()
            .push(TelemetryEvent::FrameAdvance { frame });
    }
}

#[cfg(loom)]
impl SessionTelemetry for CollectingTelemetry {
    fn on_rollback(&self, depth: usize, frame: Frame) {
        self.events
            .lock()
            .unwrap()
            .push(TelemetryEvent::Rollback { depth, frame });
    }

    fn on_prediction_miss(&self, player: PlayerHandle, frame: Frame) {
        self.events
            .lock()
            .unwrap()
            .push(TelemetryEvent::PredictionMiss { player, frame });
    }

    fn on_network_stats(&self, player: PlayerHandle, stats: &NetworkStats) {
        self.events
            .lock()
            .unwrap()
            .push(TelemetryEvent::NetworkStatsUpdate {
                player,
                stats: *stats,
            });
    }

    fn on_frame_advance(&self, frame: Frame) {
        self.events
            .lock()
            .unwrap()
            .push(TelemetryEvent::FrameAdvance { frame });
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

    #[test]
    fn test_violation_severity_ordering() {
        assert!(ViolationSeverity::Warning < ViolationSeverity::Error);
        assert!(ViolationSeverity::Error < ViolationSeverity::Critical);
    }

    #[test]
    fn test_violation_severity_as_str() {
        assert_eq!(ViolationSeverity::Warning.as_str(), "warning");
        assert_eq!(ViolationSeverity::Error.as_str(), "error");
        assert_eq!(ViolationSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_violation_kind_as_str() {
        assert_eq!(ViolationKind::FrameSync.as_str(), "frame_sync");
        assert_eq!(ViolationKind::InputQueue.as_str(), "input_queue");
        assert_eq!(ViolationKind::StateManagement.as_str(), "state_management");
        assert_eq!(ViolationKind::NetworkProtocol.as_str(), "network_protocol");
        assert_eq!(
            ViolationKind::ChecksumMismatch.as_str(),
            "checksum_mismatch"
        );
        assert_eq!(ViolationKind::Configuration.as_str(), "configuration");
        assert_eq!(ViolationKind::InternalError.as_str(), "internal_error");
    }

    #[test]
    fn test_spec_violation_builder() {
        let violation = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test message",
            "test.rs:42",
        )
        .with_frame(Frame::new(100))
        .with_context("expected", "10")
        .with_context("actual", "15");

        assert_eq!(violation.severity, ViolationSeverity::Warning);
        assert_eq!(violation.kind, ViolationKind::FrameSync);
        assert_eq!(violation.message, "test message");
        assert_eq!(violation.location, "test.rs:42");
        assert_eq!(violation.frame, Some(Frame::new(100)));
        assert_eq!(violation.context.get("expected"), Some(&"10".to_string()));
        assert_eq!(violation.context.get("actual"), Some(&"15".to_string()));
    }

    #[test]
    fn test_spec_violation_display() {
        let violation = SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "missing input",
            "test.rs:10",
        )
        .with_frame(Frame::new(50));

        let display = violation.to_string();
        assert!(display.contains("error"));
        assert!(display.contains("input_queue"));
        assert!(display.contains("missing input"));
        assert!(display.contains("test.rs:10"));
        assert!(display.contains("frame=50"));
    }

    #[test]
    fn test_collecting_observer() {
        let observer = CollectingObserver::new();
        assert!(observer.is_empty());
        assert_eq!(observer.len(), 0);

        let violation1 = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "first",
            "test.rs:1",
        );
        let violation2 = SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "second",
            "test.rs:2",
        );

        observer.on_violation(&violation1);
        observer.on_violation(&violation2);

        assert!(!observer.is_empty());
        assert_eq!(observer.len(), 2);
        assert!(observer.has_violation(ViolationKind::FrameSync));
        assert!(observer.has_violation(ViolationKind::InputQueue));
        assert!(!observer.has_violation(ViolationKind::NetworkProtocol));

        assert!(observer.has_severity(ViolationSeverity::Warning));
        assert!(observer.has_severity(ViolationSeverity::Error));
        assert!(!observer.has_severity(ViolationSeverity::Critical));
    }

    #[test]
    fn test_collecting_observer_filter_by_kind() {
        let observer = CollectingObserver::new();

        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "frame1",
            "test.rs:1",
        ));
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "input1",
            "test.rs:2",
        ));
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "frame2",
            "test.rs:3",
        ));

        let frame_violations = observer.violations_of_kind(ViolationKind::FrameSync);
        assert_eq!(frame_violations.len(), 2);
        assert!(frame_violations
            .iter()
            .all(|v| v.kind == ViolationKind::FrameSync));
    }

    #[test]
    fn test_collecting_observer_filter_by_severity() {
        let observer = CollectingObserver::new();

        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "warning",
            "test.rs:1",
        ));
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "error",
            "test.rs:2",
        ));
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Critical,
            ViolationKind::InternalError,
            "critical",
            "test.rs:3",
        ));

        let errors_and_above = observer.violations_at_severity(ViolationSeverity::Error);
        assert_eq!(errors_and_above.len(), 2);
        assert!(errors_and_above
            .iter()
            .all(|v| v.severity >= ViolationSeverity::Error));
    }

    #[test]
    fn test_collecting_observer_clear() {
        let observer = CollectingObserver::new();

        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test",
            "test.rs:1",
        ));
        assert!(!observer.is_empty());

        observer.clear();
        assert!(observer.is_empty());
    }

    // ==========================================
    // CollectingObserver Concurrent Access Tests
    // ==========================================

    /// Tests that CollectingObserver handles concurrent writes correctly.
    /// With parking_lot::Mutex, this should never deadlock or panic.
    #[test]
    fn test_collecting_observer_concurrent_writes() {
        use std::thread;

        let observer = Arc::new(CollectingObserver::new());
        let mut handles = vec![];

        // Spawn 10 threads, each adding 100 violations
        for thread_id in 0..10 {
            let observer_clone = observer.clone();
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let violation = SpecViolation::new(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        format!("thread {} violation {}", thread_id, i),
                        "concurrent_test.rs:1",
                    );
                    observer_clone.on_violation(&violation);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Should have exactly 1000 violations (10 threads * 100 violations)
        assert_eq!(observer.len(), 1000);
    }

    /// Tests that CollectingObserver handles concurrent reads correctly.
    #[test]
    fn test_collecting_observer_concurrent_reads() {
        use std::thread;

        let observer = Arc::new(CollectingObserver::new());

        // Add some violations first
        for i in 0..100 {
            observer.on_violation(&SpecViolation::new(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                format!("violation {}", i),
                "test.rs:1",
            ));
        }

        let mut handles = vec![];

        // Spawn 10 threads, each reading violations multiple times
        for _ in 0..10 {
            let observer_clone = observer.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let len = observer_clone.len();
                    assert_eq!(len, 100);

                    let is_empty = observer_clone.is_empty();
                    assert!(!is_empty);

                    let has_frame_sync = observer_clone.has_violation(ViolationKind::FrameSync);
                    assert!(has_frame_sync);

                    let violations = observer_clone.violations();
                    assert_eq!(violations.len(), 100);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }
    }

    /// Tests that CollectingObserver handles concurrent reads and writes.
    #[test]
    fn test_collecting_observer_concurrent_read_write() {
        use std::thread;

        let observer = Arc::new(CollectingObserver::new());
        let mut handles = vec![];

        // Spawn writer threads
        for thread_id in 0..5 {
            let observer_clone = observer.clone();
            let handle = thread::spawn(move || {
                for i in 0..50 {
                    let violation = SpecViolation::new(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        format!("write thread {} violation {}", thread_id, i),
                        "concurrent_rw_test.rs:1",
                    );
                    observer_clone.on_violation(&violation);
                }
            });
            handles.push(handle);
        }

        // Spawn reader threads
        for _ in 0..5 {
            let observer_clone = observer.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    // These operations should not panic even while other threads are writing
                    let _ = observer_clone.len();
                    let _ = observer_clone.is_empty();
                    let _ = observer_clone.has_violation(ViolationKind::FrameSync);
                    let _ = observer_clone.violations();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Should have exactly 250 violations (5 write threads * 50 violations)
        assert_eq!(observer.len(), 250);
    }

    /// Tests that parking_lot::Mutex doesn't poison on panic (unlike std::sync::Mutex).
    /// This is a key property that ensures the observer remains usable even if a
    /// thread panics while holding the lock.
    #[test]
    fn test_collecting_observer_no_poison_on_panic() {
        use std::thread;

        let observer = Arc::new(CollectingObserver::new());

        // Add a violation before the panic
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "before panic",
            "test.rs:1",
        ));

        // Spawn a thread that will panic while using the observer
        // (though parking_lot's operations are so fast it's hard to panic mid-operation)
        let observer_clone = observer.clone();
        let handle = thread::spawn(move || {
            // Use the observer
            let _ = observer_clone.len();
            // Panic
            panic!("intentional panic for testing");
        });

        // Wait for the thread (it should panic)
        let result = handle.join();
        assert!(result.is_err(), "Thread should have panicked");

        // The observer should still be usable (not poisoned)
        // With std::sync::Mutex, this would panic with "PoisonError"
        // With parking_lot::Mutex, this works fine
        assert_eq!(observer.len(), 1);
        assert!(!observer.is_empty());

        // Should still be able to add violations
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "after panic",
            "test.rs:2",
        ));
        assert_eq!(observer.len(), 2);
    }

    #[test]
    fn test_composite_observer() {
        let collector1 = Arc::new(CollectingObserver::new());
        let collector2 = Arc::new(CollectingObserver::new());

        let mut composite = CompositeObserver::new();
        composite.add(collector1.clone());
        composite.add(collector2.clone());

        let violation = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test",
            "test.rs:1",
        );

        composite.on_violation(&violation);

        assert_eq!(collector1.len(), 1);
        assert_eq!(collector2.len(), 1);
    }

    #[test]
    fn test_report_violation_macro_basic() {
        // Just ensure it compiles and doesn't panic
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test message"
        );
    }

    #[test]
    fn test_report_violation_macro_with_format() {
        let expected = 10;
        let actual = 15;
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "mismatch: expected={}, actual={}",
            expected,
            actual
        );
    }

    #[test]
    fn test_report_violation_macro_with_observer() {
        let observer = CollectingObserver::new();
        // Use the observer directly instead of the macro
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "test with observer",
            "test.rs:1",
        ));
        assert_eq!(observer.len(), 1);
    }

    #[test]
    fn test_assert_no_violations_macro() {
        let observer = CollectingObserver::new();
        assert_no_violations!(observer);
    }

    #[test]
    fn test_assert_violation_macro() {
        let observer = CollectingObserver::new();
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test",
            "test.rs:1",
        ));
        assert_violation!(observer, ViolationKind::FrameSync);
    }

    #[test]
    fn test_tracing_observer_creation() {
        let observer = TracingObserver::new();
        // Just ensure it doesn't panic when called
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test",
            "test.rs:1",
        ));
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::InputQueue,
            "test",
            "test.rs:2",
        ));
        observer.on_violation(&SpecViolation::new(
            ViolationSeverity::Critical,
            ViolationKind::InternalError,
            "test",
            "test.rs:3",
        ));
    }

    #[test]
    fn test_report_to_observer_with_some() {
        let observer = Arc::new(CollectingObserver::new());
        let violation = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test message",
            "test.rs:1",
        );

        report_to_observer(Some(&observer), &violation);
        assert_eq!(observer.len(), 1);
        assert!(observer.has_violation(ViolationKind::FrameSync));
    }

    #[test]
    fn test_report_to_observer_with_none() {
        // Just ensure it doesn't panic when observer is None
        let violation = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test message",
            "test.rs:1",
        );

        let no_observer: Option<&Arc<CollectingObserver>> = None;
        report_to_observer(no_observer, &violation);
        // If we get here without panic, test passes
    }

    #[test]
    fn test_report_violation_to_macro_basic() {
        let observer: Option<Arc<dyn ViolationObserver>> =
            Some(Arc::new(CollectingObserver::new()));
        report_violation_to!(
            &observer,
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test message"
        );
        // Just ensure it compiles and doesn't panic
    }

    #[test]
    fn test_report_violation_to_macro_with_format() {
        let observer: Option<Arc<dyn ViolationObserver>> =
            Some(Arc::new(CollectingObserver::new()));
        let expected = 10;
        let actual = 15;
        report_violation_to!(
            &observer,
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "mismatch: expected={}, actual={}",
            expected,
            actual
        );
        // Just ensure it compiles and doesn't panic
    }

    #[test]
    fn test_report_violation_to_macro_with_none() {
        let observer: Option<Arc<dyn ViolationObserver>> = None;
        report_violation_to!(
            &observer,
            ViolationSeverity::Warning,
            ViolationKind::FrameSync,
            "test message"
        );
        // Falls back to TracingObserver, shouldn't panic
    }

    // ==========================================
    // Synchronization Violation Tests
    // ==========================================

    #[test]
    fn test_violation_kind_synchronization_as_str() {
        assert_eq!(ViolationKind::Synchronization.as_str(), "synchronization");
    }

    #[test]
    fn test_synchronization_violation_can_be_created() {
        let observer = Arc::new(CollectingObserver::new());
        let observer_ref: Option<Arc<dyn ViolationObserver>> = Some(observer.clone());

        report_violation_to!(
            &observer_ref,
            ViolationSeverity::Warning,
            ViolationKind::Synchronization,
            "Excessive sync retries: {} requests sent (threshold: {}). Possible high packet loss.",
            15,
            10
        );

        let violations = observer.violations();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, ViolationKind::Synchronization);
        assert_eq!(violations[0].severity, ViolationSeverity::Warning);
        assert!(violations[0].message.contains("Excessive sync retries"));
        assert!(violations[0].message.contains("15"));
        assert!(violations[0].message.contains("10"));
    }

    #[test]
    fn test_synchronization_violation_ordering() {
        // Verify Synchronization is ordered correctly with other variants
        assert!(ViolationKind::FrameSync < ViolationKind::Synchronization);
        assert!(ViolationKind::Invariant < ViolationKind::Synchronization);
    }

    #[test]
    fn test_synchronization_duration_warning() {
        let observer = Arc::new(CollectingObserver::new());
        let observer_ref: Option<Arc<dyn ViolationObserver>> = Some(observer.clone());

        report_violation_to!(
            &observer_ref,
            ViolationSeverity::Warning,
            ViolationKind::Synchronization,
            "Sync duration exceeded threshold: {}ms (threshold: {}ms). Network latency may be high.",
            5000,
            3000
        );

        let violations = observer.violations();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("5000ms"));
        assert!(violations[0].message.contains("3000ms"));
    }

    // ==========================================
    // Invariant Checker Tests
    // ==========================================

    #[test]
    fn test_violation_kind_invariant_as_str() {
        assert_eq!(ViolationKind::Invariant.as_str(), "invariant");
    }

    #[test]
    fn test_invariant_violation_new() {
        let violation = InvariantViolation::new("TestType", "value out of range");

        assert_eq!(violation.type_name, "TestType");
        assert_eq!(violation.invariant, "value out of range");
        assert!(violation.details.is_none());
    }

    #[test]
    fn test_invariant_violation_with_details() {
        let violation = InvariantViolation::new("Counter", "negative value")
            .with_details("value=-5, expected>=0");

        assert_eq!(violation.type_name, "Counter");
        assert_eq!(violation.invariant, "negative value");
        assert_eq!(violation.details, Some("value=-5, expected>=0".to_string()));
    }

    #[test]
    fn test_invariant_violation_display_without_details() {
        let violation = InvariantViolation::new("Queue", "length exceeds capacity");

        let display = violation.to_string();
        assert!(display.contains("Queue"));
        assert!(display.contains("length exceeds capacity"));
    }

    #[test]
    fn test_invariant_violation_display_with_details() {
        let violation =
            InvariantViolation::new("Buffer", "overflow").with_details("size=200, max=128");

        let display = violation.to_string();
        assert!(display.contains("Buffer"));
        assert!(display.contains("overflow"));
        assert!(display.contains("size=200, max=128"));
    }

    #[test]
    fn test_invariant_violation_with_checksum_mismatch() {
        let violation = InvariantViolation::new("P2PSession", "checksum mismatch")
            .with_checksum_mismatch(Frame::new(42), PlayerHandle(1), 0xDEAD_BEEF, 0xCAFE_BABE);

        assert_eq!(violation.type_name, "P2PSession");
        assert_eq!(violation.invariant, "checksum mismatch");

        let details = violation.details.as_ref().expect("details should be set");
        assert!(details.contains("Desync at frame 42"));
        assert!(details.contains("player 1"));
        assert!(details.contains("0xdeadbeef"));
        assert!(details.contains("0xcafebabe"));
    }

    #[test]
    fn test_invariant_violation_with_checksum_mismatch_display() {
        let violation = InvariantViolation::new("Session", "desync detected")
            .with_checksum_mismatch(Frame::new(100), PlayerHandle(2), 0x1234, 0x5678);

        let display = violation.to_string();
        assert!(display.contains("Session"));
        assert!(display.contains("desync detected"));
        assert!(display.contains("Desync at frame 100"));
        assert!(display.contains("player 2"));
    }

    // Test implementation of InvariantChecker for testing
    struct TestCheckerOk;

    impl InvariantChecker for TestCheckerOk {
        fn check_invariants(&self) -> Result<(), InvariantViolation> {
            Ok(())
        }
    }

    struct TestCheckerFail {
        message: &'static str,
    }

    impl InvariantChecker for TestCheckerFail {
        fn check_invariants(&self) -> Result<(), InvariantViolation> {
            Err(InvariantViolation::new("TestCheckerFail", self.message))
        }
    }

    #[test]
    fn test_invariant_checker_trait_ok() {
        let checker = TestCheckerOk;
        checker.check_invariants().unwrap();
    }

    #[test]
    fn test_invariant_checker_trait_fail() {
        let checker = TestCheckerFail {
            message: "test failure",
        };
        let result = checker.check_invariants();
        assert!(result.is_err());
        let violation = result.unwrap_err();
        assert_eq!(violation.type_name, "TestCheckerFail");
        assert_eq!(violation.invariant, "test failure");
    }

    #[test]
    fn test_debug_check_invariants_macro_ok() {
        let checker = TestCheckerOk;
        // Should not report any violations
        debug_check_invariants!(checker);
        debug_check_invariants!(checker, "with context");
    }

    #[test]
    fn test_debug_check_invariants_macro_fail() {
        let checker = TestCheckerFail {
            message: "macro test",
        };
        // Should report a violation via tracing (doesn't panic)
        debug_check_invariants!(checker);
        debug_check_invariants!(checker, "with context");
    }

    #[test]
    fn test_assert_invariants_macro_ok() {
        let checker = TestCheckerOk;
        // Should not panic
        assert_invariants!(checker);
        assert_invariants!(checker, "with context");
    }

    // Note: These tests are gated to debug_assertions because assert_invariants!
    // is a no-op in release mode for performance reasons.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Invariant violation")]
    fn test_assert_invariants_macro_fail() {
        let checker = TestCheckerFail {
            message: "panic test",
        };
        assert_invariants!(checker);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "test context")]
    fn test_assert_invariants_macro_fail_with_context() {
        let checker = TestCheckerFail {
            message: "panic test",
        };
        assert_invariants!(checker, "test context");
    }

    // ==========================================
    // try_check_invariants! Macro Tests
    // ==========================================

    #[test]
    fn test_try_check_invariants_macro_ok() {
        let checker = TestCheckerOk;
        let result = try_check_invariants!(checker);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_check_invariants_macro_ok_with_context() {
        let checker = TestCheckerOk;
        let result = try_check_invariants!(checker, "test context");
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_check_invariants_macro_fail() {
        let checker = TestCheckerFail {
            message: "invariant failed",
        };
        let result = try_check_invariants!(checker);
        assert!(result.is_err());
        assert!(result.unwrap_err().invariant.contains("invariant failed"));
    }

    #[test]
    fn test_try_check_invariants_macro_fail_with_context() {
        let checker = TestCheckerFail {
            message: "invariant failed",
        };
        let result = try_check_invariants!(checker, "my context");
        assert!(result.is_err());
        let err = result.unwrap_err();
        // With context, the error is formatted as a String
        assert!(err.contains("invariant failed"));
        assert!(err.contains("my context"));
    }

    #[test]
    fn test_try_check_invariants_can_use_question_mark() {
        fn check_it<T: InvariantChecker>(item: &T) -> Result<(), InvariantViolation> {
            try_check_invariants!(item)?;
            Ok(())
        }

        let checker_ok = TestCheckerOk;
        assert!(check_it(&checker_ok).is_ok());

        let checker_fail = TestCheckerFail { message: "failed" };
        assert!(check_it(&checker_fail).is_err());
    }

    // ==========================================
    // JSON Serialization Tests
    // ==========================================

    #[cfg(feature = "json")]
    mod json_tests {
        use super::*;

        #[test]
        fn test_violation_severity_serialization() {
            assert_eq!(
                serde_json::to_string(&ViolationSeverity::Warning).unwrap(),
                r#""warning""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationSeverity::Error).unwrap(),
                r#""error""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationSeverity::Critical).unwrap(),
                r#""critical""#
            );
        }

        #[test]
        fn test_violation_kind_serialization() {
            assert_eq!(
                serde_json::to_string(&ViolationKind::FrameSync).unwrap(),
                r#""frame_sync""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::InputQueue).unwrap(),
                r#""input_queue""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::StateManagement).unwrap(),
                r#""state_management""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::NetworkProtocol).unwrap(),
                r#""network_protocol""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::ChecksumMismatch).unwrap(),
                r#""checksum_mismatch""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::Synchronization).unwrap(),
                r#""synchronization""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::Invariant).unwrap(),
                r#""invariant""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::InternalError).unwrap(),
                r#""internal_error""#
            );
            assert_eq!(
                serde_json::to_string(&ViolationKind::Configuration).unwrap(),
                r#""configuration""#
            );
        }

        #[test]
        fn test_spec_violation_json_serialization_basic() {
            let violation = SpecViolation::new(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "test message",
                "test.rs:42",
            );

            let json = violation.to_json().unwrap();
            assert!(json.contains(r#""severity":"warning""#));
            assert!(json.contains(r#""kind":"frame_sync""#));
            assert!(json.contains(r#""message":"test message""#));
            assert!(json.contains(r#""location":"test.rs:42""#));
            // frame should be null when not set
            assert!(json.contains(r#""frame":null"#));
        }

        #[test]
        fn test_spec_violation_json_serialization_with_frame() {
            let violation = SpecViolation::new(
                ViolationSeverity::Error,
                ViolationKind::InputQueue,
                "missing input",
                "queue.rs:100",
            )
            .with_frame(Frame::new(42));

            let json = violation.to_json().unwrap();
            assert!(json.contains(r#""frame":42"#));
            // Verify it's a number, not a string
            assert!(!json.contains(r#""frame":"42""#));
        }

        #[test]
        fn test_spec_violation_json_serialization_with_null_frame() {
            let violation = SpecViolation::new(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "test",
                "test.rs:1",
            )
            .with_frame(Frame::NULL);

            let json = violation.to_json().unwrap();
            // NULL frame should serialize as null, not as -1
            assert!(json.contains(r#""frame":null"#));
        }

        #[test]
        fn test_spec_violation_json_serialization_with_context() {
            let violation = SpecViolation::new(
                ViolationSeverity::Critical,
                ViolationKind::ChecksumMismatch,
                "checksum mismatch",
                "sync.rs:50",
            )
            .with_frame(Frame::new(100))
            .with_context("expected", "0x12345678")
            .with_context("actual", "0x87654321");

            let json = violation.to_json().unwrap();
            assert!(json.contains(r#""severity":"critical""#));
            assert!(json.contains(r#""frame":100"#));
            // Context should be serialized as an object
            assert!(json.contains(r#""expected":"0x12345678""#));
            assert!(json.contains(r#""actual":"0x87654321""#));
        }

        #[test]
        fn test_spec_violation_to_json_pretty() {
            let violation = SpecViolation::new(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "test",
                "test.rs:1",
            );

            let json_pretty = violation.to_json_pretty().unwrap();
            // Pretty JSON should have newlines
            assert!(json_pretty.contains('\n'));
            assert!(json_pretty.contains("  ")); // indentation
        }

        #[test]
        fn test_invariant_violation_json_serialization() {
            let violation = InvariantViolation::new("TestType", "value out of range")
                .with_details("value=-5, expected>=0");

            let json = violation.to_json().unwrap();
            assert!(json.contains(r#""type_name":"TestType""#));
            assert!(json.contains(r#""invariant":"value out of range""#));
            assert!(json.contains(r#""details":"value=-5, expected>=0""#));
        }

        #[test]
        fn test_invariant_violation_json_without_details() {
            let violation = InvariantViolation::new("Counter", "overflow");

            let json = violation.to_json().unwrap();
            assert!(json.contains(r#""type_name":"Counter""#));
            assert!(json.contains(r#""invariant":"overflow""#));
            assert!(json.contains(r#""details":null"#));
        }

        #[test]
        fn test_spec_violation_roundtrip_parseable() {
            // Verify that the JSON output can be parsed back by a JSON parser
            let violation = SpecViolation::new(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "test message with \"quotes\" and special chars",
                "test.rs:1",
            )
            .with_frame(Frame::new(42))
            .with_context("key", "value with spaces");

            let json = violation.to_json().unwrap();

            // Parse it back as a generic JSON value
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed["severity"], "warning");
            assert_eq!(parsed["kind"], "frame_sync");
            assert_eq!(parsed["frame"], 42);
            assert_eq!(parsed["context"]["key"], "value with spaces");
        }
    } // end of json_tests module

    #[test]
    fn test_tracing_observer_format_frame() {
        assert_eq!(TracingObserver::format_frame(None), "null");
        assert_eq!(TracingObserver::format_frame(Some(Frame::NULL)), "null");
        assert_eq!(TracingObserver::format_frame(Some(Frame::new(0))), "0");
        assert_eq!(TracingObserver::format_frame(Some(Frame::new(100))), "100");
        assert_eq!(TracingObserver::format_frame(Some(Frame::new(-5))), "-5");
    }

    // ==========================================
    // SessionTelemetry Tests
    // ==========================================

    #[test]
    fn collecting_telemetry_records_events() {
        let telemetry = CollectingTelemetry::new();

        telemetry.on_rollback(3, Frame::new(10));
        telemetry.on_prediction_miss(PlayerHandle::new(1), Frame::new(5));
        telemetry.on_frame_advance(Frame::new(13));
        telemetry.on_network_stats(PlayerHandle::new(0), &NetworkStats::default());

        let events = telemetry.events();
        assert_eq!(events.len(), 4);
        assert!(matches!(
            events[0],
            TelemetryEvent::Rollback {
                depth: 3,
                frame
            } if frame == Frame::new(10)
        ));
        assert!(matches!(
            events[1],
            TelemetryEvent::PredictionMiss { player, frame }
            if player == PlayerHandle::new(1) && frame == Frame::new(5)
        ));
        assert!(
            matches!(events[2], TelemetryEvent::FrameAdvance { frame } if frame == Frame::new(13))
        );
        assert!(matches!(
            events[3],
            TelemetryEvent::NetworkStatsUpdate { player, .. }
            if player == PlayerHandle::new(0)
        ));
    }

    #[test]
    fn collecting_telemetry_rollbacks_filter() {
        let telemetry = CollectingTelemetry::new();

        telemetry.on_rollback(2, Frame::new(5));
        telemetry.on_frame_advance(Frame::new(7));
        telemetry.on_rollback(1, Frame::new(8));
        telemetry.on_prediction_miss(PlayerHandle::new(0), Frame::new(5));

        let rollbacks = telemetry.rollbacks();
        assert_eq!(rollbacks.len(), 2);
        assert!(matches!(
            rollbacks[0],
            TelemetryEvent::Rollback { depth: 2, .. }
        ));
        assert!(matches!(
            rollbacks[1],
            TelemetryEvent::Rollback { depth: 1, .. }
        ));
    }

    #[test]
    fn collecting_telemetry_prediction_misses_filter() {
        let telemetry = CollectingTelemetry::new();

        telemetry.on_rollback(2, Frame::new(5));
        telemetry.on_prediction_miss(PlayerHandle::new(0), Frame::new(3));
        telemetry.on_frame_advance(Frame::new(7));
        telemetry.on_prediction_miss(PlayerHandle::new(1), Frame::new(4));

        let misses = telemetry.prediction_misses();
        assert_eq!(misses.len(), 2);
        assert!(matches!(misses[0], TelemetryEvent::PredictionMiss { .. }));
        assert!(matches!(misses[1], TelemetryEvent::PredictionMiss { .. }));
    }

    #[test]
    fn collecting_telemetry_clear_removes_all() {
        let telemetry = CollectingTelemetry::new();

        telemetry.on_rollback(1, Frame::new(1));
        telemetry.on_frame_advance(Frame::new(2));
        assert_eq!(telemetry.len(), 2);
        assert!(!telemetry.is_empty());

        telemetry.clear();
        assert_eq!(telemetry.len(), 0);
        assert!(telemetry.is_empty());
        assert!(telemetry.events().is_empty());
    }

    #[test]
    fn session_telemetry_default_methods_are_noop() {
        // A blank implementation should compile and not panic
        struct NoOpTelemetry;
        impl SessionTelemetry for NoOpTelemetry {}

        let t = NoOpTelemetry;
        t.on_rollback(5, Frame::new(10));
        t.on_prediction_miss(PlayerHandle::new(0), Frame::new(3));
        t.on_network_stats(PlayerHandle::new(1), &NetworkStats::default());
        t.on_frame_advance(Frame::new(42));
        // If we get here without panicking, the test passes
    }

    #[test]
    fn collecting_telemetry_new_is_empty() {
        let telemetry = CollectingTelemetry::new();
        assert!(telemetry.is_empty());
        assert_eq!(telemetry.len(), 0);
        assert!(telemetry.events().is_empty());
        assert!(telemetry.rollbacks().is_empty());
        assert!(telemetry.prediction_misses().is_empty());
    }

    #[test]
    fn telemetry_event_display_all_variants() {
        let rollback = TelemetryEvent::Rollback {
            depth: 5,
            frame: Frame::new(100),
        };
        assert!(format!("{rollback}").contains('5'));
        assert!(format!("{rollback}").contains("100"));

        let miss = TelemetryEvent::PredictionMiss {
            player: PlayerHandle::new(1),
            frame: Frame::new(42),
        };
        let miss_str = format!("{miss}");
        assert!(miss_str.contains("42"));

        let advance = TelemetryEvent::FrameAdvance {
            frame: Frame::new(200),
        };
        assert!(format!("{advance}").contains("200"));

        let stats = TelemetryEvent::NetworkStatsUpdate {
            player: PlayerHandle::new(0),
            stats: NetworkStats::default(),
        };
        let stats_str = format!("{stats}");
        assert!(stats_str.contains("NetworkStatsUpdate"));
    }

    #[test]
    fn collecting_telemetry_network_stats_and_frame_advance_filters() {
        let telemetry = CollectingTelemetry::new();

        // Add mixed events
        telemetry.on_rollback(3, Frame::new(10));
        telemetry.on_frame_advance(Frame::new(11));
        telemetry.on_network_stats(PlayerHandle::new(0), &NetworkStats::default());
        telemetry.on_frame_advance(Frame::new(12));
        telemetry.on_prediction_miss(PlayerHandle::new(1), Frame::new(5));
        telemetry.on_network_stats(PlayerHandle::new(1), &NetworkStats::default());

        assert_eq!(telemetry.len(), 6);
        assert_eq!(telemetry.network_stats_updates().len(), 2);
        assert_eq!(telemetry.frame_advances().len(), 2);
        assert_eq!(telemetry.rollbacks().len(), 1);
        assert_eq!(telemetry.prediction_misses().len(), 1);
    }
}
