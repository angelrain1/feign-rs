use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crossbeam_epoch::{
    self as epoch,
    Atomic,
    Owned,
};
use failure::Error;
use reqwest::Client;
use trust_dns_resolver::{
    lookup::SrvLookup,
    proto::rr::RecordType::SRV,
    Resolver,
};

pub mod consul;

fn lookup(resolver: &Resolver, host: &str) -> Result<(Instant, Vec<(IpAddr, u16)>), Error> {
    let lookup = resolver.lookup(host, SRV)?;

    let instant = lookup.valid_until();

    let lookup: SrvLookup = lookup.into();

    let addrs: Vec<(IpAddr, u16)> = lookup.iter()
        .zip(lookup.ip_iter())
        .map(|(srv, ip)| (ip, srv.port()))
        .collect();

    Ok((instant, addrs))
}

pub struct FeignClient<AddrResolver = Arc<Resolver>, HttpClient = Client> {
    resolver: AddrResolver,
    pub client: HttpClient,
    host: &'static str,
    next: AtomicUsize,
    cache: Atomic<(Instant, Vec<(IpAddr, u16)>)>,
}

pub struct Builder<AddrResolver, HttpClient> {
    resolver: AddrResolver,
    client: HttpClient,
}

impl FeignClient {
    pub fn builder(socket_addr: SocketAddr) -> Result<Builder<Arc<Resolver>, Client>, Error> {
        let resolver = Arc::new(consul::build(socket_addr)?);
        let client = Client::builder().build()?;

        Ok(Builder {
            resolver,
            client,
        })
    }

    pub fn next_addr(&self) -> Result<(IpAddr, u16), Error> {
        let now = Instant::now();
        let guard = &epoch::pin();
        let shared = self.cache.load_consume(guard);
        unsafe {
            let cache = shared.deref();
            if &now > &cache.0 {
                // cache is valid
                let vec = &cache.1;
                return Ok(vec[self.next.fetch_add(1, Ordering::Release) % vec.len()]);
            }
        }

        let new_cache = lookup(&self.resolver, self.host)?;
        let vec = &new_cache.1;
        let result = vec[self.next.fetch_add(1, Ordering::Release) % vec.len()];
        self.cache.store(Owned::new(new_cache), Ordering::Release);
        Ok(result)
    }
}

impl Builder<Arc<Resolver>, Client> {
    pub fn build(&self, host: &'static str) -> Result<FeignClient, Error> {
        let resolver = Arc::clone(&self.resolver);
        let client = self.client.clone();
        let cache = Atomic::new(lookup(&resolver, host)?);

        Ok(FeignClient {
            resolver,
            client,
            host,
            next: AtomicUsize::new(0),
            cache,
        })
    }
}



