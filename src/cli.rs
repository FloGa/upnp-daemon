use std::error::Error;
use std::io;

use clap::{crate_authors, crate_description, crate_name, crate_version, App};

use crate::run;

pub struct Cli;

impl Cli {
    pub fn run() -> Result<(), Box<dyn Error>> {
        let _arguments = App::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .get_matches_safe()
            .unwrap_or_else(|e| e.exit());

        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(io::stdin());

        for result in rdr.deserialize() {
            run(result?)?;
        }

        Ok(())
    }
}
