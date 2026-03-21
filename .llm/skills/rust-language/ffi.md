<!-- CATEGORY: Rust Language -->
<!-- WHEN: Writing FFI code, C interop, bindgen usage -->
# Rust FFI Best Practices

## When to Use FFI

**Good candidates:** Leaf libraries, parsers, modules processing untrusted data, media handling.
**Avoid converting:** Battle-tested crypto, code without allocation, long-stable code.
**Problematic:** Complex OOP hierarchies, Qt apps, template-heavy code.

## Tool Comparison

| Tool | Direction | Best For |
|------|-----------|----------|
| `extern "C"` | Both | Simple C functions |
| `bindgen` | C/C++ -> Rust | Consuming C libraries |
| `cbindgen` | Rust -> C/C++ | Exposing Rust to C |
| `CXX` | Bidirectional | Safe interop with type restrictions |
| `AutoCXX` | C++ -> Rust | Existing C++ APIs |
| `UniFFI` | Rust -> Many | Mobile (Swift/Kotlin) |
| `PyO3` | Rust <-> Python | Python extensions |

## Basic C FFI

```rust
#[no_mangle]
pub unsafe extern "C" fn process_string(input: *const c_char) -> c_int {
    if input.is_null() { return -1; }
    let c_str = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    0 // success
}
```

## Opaque Handle Pattern

```rust
pub struct GameEngine { /* fields */ }

#[no_mangle]
pub extern "C" fn engine_create() -> *mut GameEngine {
    Box::into_raw(Box::new(GameEngine::new()))
}

#[no_mangle]
pub unsafe extern "C" fn engine_destroy(handle: *mut GameEngine) {
    if !handle.is_null() { drop(Box::from_raw(handle)); }
}
```

## Memory Safety: The Microservice Model

Treat the Rust/C++ boundary like a service boundary. Don't share memory; exchange messages.

### Ownership Rules

| Pattern | Rule |
|---------|------|
| Rust creates, C borrows | Rust retains ownership; C must not store pointer |
| Rust creates, C takes ownership | C must call Rust's free function |
| C creates, Rust uses | Rust must not free; document lifetime |
| Shared data | Reference counting or explicit sync |

## Error Handling

```rust
#[repr(C)]
pub enum ErrorCode {
    Success = 0, NullPointer = -1, InvalidUtf8 = -2,
    InvalidArgument = -3, OutOfMemory = -4, Unknown = -255,
}
```

### Extended Error Info (thread-local)
```rust
thread_local! { static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None); }

#[no_mangle]
pub extern "C" fn get_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(ptr::null(), |s| s.as_ptr()))
}
```

### CRITICAL: Never Panic Across FFI

```rust
#[no_mangle]
pub extern "C" fn safe_function() -> ErrorCode {
    // Closure must be UnwindSafe; AssertUnwindSafe opts in when you've verified safety
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { /* logic */ })) {
        Ok(result) => result,
        Err(_) => ErrorCode::Unknown,
    }
}
```

## CXX for Safe Bidirectional FFI

```rust
#[cxx::bridge]
mod ffi {
    struct GameConfig { width: u32, height: u32, fullscreen: bool }
    extern "Rust" {
        type GameEngine;
        fn create_engine(config: GameConfig) -> Box<GameEngine>;
    }
    unsafe extern "C++" {
        include!("renderer.h");
        type Renderer;
        fn render_frame(renderer: &Renderer, data: &[u8]);
    }
}
```

## Build System

### cbindgen (Rust -> C header)
```rust
// build.rs: panic is appropriate for build failures
cbindgen::Builder::new()
    .with_crate(&crate_dir)
    .with_language(cbindgen::Language::C)
    .generate().unwrap()
    .write_to_file("include/my_lib.h");
```

### bindgen (C header -> Rust)
```rust
// build.rs: panic is appropriate for build failures
bindgen::Builder::default()
    .header("wrapper.h")
    .generate().unwrap()
    .write_to_file(out_path.join("bindings.rs")).unwrap();
```

### Cargo.toml
```toml
[lib]
crate-type = ["cdylib", "staticlib", "lib"]
```

## Performance

- **Batch operations** -- reduce call frequency across FFI
- **Process in-place** -- avoid copying data across boundary
- **Enable LTO** -- `lto = true` in release profile + `CC=clang`

## Safety Checklist

- [ ] All `unsafe` blocks justified and minimal
- [ ] Null checks before pointer dereference
- [ ] Panic caught at every FFI entry point
- [ ] Memory ownership clearly documented
- [ ] Error codes defined and consistent
- [ ] Test with AddressSanitizer and Miri
- [ ] Fuzz the FFI boundary for untrusted input
