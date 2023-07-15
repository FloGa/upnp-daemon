use std::error::Error;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};

use cidr_utils::cidr::Ipv4Cidr;
use igd::{AddPortError, Gateway, SearchOptions};
use log::{debug, error, info, warn};
use serde::Deserialize;

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

#[derive(Debug, Deserialize)]
pub struct UpnpConfig {
    pub address: Option<Ipv4Cidr>,
    pub port: u16,
    pub protocol: PortMappingProtocol,
    pub duration: u32,
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

pub fn add_ports(configs: impl Iterator<Item = anyhow::Result<UpnpConfig>>) {
    for config in configs {
        match config {
            Ok(config) => {
                info!("Add port: {:?}", config);
                if let Err(err) = config.add_port() {
                    error!("{}", err);
                }
            }
            Err(err) => {
                error!("{}", err);
            }
        }
    }
}

pub fn delete_ports(configs: impl Iterator<Item = anyhow::Result<UpnpConfig>>) {
    for config in configs {
        match config {
            Ok(config) => {
                info!("Remove port: {:?}", config);
                config.remove_port();
            }
            Err(err) => {
                error!("{}", err);
            }
        }
    }
}
