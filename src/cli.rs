use std::error::Error;
use std::time::Duration;
use std::{fs, thread};

use clap::{crate_authors, crate_description, crate_name, crate_version, value_t, App, Arg};
use daemonize::Daemonize;
use log::info;

use crate::{run, Options};

const ARG_FILE: &str = "file";
const ARG_FOREGROUND: &str = "foreground";
const ARG_ONESHOT: &str = "oneshot";
const ARG_INTERVAL: &str = "interval";

pub struct Cli;

impl Cli {
    pub fn run() -> Result<(), Box<dyn Error>> {
        let arguments = App::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .args(&[
                Arg::with_name(ARG_FILE)
                    .short(&ARG_FILE[0..1])
                    .long(ARG_FILE)
                    .help("The file with the port descriptions, in CSV format")
                    .required(true)
                    .takes_value(true)
                    .number_of_values(1),
                Arg::with_name(ARG_FOREGROUND)
                    .short(&ARG_FOREGROUND[0..1].to_uppercase())
                    .long(ARG_FOREGROUND)
                    .help("Run in foreground instead of forking to background"),
                Arg::with_name(ARG_ONESHOT)
                    .short("1")
                    .long(ARG_ONESHOT)
                    .help("Run just one time instead of continuously"),
                Arg::with_name(ARG_INTERVAL)
                    .short("n")
                    .long(ARG_INTERVAL)
                    .help("Specify update interval in seconds")
                    .takes_value(true)
                    .number_of_values(1),
            ])
            .get_matches_safe()
            .unwrap_or_else(|e| e.exit());

        let file = fs::canonicalize(arguments.value_of_os(ARG_FILE).unwrap())?;
        let foreground = arguments.is_present(ARG_FOREGROUND);
        let oneshot = arguments.is_present(ARG_ONESHOT);
        let interval = if arguments.is_present(ARG_INTERVAL) {
            value_t!(arguments.value_of(ARG_INTERVAL), u64).unwrap_or_else(|e| e.exit())
        } else {
            60
        };

        if !foreground {
            Daemonize::new()
                .pid_file(format!("/tmp/{}.pid", crate_name!()))
                .start()
                .expect("Failed to daemonize.");
        }

        loop {
            let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_path(&file)?;

            for result in rdr.deserialize() {
                let options: Options = result?;
                info!("Processing: {:?}", options);
                run(options)?;
            }

            if oneshot {
                break;
            } else {
                thread::sleep(Duration::from_secs(interval));
            }
        }

        Ok(())
    }
}
