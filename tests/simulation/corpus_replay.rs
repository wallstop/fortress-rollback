//! Replay checked-in minimized simulation schedules with the full oracle.

use super::harness::artifact::{read_artifact, FailureArtifact, MAX_FAILURE_ARTIFACT_BYTES};
use super::harness::oracle::OracleFailure;
use super::harness::schedule::{validate_schedule, Schedule, ScheduleEvent};
use super::harness::{run, run_with_input, RunOptions, WideStubInput};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// A standalone schedule cannot be larger than the complete artifact envelope
/// from which promotion extracts it.
const MAX_CORPUS_SCHEDULE_BYTES: usize = MAX_FAILURE_ARTIFACT_BYTES;

fn read_schedule_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("failed to read corpus schedule {}: {error}", path.display()))?;
    let limit = u64::try_from(MAX_CORPUS_SCHEDULE_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read corpus schedule {}: {error}", path.display()))?;
    if bytes.len() > MAX_CORPUS_SCHEDULE_BYTES {
        return Err(format!(
            "corpus schedule {} exceeds {MAX_CORPUS_SCHEDULE_BYTES} bytes",
            path.display()
        ));
    }
    Ok(bytes)
}

fn read_schedule(path: &Path) -> Schedule {
    let bytes = read_schedule_bytes(path).unwrap_or_else(|error| panic!("{error}"));
    let schedule: Schedule = serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        panic!(
            "failed to decode corpus schedule {}: {error}",
            path.display()
        )
    });
    validate_schedule(&schedule)
        .unwrap_or_else(|error| panic!("invalid corpus schedule {}: {error}", path.display()));
    schedule
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn corpus_id(name: &str) -> Result<u16, String> {
    let Some(stem) = name.strip_suffix(".json") else {
        return Err(format!("unexpected corpus extension/name: {name}"));
    };
    let Some((id, slug)) = stem.split_once('-') else {
        return Err(format!(
            "corpus name must be NNN-lowercase-slug.json: {name}"
        ));
    };
    if id.len() != 3 || !id.bytes().all(|byte| byte.is_ascii_digit()) || !valid_slug(slug) {
        return Err(format!(
            "corpus name must be NNN-lowercase-slug.json: {name}"
        ));
    }
    let id = id
        .parse::<u16>()
        .map_err(|error| format!("invalid corpus id in {name}: {error}"))?;
    if id == 0 || id > 999 {
        return Err(format!("corpus id must be 001..=999: {name}"));
    }
    Ok(id)
}

fn corpus_paths_in(root: &Path, allow_hot_join_dir: bool) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    let mut ids = BTreeSet::new();
    let entries = std::fs::read_dir(root).map_err(|error| {
        format!(
            "failed to read corpus directory {}: {error}",
            root.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read corpus entry: {error}"))?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| "corpus entry name is not UTF-8".to_owned())?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "failed to inspect corpus entry {}: {error}",
                entry.path().display()
            )
        })?;
        if file_type.is_symlink() {
            return Err(format!(
                "corpus entries must not be symlinks: {}",
                entry.path().display()
            ));
        }
        if name == "hot-join" && allow_hot_join_dir {
            if !file_type.is_dir() {
                return Err(format!(
                    "corpus hot-join entry must be a directory: {}",
                    entry.path().display()
                ));
            }
            continue;
        }
        if name == ".promotion.lock" {
            if !file_type.is_dir() {
                return Err(format!(
                    "corpus promotion lock must be a directory: {}",
                    entry.path().display()
                ));
            }
            continue;
        }
        if name.starts_with(".candidate.") {
            if !file_type.is_file() {
                return Err(format!(
                    "hidden corpus candidate must be a regular file: {}",
                    entry.path().display()
                ));
            }
            continue;
        }
        if !file_type.is_file() {
            return Err(format!(
                "unexpected non-regular corpus entry: {}",
                entry.path().display()
            ));
        }
        let id = corpus_id(name)?;
        if !ids.insert(id) {
            return Err(format!("duplicate corpus id {id:03} in {}", root.display()));
        }
        paths.push(entry.path());
    }
    paths.sort();
    Ok(paths)
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/simulation/corpus")
}

fn default_corpus_paths() -> Vec<PathBuf> {
    let root = corpus_root();
    corpus_paths_in(&root, true)
        .unwrap_or_else(|error| panic!("invalid default simulation corpus: {error}"))
}

#[cfg(feature = "hot-join")]
fn hot_join_corpus_paths() -> Vec<PathBuf> {
    let root = corpus_root().join("hot-join");
    corpus_paths_in(&root, false)
        .unwrap_or_else(|error| panic!("invalid hot-join simulation corpus: {error}"))
}

fn replay_paths(paths: Vec<PathBuf>) {
    for path in paths {
        let schedule = read_schedule(&path);
        let report = run(&schedule, &RunOptions::default());
        report.expect_pass(&schedule);
    }
}

#[test]
fn checked_in_default_corpus_replays_with_full_oracle() {
    let paths = default_corpus_paths();
    assert!(
        !paths.is_empty(),
        "default simulation corpus must contain at least one minimized schedule"
    );
    replay_paths(paths);
}

#[cfg(feature = "hot-join")]
#[test]
fn checked_in_hot_join_corpus_replays_with_full_oracle() {
    replay_paths(hot_join_corpus_paths());
}

fn has_hot_join(schedule: &Schedule) -> bool {
    schedule
        .events
        .iter()
        .any(|(_, event)| matches!(event, ScheduleEvent::HotJoin { .. }))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Err(format!("refusing to overwrite output: {}", path.display()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("output has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    // A promotion candidate lives inside the corpus directory. Keep the
    // unpublished helper file under the discovery allowlist too, so a
    // concurrent corpus replay never mistakes it for a malformed entry.
    let mut opened = None;
    for attempt in 0..16u8 {
        let temporary = parent.join(format!(
            ".candidate.{}.{}.{}",
            std::process::id(),
            nonce,
            attempt
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => {
                opened = Some((temporary, file));
                break;
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(error) => {
                return Err(format!("failed to create {}: {error}", temporary.display()));
            },
        }
    }
    let Some((temporary, mut file)) = opened else {
        return Err("failed to allocate a unique corpus candidate after 16 attempts".to_owned());
    };
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("failed to write {}: {error}", temporary.display()));
    }
    if let Err(error) = std::fs::hard_link(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!(
            "failed to atomically publish {}: {error}",
            path.display()
        ));
    }
    std::fs::remove_file(&temporary).map_err(|error| {
        format!(
            "published {} but failed to remove temporary {}: {error}",
            path.display(),
            temporary.display()
        )
    })?;
    Ok(())
}

fn reproduce_artifact(artifact: &FailureArtifact) -> Result<(), String> {
    let report = match artifact.replay_input_width_bytes {
        4 => run(&artifact.schedule, &artifact.replay_options),
        32 => run_with_input::<WideStubInput>(&artifact.schedule, &artifact.replay_options),
        width => return Err(format!("unsupported replay input width {width}")),
    };
    if report.trace_hash != artifact.trace_hash {
        return Err(format!(
            "candidate trace hash changed: recorded={} replayed={}",
            artifact.trace_hash, report.trace_hash
        ));
    }
    for (name, matches) in [
        (
            "progress_samples",
            report.progress_samples == artifact.progress_samples,
        ),
        ("trace_tail", report.trace_tail == artifact.trace_tail),
        (
            "frame_opportunities",
            report.frame_opportunities == artifact.frame_opportunities,
        ),
        (
            "wait_frames_obeyed",
            report.wait_frames_obeyed == artifact.wait_frames_obeyed,
        ),
        (
            "wait_recommendation_max",
            report.wait_recommendation_max == artifact.wait_recommendation_max,
        ),
        (
            "wait_recommendation_frames",
            report.wait_recommendation_frames == artifact.wait_recommendation_frames,
        ),
        (
            "wait_recommendations_accepted",
            report.wait_recommendations_accepted == artifact.wait_recommendations_accepted,
        ),
        (
            "wait_frames_accepted",
            report.wait_frames_accepted == artifact.wait_frames_accepted,
        ),
        ("cpu_feedback", report.cpu_feedback == artifact.cpu_feedback),
    ] {
        if !matches {
            return Err(format!(
                "candidate {name} evidence changed independently of the recorded trace hash"
            ));
        }
    }
    let expected = artifact.failures[0].class;
    if report.verdict.failures.first().map(OracleFailure::class) != Some(expected.as_str()) {
        return Err(format!(
            "candidate did not reproduce first failure class {expected:?}: {:?}",
            report.verdict.failures
        ));
    }
    Ok(())
}

fn validate_reproduce_and_extract(artifact_path: &Path, output: &Path) -> Result<(), String> {
    let artifact = read_artifact(artifact_path)?;
    reproduce_artifact(&artifact)?;
    let bytes = serde_json::to_vec_pretty(&artifact.schedule)
        .map_err(|error| format!("failed to serialize extracted schedule: {error}"))?;
    atomic_write(output, &bytes)
}

/// Promotion helper. Classification validates the full artifact under the
/// superset feature build; extraction must then run under exactly the feature
/// set required by the materialized schedule.
#[test]
#[ignore = "promotion helper; requires FORTRESS_SIM_CORPUS_* paths"]
fn validate_and_extract_candidate_artifact() {
    let artifact_path = std::env::var("FORTRESS_SIM_CORPUS_ARTIFACT")
        .expect("FORTRESS_SIM_CORPUS_ARTIFACT must name a failure artifact");
    let mode = std::env::var("FORTRESS_SIM_CORPUS_MODE")
        .expect("FORTRESS_SIM_CORPUS_MODE must be classify or extract");
    let artifact = read_artifact(Path::new(&artifact_path))
        .unwrap_or_else(|error| panic!("invalid promotion artifact: {error}"));
    let route = if has_hot_join(&artifact.schedule) {
        "hot-join"
    } else {
        "default"
    };
    match mode.as_str() {
        "classify" => {
            let output = std::env::var("FORTRESS_SIM_CORPUS_ROUTE_OUTPUT")
                .expect("classify mode requires FORTRESS_SIM_CORPUS_ROUTE_OUTPUT");
            atomic_write(Path::new(&output), format!("{route}\n").as_bytes())
                .unwrap_or_else(|error| panic!("failed to write corpus route: {error}"));
        },
        "extract" => {
            let feature_matches = has_hot_join(&artifact.schedule) == cfg!(feature = "hot-join");
            assert!(
                feature_matches,
                "promotion helper feature mismatch: route={route}, hot-join-feature={}",
                cfg!(feature = "hot-join")
            );
            let output = std::env::var("FORTRESS_SIM_CORPUS_OUTPUT")
                .expect("extract mode requires FORTRESS_SIM_CORPUS_OUTPUT");
            validate_reproduce_and_extract(Path::new(&artifact_path), Path::new(&output))
                .unwrap_or_else(|error| panic!("candidate promotion failed: {error}"));
        },
        other => panic!("unknown FORTRESS_SIM_CORPUS_MODE {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::harness::artifact::{write_artifact, ExistingArtifact, FailureArtifact};
    use crate::simulation::harness::schedule::{generate, CpuFeedbackPolicy, SimConfig};

    #[test]
    fn artifact_reproduction_rejects_mutated_progress_evidence() {
        let schedule = generate(17, SimConfig::smoke(2));
        let options = RunOptions {
            corrupt_state_from: Some((1, 10)),
            ..RunOptions::default()
        };
        let report = run(&schedule, &options);
        assert!(!report.verdict.passed(), "negative control must fail");
        let artifact = FailureArtifact::from_report(&schedule, &report);
        assert_eq!(reproduce_artifact(&artifact), Ok(()));

        let mut mutated = artifact;
        mutated.frame_opportunities[0] = mutated.frame_opportunities[0].saturating_add(1);
        let error = reproduce_artifact(&mutated).expect_err("mutated evidence must be rejected");
        assert!(
            error.contains("frame_opportunities"),
            "wrong error: {error}"
        );
    }

    #[test]
    fn artifact_reproduction_rejects_mutated_cpu_feedback_evidence() {
        let mut config = SimConfig::smoke(2);
        config.steps = 60;
        config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: 8_001,
            max_poll_delay_steps: 8,
        });
        let schedule = generate(18, config);
        let options = RunOptions {
            corrupt_state_from: Some((1, 10)),
            ..RunOptions::default()
        };
        let report = run(&schedule, &options);
        assert!(!report.verdict.passed(), "negative control must fail");
        let artifact = FailureArtifact::from_report(&schedule, &report);
        assert_eq!(reproduce_artifact(&artifact), Ok(()));

        let mut mutated = artifact;
        mutated.cpu_feedback[0].delayed_poll_steps =
            mutated.cpu_feedback[0].delayed_poll_steps.saturating_add(1);
        let error =
            reproduce_artifact(&mutated).expect_err("mutated CPU evidence must be rejected");
        assert!(error.contains("cpu_feedback"), "wrong error: {error}");

        let mut mutated_trace = FailureArtifact::from_report(&schedule, &report);
        mutated_trace.trace_tail[0].cpu_feedback[0].delayed_poll_steps =
            mutated_trace.trace_tail[0].cpu_feedback[0]
                .delayed_poll_steps
                .saturating_add(1);
        let error = reproduce_artifact(&mutated_trace)
            .expect_err("mutated per-step CPU trace evidence must be rejected");
        assert!(error.contains("trace_tail"), "wrong error: {error}");
    }

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "fortress-corpus-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn corpus_discovery_rejects_malformed_symlink_and_duplicate_entries() {
        let cases = [
            "bad.txt",
            "001-BAD.json",
            "000-zero.json",
            "001-double--dash.json",
        ];
        for name in cases {
            let root = temp_root("bad-name");
            std::fs::create_dir_all(&root).expect("temp corpus creates");
            std::fs::write(root.join(name), b"{}").expect("bad entry writes");
            assert!(corpus_paths_in(&root, true).is_err(), "accepted {name}");
            std::fs::remove_dir_all(root).expect("temp corpus removes");
        }

        let root = temp_root("duplicate");
        std::fs::create_dir_all(&root).expect("temp corpus creates");
        std::fs::write(root.join("001-one.json"), b"{}").expect("entry writes");
        std::fs::write(root.join("001-two.json"), b"{}").expect("entry writes");
        assert!(corpus_paths_in(&root, true).is_err());
        std::fs::remove_dir_all(root).expect("temp corpus removes");

        #[cfg(unix)]
        {
            let root = temp_root("symlink");
            std::fs::create_dir_all(&root).expect("temp corpus creates");
            std::fs::write(root.join("target"), b"{}").expect("target writes");
            std::os::unix::fs::symlink(root.join("target"), root.join("001-link.json"))
                .expect("symlink creates");
            assert!(corpus_paths_in(&root, true).is_err());
            std::fs::remove_dir_all(root).expect("temp corpus removes");
        }
    }

    #[test]
    fn corpus_discovery_accepts_only_expected_hidden_and_hot_join_entries() {
        let root = temp_root("allowed");
        std::fs::create_dir_all(root.join("hot-join")).expect("hot-join root creates");
        std::fs::create_dir_all(root.join(".promotion.lock")).expect("lock creates");
        std::fs::write(root.join(".candidate.123"), b"temporary").expect("candidate writes");
        std::fs::write(root.join("001-valid-name.json"), b"{}").expect("entry writes");
        let paths = corpus_paths_in(&root, true).expect("expected entries accepted");
        assert_eq!(paths, vec![root.join("001-valid-name.json")]);
        std::fs::remove_dir_all(root).expect("temp corpus removes");
    }

    #[test]
    fn corpus_reader_rejects_oversized_schedule_before_deserialization() {
        let root = temp_root("oversized");
        std::fs::create_dir_all(&root).expect("temporary root creates");
        let path = root.join("oversized.json");
        let file = std::fs::File::create(&path).expect("oversized corpus file creates");
        file.set_len(
            u64::try_from(MAX_CORPUS_SCHEDULE_BYTES + 1).expect("corpus byte limit fits u64"),
        )
        .expect("oversized corpus file extends");

        let error = read_schedule_bytes(&path).expect_err("oversized corpus input must fail");
        assert!(error.contains("exceeds"), "wrong diagnostic: {error}");
        std::fs::remove_dir_all(root).expect("temporary root removes");
    }

    #[test]
    fn rust_promotion_helper_validates_reproduction_and_extracts_atomically() {
        let mut config = SimConfig::smoke(2);
        config.steps = 60;
        let schedule = generate(91, config);
        let options = RunOptions {
            corrupt_state_from: Some((1, 8)),
            probe_confirmed_at: Some(30),
            ..RunOptions::default()
        };
        let report = run_with_input::<WideStubInput>(&schedule, &options);
        assert!(!report.verdict.passed(), "planted failure must fire");
        let artifact = FailureArtifact::from_report(&schedule, &report);
        assert_eq!(artifact.replay_options, options);
        assert_eq!(artifact.replay_input_width_bytes, 32);
        let root = temp_root("extract");
        let artifact_path = write_artifact(&root, "source", &artifact, ExistingArtifact::Refuse)
            .expect("source artifact writes");
        let output = root.join(".candidate");

        validate_reproduce_and_extract(&artifact_path, &output)
            .expect("full artifact reproduces and extracts");
        assert_eq!(read_schedule(&output), schedule);
        assert!(validate_reproduce_and_extract(&artifact_path, &output).is_err());

        let mut bad_hash = artifact;
        bad_hash.trace_hash ^= 1;
        let bad_path = write_artifact(&root, "bad", &bad_hash, ExistingArtifact::Refuse)
            .expect("modified artifact writes");
        assert!(validate_reproduce_and_extract(&bad_path, &root.join(".bad-candidate")).is_err());
        std::fs::remove_dir_all(root).expect("temp tree removes");
    }

    #[cfg(unix)]
    #[test]
    fn promotion_script_routes_numbers_locks_and_anchors_to_repo_root() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("script");
        let outside = root.join("outside");
        let corpus = root.join("corpus");
        std::fs::create_dir_all(&outside).expect("outside cwd creates");
        let artifact = root.join("artifact.json");
        std::fs::write(&artifact, b"{}\n").expect("dummy artifact writes");
        let fake_cargo = root.join("fake-cargo");
        let fake_log = root.join("fake-cargo.log");
        let fake_bin = root.join("fake-bin");
        std::fs::create_dir_all(&fake_bin).expect("fake bin creates");
        let fake_ln = fake_bin.join("ln");
        std::fs::write(
            &fake_ln,
            br#"#!/bin/sh
set -eu
if [ "${FORTRESS_FAKE_ROUTE:-default}" = signal-publish ]; then
  kill -TERM "$PPID"
  sleep 1
fi
exec /bin/ln "$@"
"#,
        )
        .expect("fake ln writes");
        std::fs::set_permissions(&fake_ln, std::fs::Permissions::from_mode(0o755))
            .expect("fake ln is executable");
        std::fs::write(
            &fake_cargo,
            br#"#!/bin/sh
set -eu
printf '%s|%s|%s\n' "$FORTRESS_SIM_CORPUS_MODE" "${FORTRESS_FAKE_ROUTE:-default}" "$*" >> "$FORTRESS_FAKE_LOG"
case "$FORTRESS_SIM_CORPUS_MODE" in
  classify)
    route=${FORTRESS_FAKE_ROUTE:-default}
    case "$route" in fail-extract|signal-extract|signal-publish) route=default ;; esac
    printf '%s\n' "$route" > "$FORTRESS_SIM_CORPUS_ROUTE_OUTPUT"
    ;;
  extract)
    [ "${FORTRESS_FAKE_ROUTE:-default}" = fail-extract ] && exit 91
    if [ "${FORTRESS_FAKE_ROUTE:-default}" = signal-extract ]; then
      kill -TERM "$PPID"
      sleep 1
      exit 92
    fi
    printf '%s\n' '{"schema_version":9}' > "$FORTRESS_SIM_CORPUS_OUTPUT"
    ;;
  *) exit 90 ;;
esac
"#,
        )
        .expect("fake cargo writes");
        std::fs::set_permissions(&fake_cargo, std::fs::Permissions::from_mode(0o755))
            .expect("fake cargo is executable");
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/simulation/promote-artifact.sh");
        let fake_path = format!(
            "{}:{}",
            fake_bin.display(),
            std::env::var("PATH").expect("PATH is set")
        );
        let invoke = |route: &str, slug: &str| {
            std::process::Command::new(&script)
                .args([artifact.as_os_str(), std::ffi::OsStr::new(slug)])
                .current_dir(&outside)
                .env("FORTRESS_SIM_CARGO", &fake_cargo)
                .env("FORTRESS_SIM_CORPUS_ROOT", &corpus)
                .env("RUNNER_TEMP", root.join("runner-temp"))
                .env("PATH", &fake_path)
                .env("FORTRESS_FAKE_ROUTE", route)
                .env("FORTRESS_FAKE_LOG", &fake_log)
                .output()
                .expect("promotion script executes")
        };

        let first = invoke("default", "first-case");
        assert!(
            first.status.success(),
            "stdout: {}; stderr: {}",
            String::from_utf8_lossy(&first.stdout),
            String::from_utf8_lossy(&first.stderr),
        );
        let second = invoke("default", "second-case");
        assert!(
            second.status.success(),
            "stdout: {}; stderr: {}",
            String::from_utf8_lossy(&second.stdout),
            String::from_utf8_lossy(&second.stderr),
        );
        let hot_join = invoke("hot-join", "joined-case");
        assert!(
            hot_join.status.success(),
            "stdout: {}; stderr: {}",
            String::from_utf8_lossy(&hot_join.stdout),
            String::from_utf8_lossy(&hot_join.stderr),
        );
        let list_dir = |path: &Path| {
            std::fs::read_dir(path)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.path().display().to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        for (path, output) in [
            (corpus.join("001-first-case.json"), &first),
            (corpus.join("002-second-case.json"), &second),
            (corpus.join("hot-join/001-joined-case.json"), &hot_join),
        ] {
            assert!(
                path.is_file(),
                "missing promoted file {}; corpus tree: {:?}; stdout: {}; stderr: {}",
                path.display(),
                (list_dir(&corpus), list_dir(&corpus.join("hot-join"))),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        let invocations = std::fs::read_to_string(&fake_log).expect("fake cargo log reads");
        for line in invocations.lines() {
            let has_hot_join_feature = line.contains("--features hot-join");
            if line.starts_with("classify|") || line.starts_with("extract|hot-join|") {
                assert!(
                    has_hot_join_feature,
                    "missing hot-join feature route: {line}"
                );
            } else if line.starts_with("extract|default|") {
                assert!(
                    !has_hot_join_feature,
                    "default extraction leaked feature: {line}"
                );
            }
        }

        std::fs::create_dir_all(corpus.join(".promotion.lock")).expect("competing lock creates");
        let locked = invoke("default", "locked-case");
        assert!(!locked.status.success(), "promotion ignored an active lock");
        assert!(
            corpus.join(".promotion.lock").is_dir(),
            "failed promotion removed the competing lock"
        );
        assert!(!corpus.join("003-locked-case.json").exists());
        std::fs::remove_dir(corpus.join(".promotion.lock")).expect("competing lock removes");

        let failed_extract = invoke("fail-extract", "failed-case");
        assert_eq!(failed_extract.status.code(), Some(91));
        assert!(
            !corpus.join(".promotion.lock").exists(),
            "failed extraction leaked its candidate lock"
        );

        let signaled_extract = invoke("signal-extract", "signaled-case");
        assert_eq!(signaled_extract.status.code(), Some(143));
        assert!(
            !corpus.join(".promotion.lock").exists(),
            "signaled extraction leaked its candidate lock"
        );

        let signaled_publish = invoke("signal-publish", "signaled-publish");
        assert!(
            signaled_publish.status.success(),
            "stdout: {}; stderr: {}",
            String::from_utf8_lossy(&signaled_publish.stdout),
            String::from_utf8_lossy(&signaled_publish.stderr),
        );
        assert!(corpus.join("003-signaled-publish.json").is_file());
        assert_eq!(
            std::fs::read_dir(root.join("runner-temp"))
                .expect("runner temp reads")
                .count(),
            0,
            "promotion leaked route scratch state"
        );
        std::fs::remove_dir_all(root).expect("script temp tree removes");
    }
}
