//! Flow Collector
//!
//! High-performance NetFlow listener and forwarder.
mod output;
mod types;

use clap::{Arg, Command};
use flexi_logger::{Cleanup, Criterion, Logger, Naming};
use futures::future::join_all;
use log::{info, error, warn};
use netflow_parser::RouterScopedParser;
use std::env;
use std::fs::File;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{net::UdpSocket as TokioUdpSocket, signal, task};

/// Sets up logging based on configuration.
fn setup_logging(log_config: &types::Logging) -> Result<(), Box<dyn std::error::Error>> {
    Logger::try_with_str("info")?
        .log_to_file(flexi_logger::FileSpec::default().basename(&log_config.file))
        .rotate(
            Criterion::Size(log_config.maxsize),
            Naming::Numbers,
            Cleanup::KeepLogFiles(log_config.keep),
        )
        .start()?;

    info!("Logging initialized");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Flow Daemon")
        .arg(
            Arg::new("config-file")
                .long("config-file")
                .value_name("FILE")
                .help("Path to the configuration file")
                .action(clap::ArgAction::Set),
        )
        .get_matches();

    let config_file = matches
        .get_one::<String>("config-file")
        .map(PathBuf::from)
        .or_else(|| env::var("CONFIG_FILE").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("config.yaml"));

    let config: types::NetflowConfig = serde_yaml::from_reader(File::open(config_file)?)?;
    setup_logging(&config.logging)?;

    if config.netflow.enabled {
        info!("Starting NetFlow collector...");

        let netflow_config = Arc::new(config.netflow);
        let forwarding_config = Arc::new(config.forwarding.clone());
        let mut handles = vec![];

        for &port in &netflow_config.ports {
            let n_cfg = Arc::clone(&netflow_config);
            let f_cfg = Arc::clone(&forwarding_config);
            handles.push(task::spawn(async move {
                run_flow_listener(n_cfg, f_cfg, port).await;
            }));
        }

        tokio::select! {
            _ = join_all(handles) => { info!("All listeners stopped."); }
            _ = signal::ctrl_c() => { info!("Shutdown signal received."); }
        }
    }

    info!("Exiting...");
    Ok(())
}

async fn run_flow_listener(
    netflow_config: Arc<types::Netflow>,
    forwarding_config: Arc<types::Forwarding>,
    port: u16,
) {
    let mut parser = RouterScopedParser::<IpAddr>::new();
    let bind_addr = format!("{}:{}", netflow_config.address, port);

    let socket = match TokioUdpSocket::bind(&bind_addr).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to bind to {}: {}", bind_addr, e);
            return;
        }
    };

    info!("Listening on {}", bind_addr);
    let mut buffer = [0u8; 65_535];

    loop {
        match socket.recv_from(&mut buffer).await {
            Ok((len, addr)) => {
                let data = &buffer[..len];
                let key: IpAddr = addr.ip();

                let parse_result = parser.parse_from_source(key, data);

                if let Some(e) = parse_result.error {
                    warn!("Netflow parse error from {}: {:?}", addr, e);
                }

                if !parse_result.packets.is_empty() {
                    match serde_json::to_string(&parse_result.packets) {
                        Ok(json_data) => {
                            output::forward_data(&json_data, &forwarding_config).await;
                        }
                        Err(e) => {
                            error!("Failed to serialize netflow packets to JSON: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error receiving data: {}", e);
            }
        }
    }
}