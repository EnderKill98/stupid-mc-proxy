#![feature(addr_parse_ascii)]

use crate::protocol::client::handshake::ClientHandshake;
use crate::protocol::client::login::{ClientLoginStart, ClientLoginStartOnlyName};
use crate::protocol::client::status::{ClientStatusPing, ClientStatusRequest};
use crate::protocol::server::status::{ServerStatusPongPacket, ServerStatusResponsePacket};
use crate::protocol::types::{MinecraftDataType, VarInt};
use crate::protocol::Packet;
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use log::{error, info};
use serde_json::Value;
use std::io::{Cursor, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use tracing::span::EnteredSpan;
use tracing::{span, Level};
use tracing_subscriber::prelude::*;

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
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = TcpListener::bind(&opts.bind).context("Bind own server")?;
    info!("Ready");

    loop {
        let (client, addr) = server.accept().context("Accept new client")?;
        let target_host = opts.target_host.to_owned();
        let target_port = opts.target_port.to_owned();
        let verbose = opts.verbose;
        std::thread::spawn(move || {
            let entered_span = span!(
                Level::INFO,
                "conn",
                ip = addr.ip().to_string(),
                user = tracing::field::Empty
            )
            .entered();
            info!("Connected to new client");
            let start = Instant::now();
            match handle_client(entered_span, client, target_host, target_port) {
                Ok(_) => info!(
                    "Connection finished after {}",
                    format_duration(start.elapsed())
                ),
                Err(err) => {
                    let duration_formatted = format_duration(start.elapsed());
                    if verbose {
                        error!("Finished with error after {duration_formatted}: {err:?}");
                    } else {
                        error!("Finished with error after {duration_formatted}: {err}");
                    }
                }
            }
        });
    }
}

fn format_duration(duration: Duration) -> String {
    let mut millis = duration.as_millis();
    let (mut hours, mut minutes, mut seconds) = (0, 0, 0);
    if millis >= 1000 * 60 * 60 {
        hours = millis / 1000 * 60 * 60;
        millis %= 1000 * 60 * 60;
    }
    if millis >= 1000 * 60 {
        millis = millis / 1000 * 60;
        millis %= 1000 * 60;
    }
    if millis >= 1000 {
        seconds = millis / 1000;
        millis %= 1000;
    }

    let mut formatted = String::with_capacity(24);
    if hours > 0 {
        formatted.push_str(&format!("{hours}h"));
    }
    if minutes > 0 {
        formatted.push_str(&format!("{minutes}m"));
    }
    if seconds > 0 {
        formatted.push_str(&format!("{seconds}s"));
    }
    if hours == 0 && minutes == 0 && seconds < 10 {
        formatted.push_str(&format!("{millis}ms"));
    }

    formatted
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

fn handle_client(
    entered_span: EnteredSpan,
    mut client: TcpStream,
    target_host: String,
    target_port: u16,
) -> Result<()> {
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

    let mut target = TcpStream::connect(target_addr).context("Connect to target")?;
    info!("Connected to target.");

    // Next state: Login (2)
    // Forward handshake with modified (server/host) to target
    ClientHandshake {
        protocol_version: handshake.protocol_version,
        next_state: VarInt(2),
        server_port: target_port,
        server_address: target_host,
    }
    .write_with_header_to(&mut target)
    .context("Send handshake to target")?;

    {
        let (login_first_packet_id, login_first_packet_data) =
            protocol::read_raw_packet_id_and_data(&mut client)?;
        if login_first_packet_id != ClientLoginStart::packet_id() {
            bail!(
                "Expect to receive Packet LoginStart (id {}, but got {} instead)!",
                ClientLoginStart::packet_id(),
                login_first_packet_id
            );
        }

        if let Ok(login_start) =
            ClientLoginStart::from_cursor(&mut Cursor::new(login_first_packet_data.as_slice()))
        {
            info!(
                "Client claims to be {} ({})",
                login_start.username, login_start.uuid
            );
            entered_span.record("user", login_start.username);
        } else {
            let login_start = ClientLoginStartOnlyName::from_cursor(&mut Cursor::new(
                login_first_packet_data.as_slice(),
            ))?;
            info!(
                "Client claims to be {} (old format, so likely no uuid sent)",
                login_start.username
            );
            entered_span.record("user", login_start.username);
        }

        // Forward exact received packet data to target (can vary between version)
        let mut cursor = Cursor::new(Vec::with_capacity(4 + login_first_packet_data.len()));
        login_first_packet_id.write_as_mc_type(&mut cursor)?;
        cursor.write_all(&login_first_packet_data)?;
        VarInt(cursor.position() as i32).write_as_mc_type(&mut target)?;
        target.write_all(&cursor.into_inner())?;
    }

    info!("Proxying raw data to each other...");

    client.set_nonblocking(true)?;
    target.set_nonblocking(true)?;

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
