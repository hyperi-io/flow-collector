//! Configuration structures for Flow Collector.
//!
//! This module defines configuration structures for NetFlow collection,
//! data forwarding, and logging. It also provides deserialization logic
//! to parse these configurations from a YAML file.

use serde::{Deserialize, Deserializer};

/// Top-level configuration structure for Flow Collector.
#[derive(Debug, Deserialize, Clone)]
pub struct NetflowConfig {
    /// NetFlow-related settings.
    pub netflow: Netflow,
    /// Forwarding configuration.
    pub forwarding: Forwarding,
    /// Logging configuration.
    pub logging: Logging,
}

/// Configuration for NetFlow collection.
#[derive(Debug, Deserialize, Clone)]
pub struct Netflow {
    /// Whether NetFlow collection is enabled.
    pub enabled: bool,
    /// Address to bind the NetFlow listener.
    pub address: String,
    /// List of UDP ports to listen on for NetFlow traffic.
    pub ports: Vec<u16>,
}

/// Configuration for forwarding parsed NetFlow data.
#[derive(Debug, Clone)]
pub struct Forwarding {
    /// Destination address for forwarding data.
    pub address: String,
    /// Destination port for forwarding data.
    pub port: u16,
    /// Protocol to use for forwarding (`udp`, `tcp`, or `unix_stream`).
    pub protocol: Protocol,
    /// Unix socket path for `unix_stream` protocol.
    pub unix_socket_path: String,
}

/// Supported forwarding protocols.
#[derive(Debug, Clone)]
pub enum Protocol {
    Udp,
    Tcp,
    UnixStream,
    UnixDatagram,
}

/// Implements deserialization for `Protocol` enum to support YAML configuration.
impl<'de> Deserialize<'de> for Protocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "udp" => Ok(Protocol::Udp),
            "tcp" => Ok(Protocol::Tcp),
            "unix_stream" => Ok(Protocol::UnixStream),
            "unix_datagram" => Ok(Protocol::UnixDatagram),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid protocol: {}. Allowed values are 'udp', 'tcp', 'unix_stream', or 'unix_datagram'.",
                s
            ))),
        }
    }
}

/// Implements `PartialEq` to allow direct comparison between `Protocol` and `&str`.
impl PartialEq<&str> for Protocol {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Protocol::Udp => *other == "udp",
            Protocol::Tcp => *other == "tcp",
            Protocol::UnixStream => *other == "unix_stream",
            Protocol::UnixDatagram => *other == "unix_datagram",
        }
    }
}

/// Implements deserialization for `Forwarding` structure.
impl<'de> Deserialize<'de> for Forwarding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = std::collections::HashMap::<String, serde_yaml::Value>::deserialize(deserializer)?;

        // Extract and validate `port`
        let port = match map.remove("port") {
            Some(serde_yaml::Value::Number(num)) => {
                if let Some(port) = num.as_u64() {
                    if (2..=65535).contains(&port) {
                        port as u16
                    } else {
                        return Err(serde::de::Error::custom(format!(
                            "Port {} is out of valid range (2-65535).",
                            port
                        )));
                    }
                } else {
                    return Err(serde::de::Error::custom("Port must be a positive integer."));
                }
            }
            _ => return Err(serde::de::Error::custom("Missing or invalid 'port' field.")),
        };

        let protocol: Protocol = match map.remove("protocol") {
            Some(value) => serde_yaml::from_value(value).map_err(serde::de::Error::custom)?,
            None => return Err(serde::de::Error::custom("Missing 'protocol' field.")),
        };

        let address: String = match map.remove("address") {
            Some(serde_yaml::Value::String(addr)) => addr,
            _ => return Err(serde::de::Error::custom("Missing or invalid 'address' field.")),
        };

        let unix_socket_path: String = match map.remove("unix_socket_path") {
            Some(serde_yaml::Value::String(path)) => path,
            _ => "".to_string(),
        };

        Ok(Forwarding {
            address,
            port,
            protocol,
            unix_socket_path,
        })
    }
}

/// Configuration for logging settings.
#[derive(Debug, Deserialize, Clone)]
pub struct Logging {
    /// Path to the log file.
    pub file: String,
    /// Maximum log file size in bytes.
    pub maxsize: u64,
    /// Number of log files to retain.
    pub keep: usize,
}
