# easy-upnp

[![badge github]][url github]
[![badge crates.io]][url crates.io]
[![badge docs.rs]][url docs.rs]
[![badge license]][url license]

[badge github]: https://img.shields.io/badge/github-FloGa%2Fupnp--daemon-green
[badge crates.io]: https://img.shields.io/crates/v/easy-upnp
[badge docs.rs]: https://img.shields.io/docsrs/easy-upnp
[badge license]: https://img.shields.io/crates/l/easy-upnp

[url github]: https://github.com/FloGa/upnp-daemon/crates/easy-upnp
[url crates.io]: https://crates.io/crates/easy-upnp
[url docs.rs]: https://docs.rs/easy-upnp
[url license]:
https://github.com/FloGa/upnp-daemon/blob/develop/crates/easy-upnp/LICENSE

A minimalistic wrapper around [IGD] to open and close network ports via
[UPnP]. Mainly this library is used in the CLI application [`upnp-daemon`],
but it can also be used as a library in other crates that just want to open
and close ports with minimal possible configuration.

[IGD]: https://docs.rs/igd/
[UPnP]: https://en.wikipedia.org/wiki/Universal_Plug_and_Play
[`upnp-daemon`]: https://github.com/FloGa/upnp-daemon

## Example

Here is a hands-on example to demonstrate the usage. It will add some ports and immediately remove them again.

```rust no_run
use std::error::Error;

use cidr_utils::cidr::Ipv4Cidr;
use easy_upnp::{add_ports, delete_ports, PortMappingProtocol, UpnpConfig};

fn get_configs() -> Result<[UpnpConfig; 3], Box<dyn Error>> {
    let config_no_address = UpnpConfig {
        address: None,
        port: 80,
        protocol: PortMappingProtocol::TCP,
        duration: 3600,
        comment: "Webserver".to_string(),
    };

    let config_specific_address = UpnpConfig {
        address: Some(Ipv4Cidr::from_str("192.168.0.10/24")?),
        port: 8080,
        protocol: PortMappingProtocol::TCP,
        duration: 3600,
        comment: "Webserver alternative".to_string(),
    };

    let config_address_range = UpnpConfig {
        address: Some(Ipv4Cidr::from_str("192.168.0")?),
        port: 8081,
        protocol: PortMappingProtocol::TCP,
        duration: 3600,
        comment: "Webserver second alternative".to_string(),
    };

    Ok([
        config_no_address,
        config_specific_address,
        config_address_range,
    ])
}

fn main() -> Result<(), Box<dyn Error>> {
    add_ports(get_configs()?);

    delete_ports(get_configs()?);

    Ok(())
}
```
