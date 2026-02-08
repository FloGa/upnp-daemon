//! # easy-upnp
//!
//! [![badge github]][url github]
//! [![badge crates.io]][url crates.io]
//! [![badge docs.rs]][url docs.rs]
//! [![badge license]][url license]
//!
//! [//]: # (@formatter:off)
//! [badge github]: https://img.shields.io/badge/github-FloGa%2Fupnp--daemon-green
//! [badge crates.io]: https://img.shields.io/crates/v/easy-upnp
//! [badge docs.rs]: https://img.shields.io/docsrs/easy-upnp
//! [badge license]: https://img.shields.io/crates/l/easy-upnp
//!
//! [url github]: https://github.com/FloGa/upnp-daemon/crates/easy-upnp
//! [url crates.io]: https://crates.io/crates/easy-upnp
//! [url docs.rs]: https://docs.rs/easy-upnp
//! [url license]: https://github.com/FloGa/upnp-daemon/blob/develop/crates/easy-upnp/LICENSE
//! [//]: # (@formatter:on)
//!
//! Easily open and close UPnP ports.
//!
//! A minimalistic wrapper around [IGD] to open and close network ports via
//! [UPnP]. Mainly this library is used in the CLI application [`upnp-daemon`],
//! but it can also be used as a library in other crates that just want to open
//! and close ports with minimal possible configuration.
//!
//! [IGD]: https://docs.rs/igd/
//!
//! [UPnP]: https://en.wikipedia.org/wiki/Universal_Plug_and_Play
//!
//! [`upnp-daemon`]: https://github.com/FloGa/upnp-daemon
//!
//! ## Example
//!
//! Here is a hands-on example to demonstrate the usage. It will add some ports
//! and immediately remove them again.
//!
//! ```rust no_run
//! use std::error::Error;
//! use std::str::FromStr;
//! use log::error;
//! use easy_upnp::{add_ports, delete_ports, Ipv4Cidr, PortMappingProtocol, UpnpConfig};
//!
//! fn get_configs() -> Result<[UpnpConfig; 3], Box<dyn Error>> {
//!     let config_no_address = UpnpConfig {
//!         address: None,
//!         port: 80,
//!         protocol: PortMappingProtocol::TCP,
//!         duration: 3600,
//!         comment: "Webserver".to_string(),
//!     };
//!
//!     let config_specific_address = UpnpConfig {
//!         address: Some(Ipv4Cidr::from_str("192.168.0.10")?),
//!         port: 8080,
//!         protocol: PortMappingProtocol::TCP,
//!         duration: 3600,
//!         comment: "Webserver alternative".to_string(),
//!     };
//!
//!     let config_address_range = UpnpConfig {
//!         address: Some(Ipv4Cidr::from_str("192.168.0.0/24")?),
//!         port: 8081,
//!         protocol: PortMappingProtocol::TCP,
//!         duration: 3600,
//!         comment: "Webserver second alternative".to_string(),
//!     };
//!
//!     Ok([
//!         config_no_address,
//!         config_specific_address,
//!         config_address_range,
//!     ])
//! }
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     for result in add_ports(get_configs()?) {
//!         if let Err(err) = result {
//!             error!("{}", err);
//!         }
//!     }
//!
//!     for result in delete_ports(get_configs()?) {
//!         if let Err(err) = result {
//!             error!("{}", err);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]

use std::net::{IpAddr, SocketAddr, SocketAddrV4};

pub use cidr::Ipv4Cidr;
use igd_next::{Gateway, SearchOptions};
use log::{debug, info, warn};
use serde::Deserialize;
use thiserror::Error;

/// Convenience wrapper over all possible Errors
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("No matching gateway found")]
    NoMatchingGateway,

    #[error("Could not get interface address: {0}")]
    CannotGetInterfaceAddress(#[source] std::io::Error),

    #[error("Error adding port: {0}")]
    IgdAddPortError(#[from] igd_next::AddPortError),

    #[error("Error searching for gateway: {0}")]
    IgdSearchError(#[from] igd_next::SearchError),
}

type Result<R> = std::result::Result<R, Error>;

/// The protocol for which the given port will be opened. Possible values are
/// [`UDP`](PortMappingProtocol::UDP) and [`TCP`](PortMappingProtocol::TCP).
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum PortMappingProtocol {
    TCP,
    UDP,
}

impl From<PortMappingProtocol> for igd_next::PortMappingProtocol {
    fn from(proto: PortMappingProtocol) -> Self {
        match proto {
            PortMappingProtocol::TCP => igd_next::PortMappingProtocol::TCP,
            PortMappingProtocol::UDP => igd_next::PortMappingProtocol::UDP,
        }
    }
}

fn find_gateway_with_bind_addr(bind_addr: SocketAddr) -> Result<Gateway> {
    let options = SearchOptions {
        bind_addr,
        ..Default::default()
    };
    Ok(igd_next::search_gateway(options)?)
}

fn find_gateway_and_addr(cidr: &Option<Ipv4Cidr>) -> Result<(Gateway, SocketAddr)> {
    let ifaces = if_addrs::get_if_addrs().map_err(Error::CannotGetInterfaceAddress)?;

    let (gateway, address) = ifaces
        .iter()
        .filter_map(|iface| {
            if iface.is_loopback() || !iface.ip().is_ipv4() {
                None
            } else {
                let iface_ip = match iface.ip() {
                    IpAddr::V4(ip) => ip,
                    IpAddr::V6(_) => unreachable!(),
                };

                match cidr {
                    Some(cidr) if !cidr.contains(&iface_ip) => None,
                    Some(_) => {
                        let addr = SocketAddr::new(IpAddr::V4(iface_ip), 0);

                        let gateway = find_gateway_with_bind_addr(addr);

                        Some((gateway, addr))
                    }
                    _ => {
                        let options = SearchOptions {
                            // Unwrap is okay here, IP is correctly generated
                            bind_addr: format!("{}:0", iface.addr.ip()).parse().unwrap(),
                            ..Default::default()
                        };
                        igd_next::search_gateway(options).ok().and_then(|gateway| {
                            if let if_addrs::IfAddr::V4(addr) = &iface.addr {
                                Some((Ok(gateway), SocketAddr::V4(SocketAddrV4::new(addr.ip, 0))))
                            } else {
                                // Anything other than V4 has been ruled out by the first if
                                // condition.
                                unreachable!()
                            }
                        })
                    }
                }
            }
        })
        .next()
        .ok_or(Error::NoMatchingGateway)?;

    Ok((gateway?, address))
}

fn get_gateway_and_address_from_options(
    address: &Option<Ipv4Cidr>,
    port: u16,
) -> Result<(Gateway, SocketAddr)> {
    if let Some(addr) = address {
        if addr.is_host_address() {
            let sock_addr = SocketAddr::new(IpAddr::V4(addr.first_address()), port);
            let gateway = find_gateway_with_bind_addr(sock_addr)?;
            return Ok((gateway, sock_addr));
        }
    }

    let (gateway, mut addr) = find_gateway_and_addr(address)?;
    addr.set_port(port);
    Ok((gateway, addr))
}

/// This struct defines a configuration for a port mapping.
///
/// The configuration consists of all necessary pieces of information for a proper port opening.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use easy_upnp::{Ipv4Cidr, PortMappingProtocol, UpnpConfig};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config_no_address = UpnpConfig {
///     address: None,
///     port: 80,
///     protocol: PortMappingProtocol::TCP,
///     duration: 3600,
///     comment: "Webserver".to_string(),
/// };
///
/// let config_specific_address = UpnpConfig {
///     address: Some(Ipv4Cidr::from_str("192.168.0.10")?),
///     port: 80,
///     protocol: PortMappingProtocol::TCP,
///     duration: 3600,
///     comment: "Webserver".to_string(),
/// };
///
/// let config_address_range = UpnpConfig {
///     address: Some(Ipv4Cidr::from_str("192.168.0.0/24")?),
///     port: 80,
///     protocol: PortMappingProtocol::TCP,
///     duration: 3600,
///     comment: "Webserver".to_string(),
/// };
/// #
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Deserialize)]
pub struct UpnpConfig {
    /// The IP address for which the port mapping should be added.
    ///
    /// This field can be [None], in which case every connected interface will be tried, until one
    /// gateway reports success. Useful if the IP address is dynamic and not consistent over
    /// reboots.
    ///
    /// Fill in an IP address if you want to add a port mapping for a foreign device, or if you
    /// know your machine's address and want to slightly speed up the process.
    ///
    /// For examples how to specify IP addresses, check the documentation of [Ipv4Cidr].
    pub address: Option<Ipv4Cidr>,

    /// The port number to open for the given IP address.
    ///
    /// Note that we are greedy at the moment, if a port mapping is already in place, it will be
    /// deleted and re-added with the given IP address. This might be configurable in a future
    /// release.
    pub port: u16,

    /// The protocol for which the given port will be opened. Possible values are
    /// [`UDP`](PortMappingProtocol::UDP) and [`TCP`](PortMappingProtocol::TCP).
    pub protocol: PortMappingProtocol,

    /// The lease duration for the port mapping in seconds.
    ///
    /// Please note that some UPnP capable routers might choose to ignore this value, so do not
    /// exclusively rely on this.
    pub duration: u32,

    /// A comment about the reason for the port mapping.
    ///
    /// Will be stored together with the mapping in the router.
    pub comment: String,
}

impl UpnpConfig {
    fn remove_port(&self) -> Result<()> {
        let port = self.port;
        let protocol = self.protocol.into();

        let (gateway, _) = get_gateway_and_address_from_options(&self.address, port)?;

        gateway.remove_port(protocol, port).unwrap_or_else(|e| {
            warn!(
                "The following, non-fatal error appeared while deleting port {}:",
                port
            );
            warn!("{}", e);
        });

        Ok(())
    }

    fn add_port(&self) -> Result<()> {
        let port = self.port;
        let protocol = self.protocol.into();
        let duration = self.duration;
        let comment = &self.comment;

        let (gateway, addr) = get_gateway_and_address_from_options(&self.address, port)?;

        let f = || gateway.add_port(protocol, port, addr, duration, comment);
        f().or_else(|e| match e {
            igd_next::AddPortError::PortInUse => {
                debug!("Port already in use. Delete mapping.");
                gateway.remove_port(protocol, port).unwrap();
                debug!("Retry port mapping.");
                f()
            }
            e => Err(e),
        })?;

        Ok(())
    }
}

/// Add port mappings.
///
/// This function takes an iterable of [UpnpConfig]s and opens all configures ports.
///
/// Errors are logged, but otherwise ignored. An error during opening a port will not stop the
/// processing of the other ports.
///
/// # Example
///
/// ```no_run
/// use log::error;
/// use easy_upnp::{add_ports, PortMappingProtocol, UpnpConfig};
///
/// let config = UpnpConfig {
///     address: None,
///     port: 80,
///     protocol: PortMappingProtocol::TCP,
///     duration: 3600,
///     comment: "Webserver".to_string(),
/// };
///
/// for result in add_ports([config]) {
///     if let Err(err) = result {
///         error!("{}", err);
///     }
/// }
/// ```
pub fn add_ports(
    configs: impl IntoIterator<Item = UpnpConfig>,
) -> impl Iterator<Item = Result<()>> {
    configs.into_iter().map(|config| {
        info!("Add port: {:?}", config);
        config.add_port()
    })
}

/// Delete port mappings.
///
/// This function takes an iterable of [UpnpConfig]s and closes all configures ports.
///
/// Errors are logged, but otherwise ignored. An error during closing a port will not stop the
/// processing of the other ports.
///
/// # Example
///
/// ```no_run
/// use log::error;
/// use easy_upnp::{delete_ports, PortMappingProtocol, UpnpConfig};
///
/// let config = UpnpConfig {
///     address: None,
///     port: 80,
///     protocol: PortMappingProtocol::TCP,
///     duration: 3600,
///     comment: "Webserver".to_string(),
/// };
///
/// for result in delete_ports([config]) {
///     if let Err(err) = result {
///         error!("{}", err);
///     }
/// }
/// ```
pub fn delete_ports(
    configs: impl IntoIterator<Item = UpnpConfig>,
) -> impl Iterator<Item = Result<()>> {
    configs.into_iter().map(|config| {
        info!("Remove port: {:?}", config);
        config.remove_port()
    })
}
