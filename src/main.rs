use std::error::Error;

use upnp_daemon::Cli;

fn main() -> Result<(), Box<dyn Error>> {
    Cli::run()?;

    Ok(())
}
