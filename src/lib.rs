use std::error::Error;
use std::net::{SocketAddr, SocketAddrV4};

use igd::{AddPortError, Gateway, SearchOptions};
use serde::Deserialize;

mod cli;

pub use cli::Cli;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Options {
    pub address: Option<String>,
    pub port: u16,
    pub protocol: PortMappingProtocol,
    pub ttl: u32,
    pub comment: String,
}

fn find_gateway_with_bind_addr(bind_addr: SocketAddr) -> Gateway {
    let options = SearchOptions {
        bind_addr,
        ..Default::default()
    };
    igd::search_gateway(options).unwrap()
}

fn find_gateway_and_addr() -> (Gateway, SocketAddr) {
    let ifaces = get_if_addrs::get_if_addrs().unwrap();
    ifaces
        .iter()
        .filter_map(|iface| {
            if iface.is_loopback() || !iface.ip().is_ipv4() {
                None
            } else {
                let options = SearchOptions {
                    bind_addr: format!("{}:0", iface.addr.ip()).parse().unwrap(),
                    ..Default::default()
                };
                igd::search_gateway(options).ok().and_then(|gateway| {
                    if let get_if_addrs::IfAddr::V4(addr) = &iface.addr {
                        Some((gateway, SocketAddr::V4(SocketAddrV4::new(addr.ip, 0))))
                    } else {
                        unreachable!()
                    }
                })
            }
        })
        .next()
        .unwrap()
}

pub fn run(options: Options) -> Result<(), Box<dyn Error>> {
    let port = options.port;
    let protocol = options.protocol.into();
    let ttl = options.ttl;
    let comment = options.comment;

    let (gateway, addr) = match options.address {
        None => {
            let (gateway, mut addr) = find_gateway_and_addr();
            addr.set_port(port);

            let addr = match addr {
                SocketAddr::V4(addr) => addr,
                _ => panic!("No IPv4 given"),
            };

            (gateway, addr)
        }

        Some(addr) => {
            let addr = format!("{}:{}", addr, port).parse().unwrap();

            let gateway = find_gateway_with_bind_addr(addr);

            let addr = match addr {
                SocketAddr::V4(addr) => addr,
                _ => panic!("No IPv4 given"),
            };

            (gateway, addr)
        }
    };

    let f = || gateway.add_port(protocol, port, addr, ttl, &comment);
    f().or_else(|e| match e {
        AddPortError::PortInUse => {
            gateway.remove_port(protocol, port).unwrap();
            f()
        }
        e => Err(e),
    })?;

    Ok(())
}
