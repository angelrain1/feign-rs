use std::net::{
    IpAddr,
    SocketAddr,
};

fn main() {
    let ip = IpAddr::from([127, 0, 0, 1]);
    let resolver = feign::consul::build(SocketAddr::new(ip, 8600)).expect("haha");

    use trust_dns_resolver::{
        proto::rr::RecordType::SRV,
        lookup::SrvLookup,
    };

    let lookup = resolver.lookup("xxxx.service.consul", SRV).expect("hehe");

    let _instant = dbg!(lookup.valid_until());

    let lookup: SrvLookup = lookup.into();

    let zip = lookup.iter().zip(lookup.ip_iter());

    for (srv, ip) in zip {
        println!("{}", srv.port());
        println!("{}", ip);
    }
}