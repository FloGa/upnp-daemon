use std::path::PathBuf;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref BIN_PATH: PathBuf = assert_cmd::cargo::cargo_bin!().to_path_buf();
    pub static ref BIN_NAME: String = assert_cmd::pkg_name!().to_string();
}
