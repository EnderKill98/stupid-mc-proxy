#![feature(addr_parse_ascii)]

use crate::protocol::client::handshake::ClientHandshake;
use crate::protocol::client::status::{ClientStatusPing, ClientStatusRequest};
use crate::protocol::server::status::{ServerStatusPongPacket, ServerStatusResponsePacket};
use crate::protocol::types::VarInt;
use crate::protocol::Packet;
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use log::{error, info};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

mod protocol;

#[derive(Parser)]
struct Opts {
    bind: String,

    target_host: String,
    #[clap(short = 'p', long = "port", default_value = "25565")]
    target_port: u16,

    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO");
    }
    env_logger::builder().format_timestamp_millis().init();

    let server = TcpListener::bind(&opts.bind).context("Bind own server")?;
    info!("Ready");

    loop {
        let (client, addr) = server.accept().context("Accept new client")?;
        info!("New client connected: {addr}");
        let target_host = opts.target_host.to_owned();
        let target_port = opts.target_port.to_owned();
        let verbose = opts.verbose;
        std::thread::spawn(move || {
            if let Err(err) = handle_client(client, target_host, target_port) {
                if verbose {
                    error!("handle_client failed: {err:?}");
                } else {
                    error!("handle_client failed: {err}");
                }
            }
        });
    }
}

fn query_target_status_and_ping(
    target_addr: SocketAddr,
    target_host: &str,
    target_port: u16,
    protocol_version: i32,
) -> Result<(Value, u32)> {
    let mut target = TcpStream::connect(target_addr)?;
    ClientHandshake {
        protocol_version: VarInt(protocol_version),
        server_address: target_host.to_owned(),
        server_port: target_port,
        next_state: VarInt(1), // = Status
    }
    .write_with_header_to(&mut target)?;

    // Get status
    ClientStatusRequest {}.write_with_header_to(&mut target)?;
    let status = ServerStatusResponsePacket::read_with_header_from(&mut target)?;

    // Compute ping
    let ping_start = Instant::now();
    ClientStatusPing { payload: 0 }.write_with_header_to(&mut target)?;
    ServerStatusPongPacket::read_with_header_from(&mut target)?;
    let ping = ping_start.elapsed().as_millis() as u32;

    Ok((serde_json::from_str(&status.json_response)?, ping))
}

fn handle_client(mut client: TcpStream, target_host: String, target_port: u16) -> Result<()> {
    // Resolve host
    // TODO: Improve on this ugliness!
    let addr_info = dns_lookup::getaddrinfo(Some(&target_host), None, None)
        .map_err(|e| anyhow!("{:?}", e))?
        .next()
        .ok_or(anyhow!("Didn't find any result when resolving target host"))??;
    let target_addr = SocketAddr::new(addr_info.sockaddr.ip(), target_port);

    // Get first packet from client
    let handshake =
        ClientHandshake::read_with_header_from(&mut client).context("Read handshake")?;
    if handshake.next_state == VarInt(1 /*Status*/) {
        info!(
            "Client wants to query status of {} (port {}) and uses protocol version {}",
            handshake.server_address, handshake.server_port, handshake.protocol_version
        );

        ClientStatusRequest::read_with_header_from(&mut client)?;

        // Client wants status, forward and modify from target
        let (mut status, ping) = query_target_status_and_ping(
            target_addr,
            &target_host,
            target_port,
            *handshake.protocol_version,
        )?;
        info!("Queried status from {target_host} (port {target_port}). Own ping was {ping} ms.");

        // Add own suffix to status from target server
        let suffix = format!("§8[§9Stupid MC Proxy: §3{ping}ms§8]");
        if let Some(status) = status.as_object_mut() {
            if let Some(description) = status.get_mut("description") {
                match description {
                    Value::String(description_str) => description_str.push_str(&suffix),
                    Value::Object(description_obj) => {
                        if let Some(Value::Array(extra)) = description_obj.get_mut("extra") {
                            extra.push(Value::String(suffix));
                        } else {
                            bail!("\"description.extra\" in status was not an array!")
                        }
                    }
                    Value::Array(description_arr) => description_arr.push(Value::String(suffix)),
                    _ => bail!(
                        "\"description\" in status was neither a String, Object nor or an Array!"
                    ),
                }
            } else {
                bail!("Status did not contain \"description\"!");
            }
        } else {
            bail!("Queries status was not a JSON-Object!");
        }
        ServerStatusResponsePacket {
            json_response: serde_json::to_string(&status)?,
        }
        .write_with_header_to(&mut client)?;

        let ping_request = ClientStatusPing::read_with_header_from(&mut client)?;
        ServerStatusPongPacket {
            payload: ping_request.payload,
        }
        .write_with_header_to(&mut client)?;
        info!("Done responding to client with status.");
        return Ok(());
        // SEND TO CLIENT
    } else if handshake.next_state != VarInt(2 /*Login*/) {
        bail!(
            "Client requested next state {}, which is not supported!",
            handshake.next_state
        );
    }

    info!(
        "Client wants to login to {} (port {}) and uses protocol version {}",
        handshake.server_address, handshake.server_port, handshake.protocol_version
    );

    // Next state: Login (2)
    let mut target = TcpStream::connect(target_addr).context("Connect to target")?;

    // Forward handshake with modified (server/host) to target
    ClientHandshake {
        protocol_version: handshake.protocol_version,
        next_state: VarInt(2),
        server_port: target_port,
        server_address: target_host,
    }
    .write_with_header_to(&mut target)
    .context("Send handshake to target")?;

    client.set_nonblocking(true)?;
    target.set_nonblocking(true)?;

    info!("Connected to target. Proxying client to it in new thread...");

    let mut buf = vec![0u8; 4096 * 16];
    let mut buf_2 = Vec::with_capacity(4096 * 32);
    loop {
        std::thread::sleep(Duration::from_millis(25));

        // Client -> Target
        buf_2.clear();
        loop {
            match client.read(&mut buf) {
                Ok(read) => {
                    if read == 0 {
                        info!("Connection terminated by client!");
                        return Ok(());
                    }
                    buf[..read].iter().for_each(|b| buf_2.push(*b));
                }
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            break; // Done reading
                        }
                        _ => return Err(err).context("Read client"),
                    }
                }
            }
        }
        let mut pos = 0;
        while pos < buf_2.len() {
            match target.write(&buf_2[pos..]) {
                Ok(written) => {
                    pos += written;
                }
                Err(err) => match err.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(25));
                        continue;
                    }
                    _ => return Err(err).context("Write to target"),
                },
            }
        }

        // Target -> Client
        buf_2.clear();
        loop {
            match target.read(&mut buf) {
                Ok(read) => {
                    if read == 0 {
                        info!("Connection terminated by target!");
                        return Ok(());
                    }
                    buf[..read].iter().for_each(|b| buf_2.push(*b));
                }
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            break; // Done reading
                        }
                        _ => return Err(err).context("Read target"),
                    }
                }
            }
        }
        let mut pos = 0;
        while pos < buf_2.len() {
            match client.write(&buf_2[pos..]) {
                Ok(written) => {
                    pos += written;
                }
                Err(err) => match err.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(25));
                        continue;
                    }
                    _ => return Err(err).context("Write to client"),
                },
            }
        }
    }
}
