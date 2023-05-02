use std::error::Error;
use std::fs::File;
use std::io::{stdin, BufReader, BufWriter, Seek};
use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use anyhow::anyhow;
use clap::{
    builder::{PathBufValueParser, TypedValueParser},
    Parser, ValueEnum,
};
use csv::Reader;
#[cfg(unix)]
use daemonize::Daemonize;
use serde_json::Value;
use tempfile::tempfile;

use crate::{add_ports, delete_ports, UpnpConfig};

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

enum Input {
    File(File),
    PathBuf(PathBuf),
}

impl TryFrom<CliInput> for Input {
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

fn get_csv_reader(input: &Input) -> Result<Reader<File>, std::io::Error> {
    let mut builder = csv::ReaderBuilder::new();
    let reader_builder = builder.delimiter(b';');

    Ok(match input {
        Input::File(file) => {
            // Clone file handle, so we don't move the original handle away.
            let mut file = file.try_clone()?;

            // File may have been advanced in previous iteration, so rewind it first.
            file.rewind()?;
            reader_builder.from_reader(file)
        }
        Input::PathBuf(pathbuf) => reader_builder.from_path(pathbuf)?,
    })
}

fn get_configs_from_csv_reader(
    reader: &mut Reader<File>,
) -> impl Iterator<Item = anyhow::Result<UpnpConfig>> + '_ {
    reader
        .deserialize()
        .map(|result| result.map_err(anyhow::Error::from))
}

fn get_configs_from_json(
    input: &Input,
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<UpnpConfig>> + '_> {
    let file = match input {
        Input::File(file) => {
            // Clone file handle, so we don't move the original handle away.
            let mut file = file.try_clone()?;

            // File may have been advanced in previous iteration, so rewind it first.
            file.rewind()?;
            file
        }
        Input::PathBuf(pathbuf) => File::open(pathbuf)?,
    };

    let v: Value = serde_json::from_reader(file)?;

    if !v.is_array() {
        return Err(anyhow!("Input is not a JSON array"));
    }

    Ok(if let Value::Array(v) = v {
        v.into_iter()
            .map(|v| serde_json::from_value::<UpnpConfig>(v).map_err(anyhow::Error::from))
    } else {
        unreachable!()
    })
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CliInputFormat {
    Csv,
    Json,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long, short, value_parser = PathBufValueParser::new().try_map(CliInput::try_from))]
    /// The file (or "-" for stdin) with the port descriptions
    file: CliInput,

    #[arg(long, value_enum, default_value_t = CliInputFormat::Csv)]
    /// The format of the configuration file
    format: CliInputFormat,

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

    #[cfg(unix)]
    #[arg(long, default_value = "/tmp/upnp-daemon.pid")]
    /// Absolute path to PID file for daemon mode
    pid_file: PathBuf,
}

impl Cli {
    pub fn run() -> Result<(), Box<dyn Error>> {
        let cli = Cli::parse();

        // Handle file here, because reading from stdin will fail in daemon mode.
        let file = cli.file.try_into()?;

        #[cfg(unix)]
        if !cli.foreground {
            Daemonize::new()
                .pid_file(cli.pid_file)
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
                match cli.format {
                    CliInputFormat::Csv => {
                        let mut rdr = get_csv_reader(&file)?;
                        let configs = get_configs_from_csv_reader(&mut rdr);
                        add_ports(configs);
                    }
                    CliInputFormat::Json => {
                        let configs = get_configs_from_json(&file)?;
                        add_ports(configs);
                    }
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
                        match cli.format {
                            CliInputFormat::Csv => {
                                let mut rdr = get_csv_reader(&file)?;
                                let configs = get_configs_from_csv_reader(&mut rdr);
                                delete_ports(configs);
                            }
                            CliInputFormat::Json => {
                                let configs = get_configs_from_json(&file)?;
                                delete_ports(configs);
                            }
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
