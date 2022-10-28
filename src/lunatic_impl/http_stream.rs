use std::io::{Read, Write};

use lunatic::net::{TcpStream, TlsStream};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::Kind;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum HttpStream {
    Tcp(TcpStream),
    Tls(TlsStream),
}

impl HttpStream {
    pub fn connect(url: Url) -> crate::Result<HttpStream> {
        let protocol = url.scheme();
        if protocol == "https" {
            let conn_str = format!("{}", url.host().unwrap());
            return match TlsStream::connect(&conn_str, url.port().unwrap_or(443).into()) {
                Ok(stream) => Ok(HttpStream::Tls(stream)),
                Err(e) => {
                    lunatic_log::error!("Failed to connect via TLS {:?}", e);
                    Err(crate::Error::new(
                        Kind::Builder,
                        Some("Failed to connect".to_string()),
                    ))
                }
            };
        }
        let conn_str = format!("{}:{}", url.host().unwrap(), url.port().unwrap_or(80));
        lunatic_log::debug!("Connecting {:?} | {:?}", protocol, conn_str);
        match TcpStream::connect(conn_str) {
            Ok(stream) => Ok(HttpStream::Tcp(stream)),
            Err(e) => {
                lunatic_log::error!("Failed to connect via TCP {:?}", e);
                Err(crate::Error::new(
                    Kind::Builder,
                    Some("Failed to connect".to_string()),
                ))
            }
        }
    }
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            HttpStream::Tcp(stream) => stream.read(buf),
            HttpStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for HttpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            HttpStream::Tcp(stream) => stream.write(buf),
            HttpStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            HttpStream::Tcp(stream) => stream.flush(),
            HttpStream::Tls(stream) => stream.flush(),
        }
    }
}
