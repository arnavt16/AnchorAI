//! Runs the labeled fixtures in evals/fixtures/safety_fixtures.json against
//! the local pre-filter classifier. See that file's `note` field for what
//! this test suite does and does not establish.

use anchor_lib::safety::{classify, DistressLevel};
use serde::Deserialize;

#[derive(Deserialize)]
struct FixtureFile {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    expected: String,
}

fn load_fixtures() -> FixtureFile {
    let raw = include_str!("../../evals/fixtures/safety_fixtures.json");
    serde_json::from_str(raw).expect("safety_fixtures.json must be valid")
}

fn level_str(level: DistressLevel) -> &'static str {
    match level {
        DistressLevel::None => "none",
        DistressLevel::Elevated => "elevated",
        DistressLevel::Imminent => "imminent",
    }
}

#[test]
fn safety_fixtures_match_expected_levels() {
    let file = load_fixtures();
    let mut failures = vec![];
    for case in &file.cases {
        let got = level_str(classify(&case.text));
        if got != case.expected {
            failures.push(format!("{}: expected {}, got {}", case.id, case.expected, got));
        }
    }
    assert!(failures.is_empty(), "safety fixture mismatches:\n{}", failures.join("\n"));
}

#[test]
fn figurative_kill_language_does_not_trigger_imminent() {
    assert_eq!(classify("this deadline is going to kill me"), DistressLevel::None);
}

#[test]
fn direct_statement_triggers_imminent() {
    assert_eq!(classify("I want to kill myself tonight, I have a plan"), DistressLevel::Imminent);
}
