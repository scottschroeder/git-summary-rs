use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use std::fmt;
use url;

use std::io;
use std::vec;

// TODO configurable
const TCP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SocketData {
    pub host: url::Host,
    pub port: u16,
}

impl fmt::Display for SocketData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl fmt::Debug for SocketData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

// TODO Create NetCache

impl ToSocketAddrs for SocketData {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        match self.host {
            url::Host::Domain(ref s) => (s.as_ref(), self.port).to_socket_addrs(),
            url::Host::Ipv4(ip) => (ip, self.port)
                .to_socket_addrs()
                .map(|v| v.collect::<Vec<_>>().into_iter()),
            url::Host::Ipv6(ip) => (ip, self.port)
                .to_socket_addrs()
                .map(|v| v.collect::<Vec<_>>().into_iter()),
        }
    }
}

pub fn tcp_check<T: ToSocketAddrs>(sd: T) -> bool {
    sd.to_socket_addrs()
        .and_then(|addrs| {
            for addr in addrs {
                trace!("Making connection to: {:?}", addr);
                let _ = TcpStream::connect_timeout(&addr, TCP_TIMEOUT)?;
                return Ok(true);
            }
            Ok(false)
        })
        .unwrap_or(false)
}
