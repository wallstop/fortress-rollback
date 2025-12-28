# Cross-Platform Rust Game Development Guide

> **A practical guide to building Rust games that deploy to desktop, web (WASM), and mobile (iOS/Android).**

## Overview

Cross-platform game development in Rust requires careful attention to platform differences in graphics, audio, input, and build systems. This guide covers practical patterns for games specifically.

---

## Platform Support Matrix

### Current State of Rust Game Engines (2024-2025)

| Engine/Framework | Desktop | Web (WASM) | iOS | Android | Consoles |
|------------------|---------|------------|-----|---------|----------|
| **Bevy** | ✅ Excellent | ✅ Good (WebGL2/WebGPU) | ✅ Good | ✅ Improved | ❌ NDA |
| **Macroquad** | ✅ Excellent | ✅ Excellent (WebGL1) | ⚠️ Experimental | ⚠️ Experimental | ❌ |
| **miniquad** | ✅ Excellent | ✅ Excellent | ⚠️ Experimental | ⚠️ Experimental | ❌ |
| **ggez** | ✅ Good | ⚠️ Limited | ❌ | ❌ | ❌ |
| **godot-rust** | ✅ Excellent | ✅ Via Godot | ✅ Via Godot | ✅ Via Godot | ✅ Via Godot |
| **SDL2-rs** | ✅ Proven | ❌ | ✅ Proven | ✅ Proven | Varies |

### Recommendation by Use Case

| Scenario | Recommended | Reasoning |
|----------|-------------|-----------|
| **Desktop + Web** | Macroquad | Best WASM compatibility, WebGL1 works everywhere |
| **Feature-rich 2D/3D** | Bevy | ECS architecture, rich ecosystem, active development |
| **Maximum control** | miniquad + custom | Fastest compile, smallest binary |
| **Ship to consoles** | godot-rust | Godot handles console certification |
| **Proven mobile** | SDL2-rs | Battle-tested, "A Snake's Tale" shipped on all platforms |

---

## WASM Game Development

### Build Setup

```bash
# Install targets
rustup target add wasm32-unknown-unknown

# Dev tools
cargo install wasm-server-runner   # Auto-serve for development
cargo install trunk                # Production builds with asset bundling
```

### Cargo Configuration

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
runner = "wasm-server-runner"

# For getrandom compatibility (required by many game crates)
[target.wasm32-unknown-unknown.dependencies]
rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
```

Or set via environment:
```bash
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --target wasm32-unknown-unknown
```

### WASM Size Optimization

Binary size directly impacts load times. Apply these optimizations:

```toml
# Cargo.toml
[profile.release]
opt-level = 'z'           # Optimize for size ('s' is less aggressive)
lto = true                # Link-time optimization
codegen-units = 1         # Better optimization, slower compile
panic = 'abort'           # Remove panic unwinding code
strip = true              # Strip symbols

# Post-build optimization (after wasm-bindgen)
# wasm-opt -Oz -o output.wasm input.wasm
```

**Measure your binary:**
```bash
cargo install twiggy
twiggy top target/wasm32-unknown-unknown/release/game.wasm
```

### WASM Limitations for Games

| Limitation | Impact | Workaround |
|------------|--------|------------|
| **No threads** | Single-threaded execution | Use async/await, Web Workers for heavy tasks |
| **No filesystem** | Can't read/write files | Embed assets with `include_bytes!`, use IndexedDB |
| **WebGL2 default** | Max 256 lights, some features unavailable | Target WebGL1 for compatibility or WebGPU for features |
| **No dynamic linking** | Must statically link everything | Accepted limitation |
| **Audio restrictions** | Browser requires user interaction first | Start audio on first click/keypress |

### Game Loop Pattern for WASM

```rust
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::window;

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .expect("window should exist")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register RAF");
}

#[wasm_bindgen(start)]
fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    
    let game_state = Rc::new(RefCell::new(GameState::new()));
    
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    
    *g.borrow_mut() = Some(Closure::new({
        let game_state = game_state.clone();
        move || {
            let mut state = game_state.borrow_mut();
            state.update();
            state.render();
            request_animation_frame(f.borrow().as_ref().unwrap());
        }
    }));
    
    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}
```

### Input Handling in WASM

```rust
use wasm_bindgen::prelude::*;
use web_sys::{KeyboardEvent, MouseEvent, HtmlCanvasElement};

// Enable features in Cargo.toml:
// web-sys = { features = ["Document", "KeyboardEvent", "MouseEvent", "HtmlCanvasElement"] }

fn setup_input(canvas: &HtmlCanvasElement, input_state: Rc<RefCell<InputState>>) {
    // Keyboard
    let input = input_state.clone();
    let keydown = Closure::<dyn FnMut(_)>::new(move |e: KeyboardEvent| {
        let mut state = input.borrow_mut();
        match e.key().as_str() {
            "ArrowUp" | "w" | "W" => state.up = true,
            "ArrowDown" | "s" | "S" => state.down = true,
            "ArrowLeft" | "a" | "A" => state.left = true,
            "ArrowRight" | "d" | "D" => state.right = true,
            " " => state.action = true,
            _ => {}
        }
        e.prevent_default();
    });
    
    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .unwrap();
    keydown.forget();  // Prevent cleanup
    
    // Mouse (for canvas)
    let input = input_state.clone();
    let mousedown = Closure::<dyn FnMut(_)>::new(move |e: MouseEvent| {
        let mut state = input.borrow_mut();
        state.mouse_x = e.offset_x();
        state.mouse_y = e.offset_y();
        state.mouse_down = true;
    });
    
    canvas.add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref()).unwrap();
    mousedown.forget();
}
```

---

## Mobile Game Development

### iOS Setup

**Prerequisites:**
- macOS with Xcode 12+
- Install targets:
  ```bash
  rustup target add aarch64-apple-ios        # Device
  rustup target add aarch64-apple-ios-sim    # Simulator (Apple Silicon)
  rustup target add x86_64-apple-ios         # Simulator (Intel)
  ```

**Build Commands:**
```bash
# Device build
cargo build --release --target aarch64-apple-ios

# Simulator (detect host architecture)
cargo build --release --target aarch64-apple-ios-sim  # M1/M2/M3
cargo build --release --target x86_64-apple-ios       # Intel Mac
```

**Tools:**
| Tool | Purpose |
|------|---------|
| `cargo-xcode` | Generate Xcode project from Cargo |
| `cargo-swift` | Generate Swift Package with UniFFI bindings |
| `cbindgen` | Generate C headers for FFI |

### Android Setup

**Prerequisites:**
- Android NDK (r25+ recommended)
- Install targets:
  ```bash
  rustup target add aarch64-linux-android    # ARM64 (most devices)
  rustup target add armv7-linux-androideabi  # ARMv7 (legacy)
  rustup target add x86_64-linux-android     # Emulator x86_64
  rustup target add i686-linux-android       # Emulator x86
  ```

**Using cargo-ndk:**
```bash
cargo install cargo-ndk

# Build for multiple ABIs
cargo ndk -t arm64-v8a -t armeabi-v7a -o ./app/src/main/jniLibs build --release

# Common flags
cargo ndk --platform 24 -t arm64-v8a build --release
```

**Activity Types:**
- `NativeActivity`: Simpler, no Java/Kotlin required initially
- `GameActivity`: Better input (keyboard, controllers), based on AppCompatActivity

### Mobile-Specific Patterns

**Touch Input Abstraction:**
```rust
pub struct Touch {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub phase: TouchPhase,
}

pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

pub trait InputHandler {
    fn on_touch(&mut self, touch: Touch);
    fn on_key(&mut self, key: KeyCode, pressed: bool);
}

// Abstract over platform specifics
#[cfg(target_os = "ios")]
fn get_touches() -> Vec<Touch> { /* iOS impl */ }

#[cfg(target_os = "android")]
fn get_touches() -> Vec<Touch> { /* Android impl */ }
```

**Lifecycle Handling:**
```rust
pub trait GameLifecycle {
    /// Called when app goes to background
    fn on_pause(&mut self) {
        self.save_state();
        self.pause_audio();
    }
    
    /// Called when app returns to foreground
    fn on_resume(&mut self) {
        self.restore_state();
        self.resume_audio();
    }
    
    /// Called when app is being terminated
    fn on_destroy(&mut self) {
        self.save_state();
    }
}
```

---

## Cross-Platform Architecture

### Recommended Project Structure

```
game/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── game-core/               # Platform-agnostic game logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── simulation.rs    # Deterministic game state
│   │       ├── input.rs         # Input abstraction
│   │       └── assets.rs        # Asset loading traits
│   │
│   └── game-render/             # Rendering abstraction
│       ├── Cargo.toml
│       └── src/lib.rs
│
├── platforms/
│   ├── desktop/                 # Native desktop app
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   │
│   ├── web/                     # WASM build
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs
│   │   └── index.html
│   │
│   ├── ios/                     # iOS app
│   │   ├── Cargo.toml
│   │   └── GameApp.xcodeproj/
│   │
│   └── android/                 # Android app
│       ├── Cargo.toml
│       └── app/
│
└── assets/                      # Shared game assets
```

### Feature Flags for Platforms

```toml
# game-core/Cargo.toml
[features]
default = ["std"]
std = []

# Platform presets
desktop = ["std", "threading", "filesystem"]
web = ["wasm-bindgen", "web-sys", "js-sys"]
mobile = ["std", "touch-input"]

# Capabilities
threading = ["std"]
filesystem = ["std"]
touch-input = []
audio = []
networking = ["std"]
```

### Platform Abstraction Traits

```rust
// game-core/src/platform.rs

/// Time source - different per platform
pub trait Clock {
    fn now_millis(&self) -> u64;
    fn elapsed_since(&self, previous: u64) -> u64 {
        self.now_millis().saturating_sub(previous)
    }
}

/// Random number source - must be deterministic for replays
pub trait Random {
    fn next_u32(&mut self) -> u32;
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

/// Asset loading - different storage per platform
pub trait AssetLoader {
    type Error;
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error>;
    fn load_string(&self, path: &str) -> Result<String, Self::Error>;
}

/// Audio playback
pub trait AudioPlayer {
    fn play_sound(&mut self, id: SoundId);
    fn play_music(&mut self, id: MusicId);
    fn stop_music(&mut self);
    fn set_volume(&mut self, volume: f32);
}

/// Full platform interface
pub trait Platform {
    type Clock: Clock;
    type Random: Random;
    type Assets: AssetLoader;
    type Audio: AudioPlayer;
    
    fn clock(&self) -> &Self::Clock;
    fn random(&mut self) -> &mut Self::Random;
    fn assets(&self) -> &Self::Assets;
    fn audio(&mut self) -> &mut Self::Audio;
}
```

### Native Implementation

```rust
// platforms/desktop/src/platform.rs

#[cfg(not(target_arch = "wasm32"))]
pub struct NativePlatform {
    clock: NativeClock,
    random: Pcg32,
    assets: NativeAssets,
    audio: NativeAudio,
}

pub struct NativeClock;

impl Clock for NativeClock {
    fn now_millis(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

pub struct NativeAssets {
    base_path: std::path::PathBuf,
}

impl AssetLoader for NativeAssets {
    type Error = std::io::Error;
    
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        std::fs::read(self.base_path.join(path))
    }
    
    fn load_string(&self, path: &str) -> Result<String, Self::Error> {
        std::fs::read_to_string(self.base_path.join(path))
    }
}
```

### WASM Implementation

```rust
// platforms/web/src/platform.rs

#[cfg(target_arch = "wasm32")]
pub struct WasmPlatform {
    clock: WasmClock,
    random: Pcg32,
    assets: EmbeddedAssets,
    audio: WebAudio,
}

pub struct WasmClock;

impl Clock for WasmClock {
    fn now_millis(&self) -> u64 {
        js_sys::Date::now() as u64
    }
}

pub struct EmbeddedAssets {
    // Assets compiled into binary
    data: &'static [(&'static str, &'static [u8])],
}

impl AssetLoader for EmbeddedAssets {
    type Error = AssetError;
    
    fn load_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        self.data
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, data)| data.to_vec())
            .ok_or(AssetError::NotFound(path.to_string()))
    }
    
    fn load_string(&self, path: &str) -> Result<String, Self::Error> {
        let bytes = self.load_bytes(path)?;
        String::from_utf8(bytes).map_err(|_| AssetError::InvalidUtf8)
    }
}
```

---

## Asset Embedding Strategies

### Compile-Time Embedding

```rust
// For WASM and mobile - embed assets at compile time
mod assets {
    pub const PLAYER_SPRITE: &[u8] = include_bytes!("../assets/player.png");
    pub const LEVEL_DATA: &str = include_str!("../assets/level1.json");
    pub const SOUND_JUMP: &[u8] = include_bytes!("../assets/jump.wav");
}
```

### Build Script Asset Processing

```rust
// build.rs
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("assets.rs");
    
    let mut code = String::from("pub mod embedded_assets {\n");
    
    // Process each asset file
    for entry in fs::read_dir("assets").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_stem().unwrap().to_str().unwrap();
        let name_upper = name.to_uppercase().replace("-", "_");
        
        code.push_str(&format!(
            "    pub const {}: &[u8] = include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/{}\"));\n",
            name_upper,
            path.file_name().unwrap().to_str().unwrap()
        ));
    }
    
    code.push_str("}\n");
    fs::write(&dest_path, code).unwrap();
    
    println!("cargo::rerun-if-changed=assets/");
}
```

---

## Graphics Backend Selection

### Bevy Feature Configuration

```toml
# Cargo.toml for cross-platform Bevy game
[dependencies.bevy]
version = "0.15"
default-features = false
features = [
    "bevy_asset",
    "bevy_audio",
    "bevy_sprite",
    "bevy_text",
    "bevy_ui",
    "bevy_winit",
    "png",
    "vorbis",
]

# Platform-specific
[target.'cfg(not(target_arch = "wasm32"))'.dependencies.bevy]
version = "0.15"
features = ["bevy_render", "multi_threaded"]

[target.'cfg(target_arch = "wasm32")'.dependencies.bevy]
version = "0.15"
features = ["webgl2"]  # or "webgpu" for cutting-edge browsers
```

### Manual Graphics Backend Selection

```rust
// Using wgpu for cross-platform rendering
pub fn create_graphics_backend() -> Backend {
    #[cfg(target_arch = "wasm32")]
    {
        // WebGL2 is the safe default
        Backend::WebGl2
        // Or for modern browsers: Backend::WebGpu
    }
    
    #[cfg(target_os = "macos")]
    {
        Backend::Metal
    }
    
    #[cfg(target_os = "windows")]
    {
        Backend::Dx12  // or Vulkan
    }
    
    #[cfg(target_os = "linux")]
    {
        Backend::Vulkan
    }
    
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        Backend::OpenGLES
    }
}
```

---

## Audio Handling

### Cross-Platform Audio Libraries

| Library | Desktop | WASM | Mobile | Notes |
|---------|---------|------|--------|-------|
| **kira** | ✅ | ⚠️ Limited | ⚠️ | Rich features, no file I/O in WASM |
| **rodio** | ✅ | ❌ | ⚠️ | Simple API, desktop-focused |
| **cpal** | ✅ | ⚠️ | ⚠️ | Low-level, build on top |
| **web-sys AudioContext** | ❌ | ✅ | ❌ | Native Web Audio API |

### WASM Audio Pattern

```rust
#[cfg(target_arch = "wasm32")]
mod web_audio {
    use wasm_bindgen::prelude::*;
    use web_sys::{AudioContext, OscillatorNode, GainNode};
    
    pub struct WebAudioPlayer {
        context: AudioContext,
        // Store decoded audio buffers
        sounds: std::collections::HashMap<SoundId, web_sys::AudioBuffer>,
    }
    
    impl WebAudioPlayer {
        pub fn new() -> Result<Self, JsValue> {
            let context = AudioContext::new()?;
            Ok(Self {
                context,
                sounds: std::collections::HashMap::new(),
            })
        }
        
        pub fn play(&self, sound_id: SoundId) -> Result<(), JsValue> {
            if let Some(buffer) = self.sounds.get(&sound_id) {
                let source = self.context.create_buffer_source()?;
                source.set_buffer(Some(buffer));
                source.connect_with_audio_node(&self.context.destination())?;
                source.start()?;
            }
            Ok(())
        }
    }
}
```

---

## Testing Across Platforms

### Platform-Specific Test Modules

```rust
// Core tests run everywhere
#[cfg(test)]
mod tests {
    #[test]
    fn test_game_logic() {
        let state = GameState::new();
        assert!(state.is_valid());
    }
}

// WASM-specific tests
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_wasm_rendering() {
        // Test that runs in actual browser
    }
}

// Tests requiring threading
#[cfg(all(test, not(target_arch = "wasm32")))]
mod threaded_tests {
    #[test]
    fn test_parallel_loading() {
        use std::thread;
        // Multi-threaded test
    }
}
```

### CI Matrix for Games

```yaml
# .github/workflows/game-ci.yml
name: Game CI

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        include:
          # Desktop
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          # WASM
          - os: ubuntu-latest
            target: wasm32-unknown-unknown
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Install Linux dependencies
        if: matrix.os == 'ubuntu-latest' && matrix.target != 'wasm32-unknown-unknown'
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
      
      - name: Install wasm-pack
        if: matrix.target == 'wasm32-unknown-unknown'
        run: cargo install wasm-pack
      
      - name: Build
        run: cargo build --target ${{ matrix.target }}
      
      - name: Test (native)
        if: matrix.target != 'wasm32-unknown-unknown'
        run: cargo test --target ${{ matrix.target }}
      
      - name: Test (WASM)
        if: matrix.target == 'wasm32-unknown-unknown'
        run: wasm-pack test --headless --chrome
```

---

## Common Pitfalls and Solutions

### Pitfall: Floating-Point Determinism

**Problem:** Different platforms may produce slightly different float results.

**Solution:** Use fixed-point math or [`libm`](https://crates.io/crates/libm) for cross-platform consistency.

```toml
[dependencies]
libm = "0.2"
```

```rust
use libm::{sinf, cosf, sqrtf};

fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    sqrtf(dx * dx + dy * dy)  // Cross-platform consistent
}
```

### Pitfall: HashMap Non-Determinism

**Problem:** `HashMap` iteration order varies between runs/platforms.

**Solution:** Use `BTreeMap` or sort before iteration.

```rust
use std::collections::BTreeMap;

// ✅ Deterministic iteration
let entities: BTreeMap<EntityId, Entity> = BTreeMap::new();

// ✅ Or sort HashMap keys
let mut keys: Vec<_> = hashmap.keys().collect();
keys.sort();
for key in keys {
    process(hashmap.get(key).unwrap());
}
```

### Pitfall: Audio User Interaction Requirement

**Problem:** Browsers block audio until user interaction.

**Solution:** Initialize audio context on first click/keypress.

```rust
static AUDIO_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn handle_user_input() {
    if !AUDIO_INITIALIZED.swap(true, Ordering::SeqCst) {
        // First interaction - initialize audio
        initialize_audio_context();
    }
    // Normal input handling...
}
```

### Pitfall: WASM Binary Size Bloat

**Problem:** Debug symbols and panic formatting inflate binary.

**Solution:** Strip aggressively for release.

```toml
[profile.release]
opt-level = 'z'
lto = true
codegen-units = 1
panic = 'abort'
strip = true

[profile.release.package."*"]
opt-level = 'z'
```

### Pitfall: Mobile Memory Pressure

**Problem:** Mobile devices kill background apps aggressively.

**Solution:** Save state frequently, handle lifecycle events.

```rust
impl GameLifecycle for MyGame {
    fn on_pause(&mut self) {
        // Autosave on every pause
        if let Err(e) = self.save_to_storage() {
            log::error!("Failed to save: {}", e);
        }
    }
}
```

---

## Checklist for Cross-Platform Games

### Project Setup
- [ ] Workspace structure separating core from platform code
- [ ] Feature flags for platform capabilities
- [ ] Shared asset directory

### Build Configuration
- [ ] All target triplets in `rust-toolchain.toml`
- [ ] WASM runner configured in `.cargo/config.toml`
- [ ] Release profile optimized for size (WASM) or speed (native)

### Code Quality
- [ ] Platform traits for clock, random, assets, audio
- [ ] No `std::time::Instant` in core (use trait)
- [ ] Assets embedded for WASM, loaded from disk for native
- [ ] Deterministic collections (`BTreeMap` over `HashMap`)

### Testing
- [ ] Core logic tests run on all platforms
- [ ] WASM tests with `wasm_bindgen_test`
- [ ] CI builds for all target platforms

### Mobile Specific
- [ ] Lifecycle handling (pause/resume/destroy)
- [ ] Touch input abstraction
- [ ] Signed builds for device testing

---

*Building cross-platform games in Rust is challenging but achievable with the right architecture.*
