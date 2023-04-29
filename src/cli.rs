use std::error::Error;
use std::fs::File;
use std::io::{stdin, BufReader, BufWriter, Seek};
use std::path::PathBuf;
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
use tempfile::tempfile;

use crate::UpnpConfig;

#[derive(Clone)]
enum CliInput {
    File(PathBuf),
    Stdin,
}

impl TryFrom<PathBuf> for CliInput {
    type Error = std::io::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Ok(if path == PathBuf::from("-") {
            CliInput::Stdin
        } else {
            CliInput::File(path.canonicalize()?)
        })
    }
}

enum CsvInput {
    File(File),
    PathBuf(PathBuf),
}

impl TryFrom<CliInput> for CsvInput {
    type Error = std::io::Error;

    fn try_from(cli_input: CliInput) -> Result<Self, Self::Error> {
        Ok(match cli_input {
            CliInput::File(pathbuf) => Self::PathBuf(pathbuf),
            CliInput::Stdin => {
                // Write contents of stdin to temporary file, so we can read it multiple times.
                let tempfile = tempfile()?;
                {
                    let mut reader = BufReader::new(stdin());
                    let mut writer = BufWriter::new(&tempfile);
                    std::io::copy(&mut reader, &mut writer)?;
                }
                Self::File(tempfile)
            }
        })
    }
}

fn get_csv_reader(csv_input: &CsvInput) -> Result<Reader<File>, std::io::Error> {
    let mut builder = csv::ReaderBuilder::new();
    let reader_builder = builder.delimiter(b';');

    Ok(match csv_input {
        CsvInput::File(file) => {
            // Clone file handle, so we don't move the original handle away.
            let mut file = file.try_clone()?;

            // File may have been advanced in previous iteration, so rewind it first.
            file.rewind()?;
            reader_builder.from_reader(file)
        }
        CsvInput::PathBuf(pathbuf) => reader_builder.from_path(pathbuf)?,
    })
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long, short, value_parser = PathBufValueParser::new().try_map(CliInput::try_from))]
    /// The file (or "-" for stdin) with the port descriptions, in CSV format
    file: CliInput,

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

        // Handle file here, because reading from stdin will fail in daemon mode.
        let file = cli.file.try_into()?;

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
                let mut rdr = get_csv_reader(&file)?;

                for result in rdr.deserialize() {
                    let config: UpnpConfig = result?;
                    info!("Processing: {:?}", config);
                    config.add_port()?;
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
                        let mut rdr = get_csv_reader(&file)?;

                        // Delete open port mappings
                        for result in rdr.deserialize() {
                            let config: UpnpConfig = result?;
                            info!("Deleting: {:?}", config);
                            config.remove_port();
                        }
                    }

                    break;
                }
            }
        }

        Ok(())
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
