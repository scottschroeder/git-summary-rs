use url;
use std::time::Duration;
use std::net::{TcpStream, ToSocketAddrs, SocketAddr};

use std::io;
use std::option;
use std::vec;

use std::collections::HashMap;

// TODO configurable
const TCP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SocketData {
    pub host: url::Host,
    pub port: u16,
}

// TODO display for SocketData

impl ToSocketAddrs for SocketData {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        match self.host {
            url::Host::Domain(ref s) => (s.as_ref(), self.port).to_socket_addrs(),
            url::Host::Ipv4(ip) => {
                (ip, self.port).to_socket_addrs()
                    .map(|v| v.collect::<Vec<_>>().into_iter())
            }
            url::Host::Ipv6(ip) => {
                (ip, self.port).to_socket_addrs()
                    .map(|v| v.collect::<Vec<_>>().into_iter())
            }
        }
    }
}

pub fn tcp_check(sd: &SocketData) -> bool {
    sd.to_socket_addrs()
        .and_then(|addrs| {
            for addr in addrs {
                trace!("Making connection to: {:?}", addr);
                let _ = TcpStream::connect_timeout(&addr, TCP_TIMEOUT)?;
                return Ok(true);
            }
            Ok(false)
        }).unwrap_or(false)
}
