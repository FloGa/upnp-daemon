use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use clap::{crate_authors, crate_description, crate_name, crate_version, value_t, App, Arg};
use csv::Reader;
#[cfg(unix)]
use daemonize::Daemonize;
use log::info;

use crate::{delete, run, Options};

const ARG_FILE: &str = "file";
#[cfg(unix)]
const ARG_FOREGROUND: &str = "foreground";
const ARG_ONESHOT: &str = "oneshot";
const ARG_INTERVAL: &str = "interval";
const ARG_CLOSE_ON_EXIT: &str = "close-ports-on-exit";

fn get_csv_reader<P: AsRef<Path>>(file: P) -> csv::Result<Reader<File>> {
    return csv::ReaderBuilder::new().delimiter(b';').from_path(&file);
}

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
                #[cfg(unix)]
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
                Arg::with_name(ARG_CLOSE_ON_EXIT)
                    .long(ARG_CLOSE_ON_EXIT)
                    .help("Close specified ports on program exit"),
            ])
            .get_matches_safe()
            .unwrap_or_else(|e| e.exit());

        let file = fs::canonicalize(arguments.value_of_os(ARG_FILE).unwrap())?;
        #[cfg(unix)]
        let foreground = arguments.is_present(ARG_FOREGROUND);
        let oneshot = arguments.is_present(ARG_ONESHOT);
        let interval = if arguments.is_present(ARG_INTERVAL) {
            value_t!(arguments.value_of(ARG_INTERVAL), u64).unwrap_or_else(|e| e.exit())
        } else {
            60
        };
        let close_on_exit = arguments.is_present(ARG_CLOSE_ON_EXIT);

        #[cfg(unix)]
        if !foreground {
            Daemonize::new()
                .pid_file(format!("/tmp/{}.pid", crate_name!()))
                .start()
                .expect("Failed to daemonize.");
        }

        let (tx_quitter, rx_quitter) = channel();

        {
            let tx_quitter = tx_quitter.clone();
            ctrlc::set_handler(move || {
                tx_quitter.send(true).unwrap();
            })
            .expect("Error setting Ctrl-C handler");
        }

        loop {
            let mut rdr = get_csv_reader(&file)?;

            for result in rdr.deserialize() {
                let options: Options = result?;
                info!("Processing: {:?}", options);
                run(options)?;
            }

            if oneshot {
                tx_quitter.send(true)?;
            }

            match rx_quitter.recv_timeout(Duration::from_secs(interval)) {
                Err(RecvTimeoutError::Timeout) => {
                    // Timeout reached without being interrupted, continue with loop
                }
                Err(e) => {
                    // Something bad happened
                    panic!("{}", e);
                }
                Ok(_) => {
                    // Quit signal received, break loop and quit nicely

                    if close_on_exit {
                        let mut rdr = get_csv_reader(&file)?;

                        // Delete open port mappings
                        for result in rdr.deserialize() {
                            let options: Options = result?;
                            info!("Deleting: {:?}", options);
                            delete(options);
                        }
                    }

                    break;
                }
            }
        }

        Ok(())
    }
}
