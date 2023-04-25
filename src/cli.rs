use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use clap::{
    builder::{PathBufValueParser, TypedValueParser},
    crate_name, Parser,
};
use csv::Reader;
#[cfg(unix)]
use daemonize::Daemonize;
use log::info;

use crate::{delete, run, Options};

fn get_csv_reader<P: AsRef<Path>>(file: P) -> csv::Result<Reader<File>> {
    return csv::ReaderBuilder::new().delimiter(b';').from_path(&file);
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long, short, value_parser = PathBufValueParser::new().try_map(|p| p.canonicalize()))]
    /// The file with the port descriptions, in CSV format
    file: PathBuf,

    #[cfg(unix)]
    #[arg(long, short = 'F')]
    /// Run in foreground instead of forking to background
    foreground: bool,

    #[arg(long, short = '1')]
    /// Run just one time instead of continuously
    oneshot: bool,

    #[arg(long, short = 'n', default_value_t = 60)]
    /// Specify update interval in seconds
    interval: u64,

    #[arg(long)]
    /// Close specified ports on program exit
    close_ports_on_exit: bool,

    #[arg(long)]
    /// Only close specified ports and exit
    only_close_ports: bool,
}

impl Cli {
    pub fn run() -> Result<(), Box<dyn Error>> {
        let cli = Cli::parse();

        #[cfg(unix)]
        if !cli.foreground {
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
            if !cli.only_close_ports {
                let mut rdr = get_csv_reader(&cli.file)?;

                for result in rdr.deserialize() {
                    let options: Options = result?;
                    info!("Processing: {:?}", options);
                    run(options)?;
                }
            }

            if cli.oneshot || cli.only_close_ports {
                tx_quitter.send(true)?;
            }

            match rx_quitter.recv_timeout(Duration::from_secs(cli.interval)) {
                Err(RecvTimeoutError::Timeout) => {
                    // Timeout reached without being interrupted, continue with loop
                }
                Err(e) => {
                    // Something bad happened
                    panic!("{}", e);
                }
                Ok(_) => {
                    // Quit signal received, break loop and quit nicely

                    if cli.close_ports_on_exit || cli.only_close_ports {
                        let mut rdr = get_csv_reader(&cli.file)?;

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
