# Flow Collector

## Overview
A Rust-based NetFlow collector using the netflow_parser create. If you find this useful please
support the crate author:
- [Michael Miles Mileusnich](https://ko-fi.com/michaelmileusnich)

The **Flow Collector** is a high-performance Rust-based NetFlow collector designed to:
- Capture NetFlow packets from network devices.
- Parse NetFlow data and optionally convert it to JSON.
- Forward data to different protocols such as **UDP**, **TCP**, **Unix datagram sockets**, or **Unix stream sockets**.
- Support multiple listeners on different ports.
- Provide graceful shutdown and logging features.

This tool is useful for **network monitoring, traffic analysis, and real-time event forwarding**.

## Features
✅ High-performance UDP packet processing using **Tokio**  
✅ Supports forwarding NetFlow data via **UDP, TCP, Unix datagram Sockets and Unix stream Sockets**  
✅ Configurable via **YAML configuration file**  
✅ Efficient connection handling using **async networking**  
✅ Logs events with **flexi_logger** for debugging and monitoring  
✅ Supports **graceful shutdown** via Ctrl+C

## Installation

### Prerequisites
- **Rust and Cargo** (if not installed, follow [Rust installation guide](https://www.rust-lang.org/tools/install))
- **Linux/macOS/Windows** with network access

### Build from Source
```shell
# Clone the repository
git clone https://github.com/hyperi-io/flow-collector.git
cd flow-collector

# Build the project
cargo build --release

# Run the binary
./flow-collector --config-file config.yaml
```

## Configuration
The Flow Collector is configured using a YAML file. Below is an example configuration:

```config.yaml``` __Example__:
```yaml
netflow:
  enabled: true               # Enable or disable NetFlow collection
  address: "0.0.0.0"         # Address to bind NetFlow listener
  ports: [2055, 9995]        # List of ports to listen for NetFlow packets

forwarding:
  address: "127.0.0.1"      # Destination address to forward data
  port: 9000                # Destination port
  protocol: "tcp"           # Supported: "udp", "tcp", "unix_datagram", "unix_stream"
  unix_socket_path: "/tmp/netflow.sock" # Required if using Unix datagram or Unix stream

logging:
  file: "netflow.log"       # Log file name
  maxsize: 10485760         # Max log file size (bytes)
  keep: 5                   # Number of log files to keep
```

### Configuration Options

| Section | Key | Description                                                         |
| ------- | --- |---------------------------------------------------------------------|
| netflow | enabled | Enable or disable NetFlow listening.                                |
| | address | IP address to bind the listener (e.g., 0.0.0.0 for all interfaces). |
| | ports | List of UDP ports to listen on for NetFlow packets. |
| forwarding | address | Target IP address for forwarding parsed NetFlow data. |
| | port | Target port for forwarding. |
| | protocol | Choose "udp", "tcp", "unix_datagram" or "unix_stream" as the forwarding method. |
| | unix_socket_path | Unix socket path (required if protocol is unix_stream). |
| logging | file | Log file name. |
| | maxsize | Maximum log file size (in bytes). |
| | keep | Number of log files to retain. |

## Usage
### Running the Daemon
To run Flow Collector with a custom configuration file:

```shell
./flow-collector --config-file config.yaml
```

If no configuration file is provided, it defaults to config.yaml in the current directory.

### Stopping the Daemon
Press Ctrl+C to gracefully shut down the daemon.

## Forwarding Modes
The daemon supports __three forwarding modes__:

1. UDP Forwarding:

   * Data is sent over UDP to the configured address/port.
   * Suitable for low-latency but unreliable transmission.


2. TCP Forwarding:

   * Data is sent over TCP to ensure delivery.
   * Uses a persistent connection pool to optimize performance.


3. Unix Datagram Forwarding:

    * Data is sent to a Unix domain socket.
    * Suitable for inter-process communication (IPC).

4. Unix Stream Forwarding:

    * Data is sent to a Unix domain socket.
    * Suitable for inter-process communication (IPC).

## Troubleshooting
### 1. Debugging Logs
Enable logging to diagnose issues:
```shell
tail -f netflow.log
```

### 2. Check Network Traffic
Use ```tshark``` or ```tcpdump``` to inspect incoming NetFlow packets:

```shell
sudo tcpdump -i any udp port 2055
```

### 3. Verify Ports Are Listening
Run:

```shell
netstat -tulnp | grep flow-collector
```
This should show the daemon listening on NetFlow ports.

### 4. Connection Issues?
Check firewall settings:

```shell
sudo ufw allow 2055/udp
```

## 📄 License

Flow Collector is licensed under the **GNU General Public License v3.0**.

See:

- `LICENSE` for full terms