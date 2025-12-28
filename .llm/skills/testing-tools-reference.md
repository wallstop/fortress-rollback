# Rust Testing Tools Reference — Ecosystem Guide

> **This document provides a comprehensive reference for Rust testing tools and frameworks.**
> Use this guide to select and configure the right tools for your testing needs.

## Quick Selection Guide

| Need | Tool | Command |
|------|------|---------|
| Fast test execution | cargo-nextest | `cargo nextest run` |
| Find edge cases automatically | proptest | `proptest! { }` |
| Table-driven tests | rstest / test-case | `#[rstest]` / `#[test_case]` |
| Mock dependencies | mockall | `#[automock]` |
| Mock HTTP APIs | wiremock | `MockServer::start()` |
| Snapshot testing | insta | `assert_snapshot!()` |
| Generate test data | fake | `Faker.fake()` |
| Verify test quality | cargo-mutants | `cargo mutants` |
| Fuzz testing | cargo-fuzz / afl | `cargo fuzz run` |

---

## cargo-nextest — Modern Test Runner

### Overview

A next-generation test runner that runs each test in its own process, providing better isolation, parallelism, and output.

### Installation

```bash
cargo install cargo-nextest --locked
```

### Basic Usage

```bash
# Run all tests
cargo nextest run

# Run specific test by name
cargo nextest run test_parse_empty

# Run tests matching pattern
cargo nextest run parse_

# Use filter expressions
cargo nextest run -E 'test(parse_) | test(validate_)'

# Run with retries for flaky tests
cargo nextest run --retries 2

# Show slow tests
cargo nextest run --slow-timeout 1s
```

### Configuration

Create `.config/nextest.toml`:

```toml
[profile.default]
# Fail immediately on first failure
fail-fast = true
# Timeout for slow tests
slow-timeout = { period = "60s", terminate-after = 2 }
# Retries for flaky tests
retries = 0

[profile.ci]
# Don't fail fast in CI — run all tests
fail-fast = false
# Retry flaky tests
retries = 2

# Mark specific tests as needing more resources
[[profile.default.overrides]]
filter = "test(/integration/)"
threads-required = 2
```

### Filter Expressions

```bash
# Tests containing "parse"
cargo nextest run -E 'test(parse)'

# Tests in specific package
cargo nextest run -E 'package(my_crate)'

# Combine with OR
cargo nextest run -E 'test(parse) | test(validate)'

# Exclude patterns
cargo nextest run -E 'not test(slow)'
```

### Limitations

- **Does not support doc tests** — run separately with `cargo test --doc`
- Requires separate installation

---

## proptest — Property-Based Testing

### Overview

Generates random inputs to find edge cases. When a test fails, automatically shrinks input to minimal reproduction.

### Installation

```toml
[dev-dependencies]
proptest = "1.5"
```

### Basic Usage

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_encode_decode(data in any::<Vec<u8>>()) {
        let encoded = encode(&data);
        let decoded = decode(&encoded)?;
        prop_assert_eq!(data, decoded);
    }
}
```

### Common Strategies

```rust
use proptest::prelude::*;

proptest! {
    // Integer range
    #[test]
    fn test_positive(n in 1i32..1000) {
        prop_assert!(n > 0);
    }

    // String matching regex
    #[test]
    fn test_identifier(s in "[a-zA-Z][a-zA-Z0-9_]*") {
        prop_assert!(is_valid_identifier(&s));
    }

    // Collection with bounded size
    #[test]
    fn test_vec(v in prop::collection::vec(any::<i32>(), 0..100)) {
        prop_assert!(v.len() < 100);
    }

    // Optional value
    #[test]
    fn test_option(opt in any::<Option<i32>>()) {
        // Test with Some and None
    }

    // Multiple inputs
    #[test]
    fn test_multiple(a in 0..100i32, b in 0..100i32) {
        let sum = a + b;
        prop_assert!(sum < 200);
    }
}
```

### Custom Strategies

```rust
use proptest::prelude::*;

// Combine strategies
prop_compose! {
    fn valid_player_id()(id in 0usize..100) -> PlayerId {
        PlayerId(id)
    }
}

prop_compose! {
    fn valid_input()(
        frame in 0u32..1000,
        player in valid_player_id(),
        data in prop::collection::vec(any::<u8>(), 0..32),
    ) -> GameInput {
        GameInput { frame, player, data }
    }
}

proptest! {
    #[test]
    fn test_with_custom(input in valid_input()) {
        process_input(&input)?;
    }
}
```

### Regression Files

When a test fails, proptest saves the failing input to `proptest-regressions/`:

```
proptest-regressions/
  my_module.txt  ← Failing inputs replayed on every run
```

**Commit these files** to ensure bugs don't regress.

---

## rstest — Fixtures and Parameterized Tests

### Overview

Provides test fixtures (reusable setup) and parameterized testing with excellent ergonomics.

### Installation

```toml
[dev-dependencies]
rstest = "0.22"
```

### Fixtures

```rust
use rstest::{rstest, fixture};

// Define a fixture
#[fixture]
fn session() -> Session {
    Session::new(4).expect("Failed to create session")
}

// Fixture with parameters
#[fixture]
fn session_with_players(#[default(2)] count: usize) -> Session {
    let mut session = Session::new(4).unwrap();
    for i in 0..count {
        session.add_player(PlayerHandle::new(i)).unwrap();
    }
    session
}

// Use fixtures in tests
#[rstest]
fn new_session_has_no_players(session: Session) {
    assert_eq!(session.player_count(), 0);
}

#[rstest]
fn session_with_two_players(session_with_players: Session) {
    assert_eq!(session_with_players.player_count(), 2);
}

// Override fixture default
#[rstest]
fn session_with_four_players(
    #[with(4)]
    session_with_players: Session
) {
    assert_eq!(session_with_players.player_count(), 4);
}
```

### Parameterized Tests

```rust
use rstest::rstest;

#[rstest]
#[case(0, 0, 0)]
#[case(1, 1, 2)]
#[case(2, 2, 4)]
#[case(-1, 1, 0)]
fn test_add(#[case] a: i32, #[case] b: i32, #[case] expected: i32) {
    assert_eq!(add(a, b), expected);
}
```

### Test Matrix

```rust
use rstest::rstest;

#[rstest]
fn test_all_combinations(
    #[values(Compression::None, Compression::Zstd, Compression::Lz4)]
    compression: Compression,
    #[values(true, false)]
    checksum: bool,
    #[values(1, 10, 100)]
    size: usize,
) {
    let data = generate_data(size);
    let result = process(&data, compression, checksum);
    assert!(result.is_ok());
}
// Generates 3 × 2 × 3 = 18 test cases
```

---

## test-case — Simple Parameterized Tests

### Overview

Simpler alternative to rstest for table-driven tests without fixtures.

### Installation

```toml
[dev-dependencies]
test-case = "3"
```

### Usage

```rust
use test_case::test_case;

#[test_case(-2, -4 ; "negative numbers")]
#[test_case(0, 0 ; "zeros")]
#[test_case(2, 4 ; "positive numbers")]
fn test_double(input: i32, expected: i32) {
    assert_eq!(input * 2, expected);
}

// With expected result
#[test_case("test" => "TEST" ; "lowercase to upper")]
#[test_case("TEST" => "TEST" ; "already upper")]
#[test_case("TeSt" => "TEST" ; "mixed case")]
fn test_to_uppercase(input: &str) -> String {
    input.to_uppercase()
}
```

### Test Matrix

```rust
use test_case::test_matrix;

#[test_matrix(
    [1, 2, 3],
    [10, 20]
)]
fn test_multiply(a: i32, b: i32) {
    assert!(a * b > 0);
}
// Generates 3 × 2 = 6 test cases
```

---

## mockall — Mocking Framework

### Overview

Creates mock versions of traits for unit testing, with expectations and call verification.

### Installation

```toml
[dev-dependencies]
mockall = "0.13"
```

### Basic Usage

```rust
use mockall::{automock, predicate::*};

#[automock]
trait Database {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: &str) -> Result<(), Error>;
}

#[test]
fn test_with_mock_database() {
    let mut mock = MockDatabase::new();

    // Set expectations
    mock.expect_get()
        .with(eq("user:123"))
        .times(1)
        .returning(|_| Some("Alice".to_string()));

    mock.expect_set()
        .with(eq("user:456"), eq("Bob"))
        .times(1)
        .returning(|_, _| Ok(()));

    // Use the mock
    let service = UserService::new(mock);
    let user = service.get_user("123").unwrap();
    assert_eq!(user.name, "Alice");
}
```

### Sequence Verification

```rust
use mockall::Sequence;

#[test]
fn test_call_order() {
    let mut mock = MockDatabase::new();
    let mut seq = Sequence::new();

    mock.expect_connect()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|| Ok(()));

    mock.expect_query()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![]));

    mock.expect_disconnect()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|| ());

    // Out-of-order calls will panic
}
```

### Return Different Values

```rust
#[test]
fn test_multiple_returns() {
    let mut mock = MockCounter::new();

    mock.expect_next()
        .times(3)
        .returning({
            let mut count = 0;
            move || {
                count += 1;
                count
            }
        });

    assert_eq!(mock.next(), 1);
    assert_eq!(mock.next(), 2);
    assert_eq!(mock.next(), 3);
}
```

---

## wiremock — HTTP Mocking

### Overview

Runs a real HTTP server returning pre-configured responses for testing HTTP clients.

### Installation

```toml
[dev-dependencies]
wiremock = "0.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Usage

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json};

#[tokio::test]
async fn test_api_client() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure response
    Mock::given(method("GET"))
        .and(path("/users/123"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "id": 123,
                    "name": "Alice"
                }))
        )
        .mount(&mock_server)
        .await;

    // Use mock server URL
    let client = ApiClient::new(&mock_server.uri());
    let user = client.get_user(123).await.unwrap();

    assert_eq!(user.name, "Alice");
}

#[tokio::test]
async fn test_error_handling() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users/456"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = ApiClient::new(&mock_server.uri());
    let result = client.get_user(456).await;

    assert!(result.is_err());
}
```

---

## insta — Snapshot Testing

### Overview

Compares output against stored reference values. Ideal for complex outputs that change rarely.

### Installation

```toml
[dev-dependencies]
insta = { version = "1.40", features = ["yaml"] }
```

```bash
cargo install cargo-insta
```

### Basic Usage

```rust
use insta::{assert_snapshot, assert_debug_snapshot, assert_yaml_snapshot};

#[test]
fn test_formatted_output() {
    let output = format_document(&doc);
    assert_snapshot!(output);
}

#[test]
fn test_data_structure() {
    let ast = parse("1 + 2 * 3").unwrap();
    assert_debug_snapshot!(ast);
}

#[test]
fn test_serialized_data() {
    let user = User { name: "Alice", age: 30 };
    assert_yaml_snapshot!(user);
}
```

### Workflow

```bash
# Run tests — failures create .snap.new files
cargo test

# Review snapshots interactively
cargo insta review

# Accept all pending snapshots
INSTA_UPDATE=always cargo test

# Reject all pending snapshots
cargo insta reject
```

### Inline Snapshots

Store expected value in source file:

```rust
#[test]
fn test_inline() {
    let value = compute_something();
    assert_snapshot!(value, @"expected value here");
}
```

### Redactions

For dynamic values like timestamps:

```rust
use insta::with_settings;

#[test]
fn test_with_redactions() {
    let response = api.get_response();

    insta::with_settings!({
        filters => vec![
            (r"\d{4}-\d{2}-\d{2}", "[DATE]"),
            (r"[a-f0-9-]{36}", "[UUID]"),
        ]
    }, {
        assert_yaml_snapshot!(response);
    });
}
```

---

## fake — Test Data Generation

### Overview

Generates realistic fake data — names, emails, addresses, etc.

### Installation

```toml
[dev-dependencies]
fake = { version = "3", features = ["derive"] }
```

### Basic Usage

```rust
use fake::{Fake, Faker};
use fake::faker::name::en::*;
use fake::faker::internet::en::*;

// Generate individual values
let name: String = Name().fake();
let email: String = SafeEmail().fake();
let company: String = CompanyName().fake();

// Generate with range
let age: u8 = (18..65).fake();
```

### Derive for Structs

```rust
use fake::{Dummy, Fake, Faker};

#[derive(Debug, Dummy)]
struct User {
    #[dummy(faker = "1..10000")]
    id: u64,
    #[dummy(faker = "Name()")]
    name: String,
    #[dummy(faker = "SafeEmail()")]
    email: String,
    #[dummy(faker = "18..65")]
    age: u8,
}

let user: User = Faker.fake();
println!("{:?}", user);
```

### Reproducible Data

```rust
use fake::rand::SeedableRng;
use fake::rand::rngs::StdRng;

let mut rng = StdRng::seed_from_u64(42);
let reproducible_name: String = Name().fake_with_rng(&mut rng);
// Same seed always produces same data
```

---

## cargo-mutants — Mutation Testing

### Overview

Verifies test quality by injecting bugs and checking if tests catch them.

### Installation

```bash
cargo install --locked cargo-mutants
```

### Basic Usage

```bash
# Test specific file
cargo mutants -f src/parser.rs

# List mutations without running
cargo mutants --list -f src/parser.rs

# Run with timeout
cargo mutants --timeout 30

# Use nextest (faster)
cargo mutants -- --all-targets
```

### Configuration

Create `.cargo/mutants.toml`:

```toml
# Exclude test modules
exclude_globs = ["**/tests.rs", "**/test_*.rs"]

# Exclude specific functions
exclude_re = ["^test_", "^bench_"]

# Timeout per mutation
timeout = 30
```

### Understanding Results

| Outcome | Meaning | Action |
|---------|---------|--------|
| **Caught** ✅ | Test failed → mutant killed | Good coverage |
| **Missed** ⚠️ | Tests pass → mutant survived | Improve tests |
| **Timeout** ⏱️ | Test hung | Usually OK (infinite loop) |
| **Unviable** 🔨 | Doesn't compile | Inconclusive |

---

## pretty_assertions — Better Diff Output

### Overview

Provides colorful diffs when assertions fail, making differences easier to spot.

### Installation

```toml
[dev-dependencies]
pretty_assertions = "1.4"
```

### Usage

```rust
use pretty_assertions::{assert_eq, assert_ne};

#[test]
fn test_with_pretty_diff() {
    let expected = vec!["apple", "banana", "cherry"];
    let actual = vec!["apple", "blueberry", "cherry"];
    assert_eq!(expected, actual);  // Shows colorful diff
}
```

---

## tempfile — Temporary Files

### Overview

Creates temporary files and directories that are automatically cleaned up.

### Installation

```toml
[dev-dependencies]
tempfile = "3.12"
```

### Usage

```rust
use tempfile::{tempfile, tempdir, NamedTempFile};

#[test]
fn test_with_temp_file() {
    // Temporary file (auto-deleted)
    let mut file = tempfile().unwrap();
    writeln!(file, "test data").unwrap();

    // Named temporary file
    let named = NamedTempFile::new().unwrap();
    let path = named.path();  // Use path in tests

    // Temporary directory
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
}
```

---

## Summary: Recommended Test Stack

```toml
[dev-dependencies]
# Fast test runner (install separately: cargo install cargo-nextest)

# Property testing
proptest = "1.5"

# Parameterized tests with fixtures
rstest = "0.22"

# Mocking
mockall = "0.13"

# Snapshot testing
insta = { version = "1.40", features = ["yaml"] }

# Better assertion output
pretty_assertions = "1.4"

# Test data generation
fake = { version = "3", features = ["derive"] }

# Temporary files
tempfile = "3.12"

# Async testing (if needed)
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
wiremock = "0.6"
```

---

*See also: [rust-testing-guide.md](rust-testing-guide.md), [property-testing.md](property-testing.md), [mutation-testing.md](mutation-testing.md)*
