use assert_cmd::Command;
use predicates::prelude::*;

mod common;

#[test]
fn help_works() {
    Command::new(&*common::BIN_PATH)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("\nUsage:"));
}

#[test]
fn correct_version() {
    let version = env!("CARGO_PKG_VERSION");

    Command::new(&*common::BIN_PATH)
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("{} {}\n", &*common::BIN_NAME, version));
}

#[test]
fn empty_csv_input_passes() {
    let mut command = Command::new(&*common::BIN_PATH);
    command.arg("-1f-");

    #[cfg(unix)]
    command.arg("-F");

    command.assert().success();
}

#[test]
fn empty_json_input_fails() {
    let mut command = Command::new(&*common::BIN_PATH);
    command.arg("-1f-").arg("--format=json");

    #[cfg(unix)]
    command.arg("-F");

    command.assert().failure();
}

#[test]
fn empty_json_array_input_passes() {
    let mut command = Command::new(&*common::BIN_PATH);
    command.arg("-1f-").arg("--format=json");

    #[cfg(unix)]
    command.arg("-F");

    command.write_stdin("[]").assert().success();
}
