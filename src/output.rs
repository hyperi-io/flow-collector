//! Forwarding Module
//!
//! Supports UDP, TCP, UnixStream (NDJSON), and UnixDatagram.

use crate::types;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket, UnixDatagram, UnixStream};
use tokio::sync::{Mutex, OnceCell};

lazy_static! {
    static ref TCP_CONNECTIONS: Mutex<HashMap<String, TcpStream>> = Mutex::new(HashMap::new());
}

static UDP_SOCKET: OnceCell<Arc<UdpSocket>> = OnceCell::const_new();
static UNIX_DGRAM_SOCKET: OnceCell<Arc<UnixDatagram>> = OnceCell::const_new();
static UNIX_STREAM_CONN: OnceCell<Arc<Mutex<Option<UnixStream>>>> = OnceCell::const_new();

pub async fn get_udp_socket() -> Arc<UdpSocket> {
    UDP_SOCKET
        .get_or_init(|| async {
            UdpSocket::bind("0.0.0.0:0").await.map(Arc::new).expect("UDP bind failed")
        })
        .await
        .clone()
}

pub async fn get_unix_dgram_socket() -> Arc<UnixDatagram> {
    UNIX_DGRAM_SOCKET
        .get_or_init(|| async {
            Arc::new(UnixDatagram::unbound().expect("UnixDatagram creation failed"))
        })
        .await
        .clone()
}

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
        types::Protocol::UnixStream => {
            let _ = unix_stream_send_ndjson(data, &forward_config.unix_socket_path).await;
        }
        types::Protocol::UnixDatagram => {
            let _ = unix_datagram_send(data, &forward_config.unix_socket_path).await;
        }
    }
}

pub async fn unix_datagram_send(data: &str, path: &str) -> Result<(), std::io::Error> {
    let sock = get_unix_dgram_socket().await;
    match sock.send_to(data.as_bytes(), path).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

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
        && stream.write_all(msg.as_bytes()).await.is_ok() {
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

pub async fn reconnecting_tcp_forward(data: &str, addr: &str) {
    let mut conns = TCP_CONNECTIONS.lock().await;

    if !conns.contains_key(addr) {
        if let Ok(s) = TcpStream::connect(addr).await {
            conns.insert(addr.to_string(), s);
        } else { return; }
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