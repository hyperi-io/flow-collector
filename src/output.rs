//! Forwarding Module
//!
//! Supports UDP, TCP, UnixStream (NDJSON), and UnixDatagram.

use crate::types;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{Mutex, OnceCell};

#[cfg(unix)]
use tokio::fs;
#[cfg(unix)]
use tokio::net::{UnixDatagram, UnixStream};

lazy_static! {
    static ref TCP_CONNECTIONS: Mutex<HashMap<String, TcpStream>> = Mutex::new(HashMap::new());
}

static UDP_SOCKET: OnceCell<Arc<UdpSocket>> = OnceCell::const_new();

#[cfg(unix)]
static UNIX_DGRAM_SOCKET: OnceCell<Arc<UnixDatagram>> = OnceCell::const_new();

#[cfg(unix)]
static UNIX_STREAM_CONN: OnceCell<Arc<Mutex<Option<UnixStream>>>> = OnceCell::const_new();

pub async fn get_udp_socket() -> Arc<UdpSocket> {
    UDP_SOCKET
        .get_or_init(|| async {
            UdpSocket::bind("0.0.0.0:0").await.map(Arc::new).expect("UDP bind failed")
        })
        .await
        .clone()
}

#[cfg(unix)]
pub async fn get_unix_dgram_socket() -> Arc<UnixDatagram> {
    UNIX_DGRAM_SOCKET
        .get_or_init(|| async {
            Arc::new(UnixDatagram::unbound().expect("UnixDatagram creation failed"))
        })
        .await
        .clone()
}

#[cfg(unix)]
async fn get_unix_stream_holder() -> Arc<Mutex<Option<UnixStream>>> {
    UNIX_STREAM_CONN
        .get_or_init(|| async { Arc::new(Mutex::new(None)) })
        .await
        .clone()
}

pub async fn forward_data(data: &str, forward_config: &types::Forwarding) {
    match forward_config.protocol {
        types::Protocol::Udp => {
            let addr = format!("{}:{}", forward_config.address, forward_config.port);
            let _ = get_udp_socket().await.send_to(data.as_bytes(), &addr).await;
        }
        types::Protocol::Tcp => {
            let addr = format!("{}:{}", forward_config.address, forward_config.port);
            reconnecting_tcp_forward(data, &addr).await;
        }
        #[cfg(unix)]
        types::Protocol::UnixStream => {
            let _ = unix_stream_send_ndjson(data, &forward_config.unix_socket_path).await;
        }
        #[cfg(unix)]
        types::Protocol::UnixDatagram => {
            let _ = unix_datagram_send(data, &forward_config.unix_socket_path).await;
        }
        #[cfg(not(unix))]
        types::Protocol::UnixStream | types::Protocol::UnixDatagram => {
            eprintln!(
                "Unix domain sockets are not supported on this operating system: {}",
                forward_config.unix_socket_path
            );
        }
    }
}

#[cfg(unix)]
pub async fn unix_datagram_send(data: &str, path: &str) -> Result<(), std::io::Error> {
    let sock = get_unix_dgram_socket().await;
    match sock.send_to(data.as_bytes(), path).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
pub async fn unix_datagram_send(_data: &str, _path: &str) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Unix datagram sockets are not supported on non-Unix platforms",
    ))
}

#[cfg(unix)]
pub async fn unix_stream_send_ndjson(data: &str, path: &str) -> Result<(), std::io::Error> {
    if fs::metadata(path).await.is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Socket missing"));
    }

    let holder = get_unix_stream_holder().await;
    let mut guard = holder.lock().await;

    if guard.is_none() {
        *guard = Some(UnixStream::connect(path).await?);
    }

    let msg = format!("{}\n", data);
    if let Some(stream) = guard.as_mut()
        && stream.write_all(msg.as_bytes()).await.is_ok()
    {
        return Ok(());
    }

    // Reconnect on failure
    *guard = None;
    if let Ok(mut s) = UnixStream::connect(path).await {
        s.write_all(msg.as_bytes()).await?;
        *guard = Some(s);
    }
    Ok(())
}

#[cfg(not(unix))]
pub async fn unix_stream_send_ndjson(_data: &str, _path: &str) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Unix stream sockets are not supported on non-Unix platforms",
    ))
}

pub async fn reconnecting_tcp_forward(data: &str, addr: &str) {
    let mut conns = TCP_CONNECTIONS.lock().await;

    if !conns.contains_key(addr) {
        if let Ok(s) = TcpStream::connect(addr).await {
            conns.insert(addr.to_string(), s);
        } else {
            return;
        }
    }

    let success = if let Some(s) = conns.get_mut(addr) {
        s.write_all(data.as_bytes()).await.is_ok()
    } else {
        false
    };

    if !success {
        conns.remove(addr);
    }
}
