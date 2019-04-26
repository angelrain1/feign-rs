use std::net::SocketAddr;

use failure::Error;
use trust_dns_resolver::{
    config::{
        NameServerConfig,
        Protocol,
        ResolverConfig,
        ResolverOpts,
    },
    Resolver,
};

pub fn build(socket_addr: SocketAddr) -> Result<Resolver, Error> {
    let name_server_config = NameServerConfig {
        socket_addr,
        protocol: Protocol::Udp,
        tls_dns_name: None,
    };
    let mut consul_resolver_config = ResolverConfig::new();

    consul_resolver_config.add_name_server(name_server_config);

    Ok(Resolver::new(consul_resolver_config, ResolverOpts::default())?)
}

