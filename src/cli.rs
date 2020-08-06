use std::error::Error;

use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};

use crate::run;

const ARG_FILE: &str = "file";

pub struct Cli;

impl Cli {
    pub fn run() -> Result<(), Box<dyn Error>> {
        let arguments = App::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .args(&[Arg::with_name(ARG_FILE)
                .short(&ARG_FILE[0..1])
                .long(ARG_FILE)
                .help("The file with the port descriptions, in CSV format")
                .required(true)
                .takes_value(true)
                .number_of_values(1)])
            .get_matches_safe()
            .unwrap_or_else(|e| e.exit());

        let file = arguments.value_of_os(ARG_FILE).unwrap();

        let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_path(file)?;

        for result in rdr.deserialize() {
            run(result?)?;
        }

        Ok(())
    }
}
