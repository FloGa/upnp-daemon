use std::path::PathBuf;

use assert_cmd::Command;
use lazy_static::lazy_static;
use predicates::prelude::*;

lazy_static! {
    static ref BIN_PATH: PathBuf = assert_cmd::cargo::cargo_bin("upnp-daemon");
}

#[test]
fn help_works() {
    Command::new(&*BIN_PATH)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("\nUsage:"));
}

#[test]
fn correct_version() {
    let version = env!("CARGO_PKG_VERSION");

    Command::new(&*BIN_PATH)
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("{} {}\n", "upnp-daemon", version));
}

#[test]
fn empty_csv_input_passes() {
    Command::new(&*BIN_PATH).arg("-1Ff-").assert().success();
}

#[test]
fn empty_json_input_fails() {
    Command::new(&*BIN_PATH)
        .arg("-1Ff-")
        .arg("--format=json")
        .assert()
        .failure();
}

#[test]
fn empty_json_array_input_passes() {
    Command::new(&*BIN_PATH)
        .arg("-1Ff-")
        .arg("--format=json")
        .write_stdin("[]")
        .assert()
        .success();
}
