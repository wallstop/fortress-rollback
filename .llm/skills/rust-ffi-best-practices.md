# Rust FFI and Hybrid Applications Best Practices

> **A guide to building hybrid Rust/C/C++ applications with safe, maintainable foreign function interfaces.**

## Overview

Rust's C-compatible ABI enables seamless integration with existing C/C++ codebases, making it ideal for gradual "oxidation" of legacy systems. This guide covers best practices for FFI design, memory safety across language boundaries, and build system integration.

**Related Skills:**

- [cross-platform-rust.md](cross-platform-rust.md) — Cross-platform development patterns
- [defensive-programming.md](defensive-programming.md) — Error handling patterns
- [no-std-guide.md](no-std-guide.md) — `no_std` for minimal runtime

---

## When to Use FFI

### Good Candidates for Rust Integration

| Criterion | Description |
|-----------|-------------|
| **Problematic code** | Source of bugs, crashes, or security vulnerabilities |
| **Self-contained** | Well-isolated with minimal external dependencies |
| **Clean interface** | Well-documented entry points with narrow surface area |
| **Idiomatically similar** | Limited use of templates, macros, or deep OOP hierarchies |

**Best starting points:**

- Leaf libraries at the edge of architecture
- Media/image file handling (common memory safety issues)
- Parsers and input handlers
- Modules processing untrusted external data

### When NOT to Convert

- **Battle-tested algorithms** — Stable cryptography, signal processing with limited I/O
- **Code without memory allocation** — Already avoids 70% of C++ bugs
- **Simple driver implementations** — Low complexity, low risk
- **Long-stable code** — Unchanged for years, well-exercised

### Problematic Conversions

| Area | Challenge |
|------|-----------|
| Complex OOP hierarchies | Multiple inheritance difficult in Rust |
| Qt applications | moc preprocessing, signals/slots patterns |
| Mixed string types | C-style, std::string, UTF variants, QString |
| Template-heavy code | No direct Rust equivalent |
| Preprocessor-dependent code | Macros don't translate cleanly |

---

## FFI Tool Ecosystem

### Tool Comparison

| Tool | Direction | Purpose | Best For |
|------|-----------|---------|----------|
| **`extern "C"`** | Both | Rust's built-in FFI | Simple C functions |
| **[bindgen](https://rust-lang.github.io/rust-bindgen/)** | C/C++ → Rust | Generate Rust bindings from headers | Consuming C/C++ libraries |
| **[cbindgen](https://github.com/mozilla/cbindgen)** | Rust → C/C++ | Generate C headers from Rust | Exposing Rust to C/C++ |
| **[CXX](https://cxx.rs/)** | Bidirectional | Safe interop with type restrictions | New APIs with safety guarantees |
| **[AutoCXX](https://github.com/nicorboe/autocxx)** | C++ → Rust | CXX-like but uses existing headers | Existing C++ APIs |
| **[CXX-Qt](https://github.com/KDAB/cxx-qt)** | Qt ↔ Rust | Extends CXX for Qt integration | Qt applications |
| **[UniFFI](https://mozilla.github.io/uniffi-rs/)** | Rust → Many | Mozilla's multi-language bindings | Mobile (Swift/Kotlin) |
| **[swift-bridge](https://github.com/nicorboe/swift-bridge)** | Rust ↔ Swift | Direct Swift interop | iOS applications |
| **[jni-rs](https://github.com/jni-rs/jni-rs)** | Rust ↔ Java | Java Native Interface | Android via JNI |
| **[PyO3](https://pyo3.rs/)** | Rust ↔ Python | Python bindings | Python extensions |
| **[Diplomat](https://github.com/nicorboe/diplomat)** | Rust → Many | Language-agnostic FFI | Multiple language targets |

### Decision Guide

```
Need to use existing C library?
  └─► bindgen (generates unsafe Rust wrappers)

Need to expose Rust to C/C++?
  └─► cbindgen (generates C headers)

Creating new bidirectional API with safety?
  └─► CXX (compile-time verified, limited types)

Working with existing C++ headers?
  └─► AutoCXX (uses CXX under the hood)

Need Swift/Kotlin bindings for mobile?
  └─► UniFFI (proc-macro based, generates idiomatic bindings)

Qt integration specifically?
  └─► CXX-Qt
```

---

## Core FFI Patterns

### Basic C FFI

```rust
// Rust side: lib.rs
use std::ffi::{c_char, c_int, CStr};
use std::ptr;

/// Process a C string, returning status code.
///
/// # Safety
/// - `input` must be a valid, null-terminated C string
/// - `input` must remain valid for the duration of the call
#[no_mangle]
pub unsafe extern "C" fn process_string(input: *const c_char) -> c_int {
    if input.is_null() {
        return -1; // Error: null pointer
    }

    let c_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Error: invalid UTF-8
    };

    // Process the string
    if c_str.is_empty() {
        return -3; // Error: empty string
    }

    println!("Processing: {}", c_str);
    0 // Success
}

/// Create a Rust-managed string, returning ownership to caller.
///
/// # Safety
/// Caller must eventually pass the returned pointer to `free_string`.
#[no_mangle]
pub extern "C" fn create_string() -> *mut c_char {
    let s = String::from("Hello from Rust!");
    let c_string = std::ffi::CString::new(s).unwrap();
    c_string.into_raw()
}

/// Free a Rust-allocated string.
///
/// # Safety
/// - `ptr` must have been returned by `create_string`
/// - `ptr` must not be used after this call
#[no_mangle]
pub unsafe extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(std::ffi::CString::from_raw(ptr));
    }
}
```

```c
// C side
#include <stdio.h>

extern int process_string(const char* input);
extern char* create_string(void);
extern void free_string(char* ptr);

int main() {
    // Call Rust function
    int result = process_string("Hello from C!");
    printf("Result: %d\n", result);

    // Get string from Rust
    char* rust_str = create_string();
    printf("From Rust: %s\n", rust_str);
    free_string(rust_str); // MUST free!

    return 0;
}
```

### Opaque Handle Pattern

For complex Rust types, expose opaque handles:

```rust
// Rust side
use std::sync::Mutex;

pub struct GameEngine {
    state: Mutex<GameState>,
    config: Config,
}

// Opaque pointer type for FFI
#[repr(C)]
pub struct GameEngineHandle {
    _private: [u8; 0], // Zero-sized, opaque
}

impl GameEngine {
    fn new(config: Config) -> Self {
        Self {
            state: Mutex::new(GameState::default()),
            config,
        }
    }
}

#[no_mangle]
pub extern "C" fn engine_create() -> *mut GameEngineHandle {
    let engine = Box::new(GameEngine::new(Config::default()));
    Box::into_raw(engine) as *mut GameEngineHandle
}

#[no_mangle]
pub unsafe extern "C" fn engine_update(handle: *mut GameEngineHandle, dt: f32) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let engine = &*(handle as *mut GameEngine);
    match engine.state.lock() {
        Ok(mut state) => {
            state.update(dt);
            0
        }
        Err(_) => -2, // Mutex poisoned
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_destroy(handle: *mut GameEngineHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle as *mut GameEngine));
    }
}
```

```c
// C header (generated by cbindgen or written manually)
typedef struct GameEngineHandle GameEngineHandle;

GameEngineHandle* engine_create(void);
int engine_update(GameEngineHandle* handle, float dt);
void engine_destroy(GameEngineHandle* handle);
```

---

## Memory Safety Across FFI Boundaries

### The Microservice Model

**Core Principle:** Treat the Rust/C++ boundary like a service boundary. Don't share memory; exchange messages.

```
┌─────────────────────────┐         ┌─────────────────────────┐
│       Rust Side         │         │       C++ Side          │
│                         │         │                         │
│  ┌───────────────────┐  │   FFI   │  ┌───────────────────┐  │
│  │   Rust Data       │  │ ◄─────► │  │   C++ Data        │  │
│  │   Rust Threads    │  │  Calls  │  │   C++ Threads     │  │
│  └───────────────────┘  │         │  └───────────────────┘  │
│                         │         │                         │
└─────────────────────────┘         └─────────────────────────┘
```

**Benefits:**

- Avoids data ownership conflicts
- Prevents thread blocking issues
- Shifts problem from "call this code" to "perform this service"
- Creates clean boundaries for future full Rust conversion

### Ownership Rules

| Pattern | Rule |
|---------|------|
| Rust creates, C uses temporarily | Rust retains ownership; C must not store pointer |
| Rust creates, C takes ownership | Document explicitly; C must call Rust's free function |
| C creates, Rust uses | Rust must not free; document lifetime requirements |
| Shared data | Use reference counting or explicit synchronization |

```rust
// Pattern: Rust retains ownership, C borrows temporarily
#[no_mangle]
pub extern "C" fn get_name(handle: *const MyStruct) -> *const c_char {
    // SAFETY: Returned pointer is only valid while handle is valid
    unsafe {
        if handle.is_null() {
            return ptr::null();
        }
        (*handle).name.as_ptr()
    }
}

// Pattern: Rust creates, caller takes ownership
#[no_mangle]
pub extern "C" fn clone_data(handle: *const MyStruct) -> *mut u8 {
    // Caller MUST call free_data() with this pointer
    unsafe {
        if handle.is_null() {
            return ptr::null_mut();
        }
        let data = (*handle).data.clone();
        let boxed = data.into_boxed_slice();
        Box::into_raw(boxed) as *mut u8
    }
}
```

### Thread Safety

Keep threads and data on their respective sides:

```rust
// ❌ DANGEROUS: Sharing mutable data across FFI and threads
static mut SHARED_STATE: Option<GameState> = None;

// ✅ SAFE: Thread-local or explicitly synchronized
thread_local! {
    static THREAD_STATE: RefCell<Option<GameState>> = RefCell::new(None);
}

// ✅ SAFE: Arc<Mutex<T>> for truly shared state
lazy_static::lazy_static! {
    static ref SHARED_STATE: Arc<Mutex<GameState>> =
        Arc::new(Mutex::new(GameState::default()));
}
```

---

## Error Handling Across Languages

### C-Compatible Error Codes

```rust
/// Error codes for FFI functions.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Success = 0,
    NullPointer = -1,
    InvalidUtf8 = -2,
    InvalidArgument = -3,
    OutOfMemory = -4,
    IoError = -5,
    Unknown = -255,
}

impl From<std::io::Error> for ErrorCode {
    fn from(_: std::io::Error) -> Self {
        ErrorCode::IoError
    }
}

// Return error code with out-parameter for result
#[no_mangle]
pub extern "C" fn compute_value(
    input: i32,
    output: *mut i32,
) -> ErrorCode {
    if output.is_null() {
        return ErrorCode::NullPointer;
    }

    if input < 0 {
        return ErrorCode::InvalidArgument;
    }

    // Perform computation
    let result = input * 2;

    unsafe {
        *output = result;
    }

    ErrorCode::Success
}
```

### Extended Error Information

```rust
use std::cell::RefCell;
use std::ffi::CString;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

/// Get the last error message. Returns NULL if no error.
/// The returned string is valid until the next error occurs.
#[no_mangle]
pub extern "C" fn get_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        match e.borrow().as_ref() {
            Some(s) => s.as_ptr(),
            None => ptr::null(),
        }
    })
}

#[no_mangle]
pub extern "C" fn fallible_operation(input: i32) -> ErrorCode {
    if input < 0 {
        set_last_error("input must be non-negative");
        return ErrorCode::InvalidArgument;
    }
    // ...
    ErrorCode::Success
}
```

### CRITICAL: Never Panic Across FFI

```rust
// ❌ FORBIDDEN: Panic will unwind across FFI boundary (undefined behavior)
#[no_mangle]
pub extern "C" fn dangerous_function() {
    panic!("This will corrupt the stack!");
}

// ✅ REQUIRED: Catch panics at FFI boundary
#[no_mangle]
pub extern "C" fn safe_function() -> ErrorCode {
    match std::panic::catch_unwind(|| {
        // Your actual logic here
        do_something_that_might_panic()
    }) {
        Ok(result) => result,
        Err(_) => {
            set_last_error("internal error: panic caught");
            ErrorCode::Unknown
        }
    }
}
```

---

## Using CXX for Safe Bidirectional FFI

CXX provides compile-time verified, safe interop:

```rust
// src/lib.rs
#[cxx::bridge]
mod ffi {
    // Shared structs (must be compatible on both sides)
    struct GameConfig {
        width: u32,
        height: u32,
        fullscreen: bool,
    }

    // Rust functions callable from C++
    extern "Rust" {
        type GameEngine;

        fn create_engine(config: GameConfig) -> Box<GameEngine>;
        fn update(engine: &mut GameEngine, dt: f32) -> bool;
        fn get_score(engine: &GameEngine) -> u64;
    }

    // C++ functions callable from Rust
    unsafe extern "C++" {
        include!("game/renderer.h");

        type Renderer;

        fn create_renderer() -> UniquePtr<Renderer>;
        fn render_frame(renderer: &Renderer, data: &[u8]);
    }
}

pub struct GameEngine {
    state: GameState,
    score: u64,
}

fn create_engine(config: ffi::GameConfig) -> Box<GameEngine> {
    Box::new(GameEngine {
        state: GameState::new(config.width, config.height),
        score: 0,
    })
}

fn update(engine: &mut GameEngine, dt: f32) -> bool {
    engine.state.update(dt);
    engine.score += 1;
    true
}

fn get_score(engine: &GameEngine) -> u64 {
    engine.score
}
```

```cpp
// C++ side
#include "rust/cxx.h"
#include "game/src/lib.rs.h"  // Generated by CXX

int main() {
    GameConfig config{1920, 1080, true};
    auto engine = create_engine(config);

    while (update(*engine, 0.016f)) {
        auto score = get_score(*engine);
        // ...
    }

    return 0;
}
```

---

## Build System Integration

### Cargo + CMake

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.22)
project(hybrid_app)

# Use Corrosion for Cargo integration
include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5
)
FetchContent_MakeAvailable(Corrosion)

# Import Rust crate
corrosion_import_crate(MANIFEST_PATH rust/Cargo.toml)

# Link to C++ target
add_executable(app main.cpp)
target_link_libraries(app PRIVATE my_rust_lib)
```

### build.rs with cbindgen

```rust
// build.rs
fn main() {
    // Generate C header from Rust code
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("MY_LIB_H")
        .with_pragma_once(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("include/my_lib.h");

    println!("cargo::rerun-if-changed=src/lib.rs");
}
```

### build.rs with bindgen

```rust
// build.rs
fn main() {
    // Generate Rust bindings from C header
    println!("cargo::rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
```

### Cargo.toml for FFI Library

```toml
[package]
name = "my_ffi_lib"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib", "lib"]

[dependencies]
libc = "0.2"

[build-dependencies]
cbindgen = "0.27"
```

---

## Performance Considerations

### FFI Call Overhead

FFI calls have overhead:

- Prevents cross-boundary inlining
- May require type conversions
- Affects optimizer visibility

**Mitigation strategies:**

1. **Batch operations** — Reduce call frequency
2. **Coarse-grained interfaces** — Larger units of work per call
3. **Link-Time Optimization (LTO)** — LLVM can optimize across C/Rust with proper setup

```toml
# Enable cross-language LTO
[profile.release]
lto = true
```

```bash
# Use matching LLVM for cross-language LTO
CC=clang CXX=clang++ cargo build --release
```

### Avoiding Unnecessary Copies

```rust
// ❌ SLOW: Copies data across boundary
#[no_mangle]
pub extern "C" fn process_data(data: *const u8, len: usize) -> *mut u8 {
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let mut vec = slice.to_vec(); // Copy!
    process_in_place(&mut vec);
    let boxed = vec.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

// ✅ FAST: Process in-place without copying
#[no_mangle]
pub unsafe extern "C" fn process_data_inplace(data: *mut u8, len: usize) -> i32 {
    if data.is_null() {
        return -1;
    }
    let slice = std::slice::from_raw_parts_mut(data, len);
    process_in_place(slice);
    0
}
```

---

## Testing Hybrid Code

### Test Strategy

```
tests/
├── rust_unit/          # Pure Rust unit tests
├── cpp_unit/           # Pure C++ unit tests
├── ffi_boundary/       # FFI interface tests
│   ├── type_conversion_tests
│   ├── error_handling_tests
│   └── memory_safety_tests
└── integration/        # Full system tests
```

### Memory Safety Testing

```bash
# AddressSanitizer (catches memory errors)
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test
# Also compile C++ with -fsanitize=address

# Miri for Rust-only code (catches UB)
cargo +nightly miri test

# Valgrind for the combined binary
valgrind --leak-check=full ./target/debug/my_hybrid_app

# Fuzz the FFI boundary
cargo +nightly fuzz run ffi_fuzz_target
```

### Rust Unit Tests for FFI Functions

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_process_string_valid() {
        let input = CString::new("test input").unwrap();
        let result = unsafe { process_string(input.as_ptr()) };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_process_string_null() {
        let result = unsafe { process_string(std::ptr::null()) };
        assert_eq!(result, -1);
    }

    #[test]
    fn test_engine_lifecycle() {
        unsafe {
            let handle = engine_create();
            assert!(!handle.is_null());

            let result = engine_update(handle, 0.016);
            assert_eq!(result, 0);

            engine_destroy(handle);
        }
    }
}
```

---

## Checklist

### Before Starting

- [ ] Identify self-contained, problematic module
- [ ] Define clean interface boundaries
- [ ] Choose appropriate FFI tooling
- [ ] Set up build system integration
- [ ] Plan testing strategy

### During Development

- [ ] Keep data on its native side (microservice model)
- [ ] Use `extern "C"` for all cross-language calls
- [ ] Catch panics at FFI boundary
- [ ] Never throw C++ exceptions across FFI
- [ ] Document ownership and lifetime requirements
- [ ] Minimize `unsafe` blocks

### Code Review Focus

- [ ] All `unsafe` blocks justified and minimal
- [ ] Null checks before pointer dereference
- [ ] Lifetime annotations match documentation
- [ ] Error codes defined and consistent
- [ ] Memory ownership clearly documented

### Testing

- [ ] Unit tests for both sides
- [ ] Integration tests across boundary
- [ ] Memory sanitizer passes
- [ ] Fuzz testing for untrusted input

### Production

- [ ] Static linking preferred
- [ ] Consistent Rust version across project
- [ ] Enable LTO for performance-critical code
- [ ] Document all `unsafe` code
- [ ] Panic handling at all FFI entry points

---

## Resources

- [Rust FFI Omnibus](http://jakegoulding.com/rust-ffi-omnibus/) — Comprehensive FFI examples
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — Unsafe Rust documentation
- [CXX Documentation](https://cxx.rs/) — Safe C++/Rust interop
- [bindgen User Guide](https://rust-lang.github.io/rust-bindgen/) — Generating Rust bindings
- [cbindgen Documentation](https://github.com/mozilla/cbindgen/blob/master/docs.md) — Generating C headers
- [UniFFI Book](https://mozilla.github.io/uniffi-rs/) — Mozilla's multi-language bindings
- [KDAB Whitepaper](https://publications.kdab.com/bestpractices/) — Hybrid application best practices

---

*FFI enables gradual, safe adoption of Rust in existing codebases. Design for safety at the boundary, and treat it like a service interface.*
