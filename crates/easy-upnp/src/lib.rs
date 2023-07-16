//! # easy-upnp
//!
//! The business logic for upnp-daemon.

use std::error::Error;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};

use cidr_utils::cidr::Ipv4Cidr;
use igd::{AddPortError, Gateway, SearchOptions};
use log::{debug, error, info, warn};
use serde::Deserialize;

/// The protocol for which the given port will be opened. Possible values are
/// [`UDP`](PortMappingProtocol::UDP) and [`TCP`](PortMappingProtocol::TCP).
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum PortMappingProtocol {
    TCP,
    UDP,
}

impl From<PortMappingProtocol> for igd::PortMappingProtocol {
    fn from(proto: PortMappingProtocol) -> Self {
        match proto {
            PortMappingProtocol::TCP => igd::PortMappingProtocol::TCP,
            PortMappingProtocol::UDP => igd::PortMappingProtocol::UDP,
        }
    }
}

fn find_gateway_with_bind_addr(bind_addr: SocketAddr) -> Gateway {
    let options = SearchOptions {
        bind_addr,
        ..Default::default()
    };
    igd::search_gateway(options).unwrap()
}

fn find_gateway_and_addr(cidr: &Option<Ipv4Cidr>) -> (Gateway, SocketAddr) {
    let ifaces = get_if_addrs::get_if_addrs().unwrap();
    ifaces
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
                    Some(cidr) if !cidr.contains(iface_ip) => None,
                    Some(_) => {
                        let addr = SocketAddr::new(IpAddr::V4(iface_ip), 0);

                        let gateway = find_gateway_with_bind_addr(addr);

                        Some((gateway, addr))
                    }
                    _ => {
                        let options = SearchOptions {
                            bind_addr: format!("{}:0", iface.addr.ip()).parse().unwrap(),
                            ..Default::default()
                        };
                        igd::search_gateway(options).ok().and_then(|gateway| {
                            if let get_if_addrs::IfAddr::V4(addr) = &iface.addr {
                                Some((gateway, SocketAddr::V4(SocketAddrV4::new(addr.ip, 0))))
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
        .unwrap()
}

fn get_gateway_and_address_from_options(
    address: &Option<Ipv4Cidr>,
    port: u16,
) -> (Gateway, SocketAddrV4) {
    match address {
        Some(addr) if addr.get_bits() == 32 => {
            let addr = SocketAddr::new(IpAddr::V4(addr.get_prefix_as_ipv4_addr()), port);

            let gateway = find_gateway_with_bind_addr(addr);

            let addr = match addr {
                SocketAddr::V4(addr) => addr,
                _ => panic!("No IPv4 given"),
            };

            (gateway, addr)
        }

        _ => {
            let (gateway, mut addr) = find_gateway_and_addr(address);
            addr.set_port(port);

            let addr = match addr {
                SocketAddr::V4(addr) => addr,
                _ => panic!("No IPv4 given"),
            };

            (gateway, addr)
        }
    }
}

/// This struct defines a configuration for a port mapping.
///
/// The configuration consists of all necessary pieces of information for a proper port opening.
///
/// # Examples
///
/// ```
/// use cidr_utils::cidr::Ipv4Cidr;
///
/// use easy_upnp::{PortMappingProtocol, UpnpConfig};
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
///     address: Some(Ipv4Cidr::from_str("192.168.0.10/24")?),
///     port: 80,
///     protocol: PortMappingProtocol::TCP,
///     duration: 3600,
///     comment: "Webserver".to_string(),
/// };
///
/// let config_address_range = UpnpConfig {
///     address: Some(Ipv4Cidr::from_str("192.168.0")?),
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
    fn remove_port(&self) {
        let port = self.port;
        let protocol = self.protocol.into();

        let (gateway, _) = get_gateway_and_address_from_options(&self.address, port);

        gateway.remove_port(protocol, port).unwrap_or_else(|e| {
            warn!(
                "The following, non-fatal error appeared while deleting port {}:",
                port
            );
            warn!("{}", e);
        });
    }

    fn add_port(&self) -> Result<(), Box<dyn Error>> {
        let port = self.port;
        let protocol = self.protocol.into();
        let duration = self.duration;
        let comment = &self.comment;

        let (gateway, addr) = get_gateway_and_address_from_options(&self.address, port);

        let f = || gateway.add_port(protocol, port, addr, duration, comment);
        f().or_else(|e| match e {
            AddPortError::PortInUse => {
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
/// add_ports([config]);
/// ```
pub fn add_ports(configs: impl IntoIterator<Item = UpnpConfig>) {
    for config in configs {
        info!("Add port: {:?}", config);
        if let Err(err) = config.add_port() {
            error!("{}", err);
        }
    }
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
/// delete_ports([config]);
/// ```
pub fn delete_ports(configs: impl IntoIterator<Item = UpnpConfig>) {
    for config in configs {
        info!("Remove port: {:?}", config);
        config.remove_port();
    }
}
