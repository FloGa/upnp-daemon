use std::error::Error;
use std::io;

use upnp_daemon::run;

fn main() -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(io::stdin());

    for result in rdr.deserialize() {
        run(result?)?;
    }

    Ok(())
}
