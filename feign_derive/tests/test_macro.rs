use failure::Error;
use serde::{
    Deserialize,
    Serialize,
};

use feign_derive::{feign, put};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RegisterService {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub port: u16,
    pub address: String,
}

#[feign("v1/agent/", id = "consul", port = 8500)]
trait IClient {
    #[put("service/register", json = "rs")]
    fn register(&self, rs: &RegisterService) -> Result<(), Error>;

    fn test(&self) {
        println!("test default");
    }
}

#[cfg(test)]
mod tests {
    use std::net::{
        IpAddr,
        SocketAddr,
    };
    use std::str::FromStr;

    use super::{
        build_iclient,
        IClient,
        RegisterService,
    };

    #[test]
    fn it_works() {
        let ip = IpAddr::from([127, 0, 0, 1]);
        let socket_addr = SocketAddr::new(ip, 8600);
        let client = build_iclient(socket_addr).expect("haha");


        let addr = std::net::SocketAddr::from_str("127.0.0.1:8080").expect("");
        let service = RegisterService {
            id: "xxxx".to_string(),
            name: "xxxx".to_string(),
            tags: vec![],
            port: addr.port(),
            address: "127.0.0.1".to_string(),
        };
        client.register(&service).expect("");


        assert_eq!(2 + 2, 4);
    }
}