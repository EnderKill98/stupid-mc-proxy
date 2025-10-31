use crate::protocol::client::handshake::ClientHandshake;
use crate::protocol::client::login::{ClientLoginStart, ClientLoginStartOnlyName};
use crate::protocol::client::status::{ClientStatusPing, ClientStatusRequest};
use crate::protocol::server::login::ServerLoginDisconnect;
use crate::protocol::server::status::{ServerStatusPongPacket, ServerStatusResponsePacket};
use crate::protocol::types::{MinecraftDataType, VarInt};
use crate::protocol::Packet;
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use log::{error, info};
use polling::{Event, Events, Poller};
use serde_json::Value;
use socket2::{Domain, Protocol, Socket, Type};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpListener, TcpStream};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tracing::span::EnteredSpan;
use tracing::{span, Level};
use tracing_subscriber::prelude::*;

mod protocol;

#[derive(Parser)]
struct Opts {
    /// Which host to connect clients to
    target_host: String,

    /// Connect to a different port than the default one
    #[clap(short = 'p', long = "port", default_value = "25565")]
    target_port: u16,

    /// The IP:Port combo the server is listening on
    #[clap(short, long, default_value = "[::]:25565")]
    bind: String,

    /// Define a separate host to send (instead of target_host)
    #[clap(short = 'a', long)]
    alias_host: Option<String>,

    /// Define a separate port to send (instead of target_port)
    #[clap(short = 'P', long)]
    alias_port: Option<u16>,

    /// Output longer errors on connection fails (might also need to set env RUST_BACKTRACE=1)
    #[clap(short, long)]
    verbose: bool,

    /// Specify polling rate (in ms), -1 removes polling
    #[clap(short, long, default_value = "50")]
    delay: i32,

    #[clap(short, long)]
    source_ip: Vec<String>,
}

static SOURCES: LazyLock<Arc<Mutex<Vec<Arc<IpAddr>>>>> = LazyLock::new(|| Default::default());

pub fn get_available_source_ip(v4: bool, v6: bool) -> Result<Option<Arc<IpAddr>>> {
    let sources = SOURCES.lock().expect("Lock SOURCES");
    if sources.is_empty() {
        return Ok(None);
    }

    for ip in sources.iter() {
        if Arc::strong_count(ip) > 1 {
            continue;
        }
        if (ip.is_ipv4() && v4) || (ip.is_ipv6() && v6) {
            return Ok(Some(ip.clone()));
        }
    }
    Err(anyhow!("Out of Source IPs!"))
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

    for source_ip in &opts.source_ip {
        SOURCES
            .lock()
            .expect("Lock SOURCES")
            .push(Arc::new(source_ip.parse::<IpAddr>()?));
    }

    let server = TcpListener::bind(&opts.bind).context("Bind own server")?;
    info!("Ready");

    loop {
        let (client, addr) = server.accept().context("Accept new client")?;
        let target_host = opts.target_host.to_owned();
        let target_port = opts.target_port.to_owned();
        let alias_host = opts.alias_host.as_ref().cloned();
        let alias_port = opts.alias_port.as_ref().cloned();
        let verbose = opts.verbose;
        let delay = opts.delay;
        std::thread::spawn(move || {
            let entered_span = span!(
                Level::INFO,
                "conn",
                ip = addr.ip().to_string(),
                user = tracing::field::Empty,
                via_ip = tracing::field::Empty,
            )
            .entered();
            info!("Connected to new client");
            let start = Instant::now();
            match handle_client(
                entered_span,
                client,
                target_host,
                target_port,
                alias_host,
                alias_port,
                delay,
            ) {
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
        hours = millis / (1000 * 60 * 60);
        millis %= 1000 * 60 * 60;
    }
    if millis >= 1000 * 60 {
        minutes = millis / (1000 * 60);
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
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(&format!("{minutes}m"));
    }
    if seconds > 0 {
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(&format!("{seconds}s"));
    }
    if hours == 0 && minutes == 0 && seconds < 10 {
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(&format!("{millis}ms"));
    }

    formatted
}

fn query_target_status_and_ping(
    target_addr: SocketAddr,
    target_host: &str,
    target_port: u16,
    alias_host: Option<&str>,
    alias_port: Option<u16>,
    protocol_version: i32,
) -> Result<(Value, u32)> {
    let mut target = TcpStream::connect(target_addr)?;
    ClientHandshake {
        protocol_version: VarInt(protocol_version),
        server_address: alias_host.unwrap_or(target_host).to_owned(),
        server_port: alias_port.unwrap_or(target_port),
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
    alias_host: Option<String>,
    alias_port: Option<u16>,
    delay: i32,
) -> Result<()> {
    // Resolve host
    // TODO: Improve on this ugliness!
    let mut target_addr_v4 = None;
    let mut target_addr_v6 = None;

    for addr_info in
        dns_lookup::getaddrinfo(Some(&target_host), None, None).map_err(|e| anyhow!("{:?}", e))?
    {
        let addr_info = addr_info?;
        match addr_info.sockaddr.ip() {
            IpAddr::V4(ip) => {
                if target_addr_v4.is_none() {
                    target_addr_v4 = Some(SocketAddrV4::new(ip, target_port));
                }
            }
            IpAddr::V6(ip) => {
                if target_addr_v6.is_none() {
                    target_addr_v6 = Some(SocketAddrV6::new(ip, target_port, 0, 0));
                }
            }
        }
    }

    if target_addr_v4.is_none() && target_addr_v6.is_none() {
        bail!("No address found for target host!");
    }
    let target_addr = if target_addr_v4.is_some() {
        SocketAddr::V4(target_addr_v4.unwrap())
    } else {
        SocketAddr::V6(target_addr_v6.unwrap())
    };

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
            alias_host.as_ref().map(|s| s.as_str()),
            alias_port,
            *handshake.protocol_version,
        )?;
        info!(
            "Queried status from {} (port {}). Own ping was {ping} ms.",
            handshake.server_address, handshake.server_port
        );

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

    let source_ip =
        match get_available_source_ip(target_addr_v4.is_some(), target_addr_v6.is_some()) {
            Ok(ip) => {
                if let Some(ref ip) = ip {
                    entered_span.record("via_ip", ip.to_string());
                }
                ip
            }
            Err(err) => {
                ServerLoginDisconnect {
                    reason: serde_json::json!({ "text": format!("StupidMCProxy Error: {err}") }),
                }
                .write_with_header_to(&mut client)
                .context("Kick client due to error obtaining new source ip")?;
                return Err(err);
            }
        };

    //let mut target = TcpStream::connect(target_addr).context("Connect to target")?;
    let mut target = match source_ip.as_ref().map(|ip| ip.as_ref()) {
        Some(IpAddr::V4(addr)) => {
            let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
            socket.set_reuse_address(true)?;
            socket.set_reuse_port(true)?;
            //socket.set_tcp_cork(true)?;
            socket.bind(&SocketAddr::V4(SocketAddrV4::new(*addr, 0)).into())?;
            socket
                .connect(
                    &target_addr_v4
                        .ok_or(anyhow!("Expected resolved target IPv4"))?
                        .into(),
                )
                .context("Connect to target (IPv4)")?;
            socket.into()
        }
        Some(IpAddr::V6(addr)) => {
            let socket = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))?;
            socket.set_reuse_address(true)?;
            socket.set_reuse_port(true)?;
            //socket.set_tcp_cork(true)?;
            socket.bind(&SocketAddr::V6(SocketAddrV6::new(*addr, 0, 0, 0)).into())?;
            socket
                .connect(
                    &target_addr_v6
                        .ok_or(anyhow!("Expected resolved target IPv6"))?
                        .into(),
                )
                .context("Connect to target (IPv6)")?;
            socket.into()
        }
        _ => TcpStream::connect(target_addr).context("Connect to target")?,
    };

    info!("Connected to target.");

    // Next state: Login (2)
    // Forward handshake with modified (server/host) to target
    let mut initial_packets_buffer = Cursor::new(Vec::<u8>::new());
    ClientHandshake {
        protocol_version: handshake.protocol_version,
        next_state: VarInt(2),
        server_port: alias_port.unwrap_or(target_port).to_owned(),
        server_address: alias_host.unwrap_or(target_host).to_owned(),
    }
    .write_with_header_to(&mut initial_packets_buffer)
    .context("Create handshake packet")?;

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
        VarInt(cursor.position() as i32).write_as_mc_type(&mut initial_packets_buffer)?;
        initial_packets_buffer.write_all(&cursor.into_inner())?;

        // Combining the first 2 packets is needed to bypass some weird TCPShield bot detection stuff
        initial_packets_buffer.seek(SeekFrom::Start(0))?;
        target.write_all(&initial_packets_buffer.into_inner())?;
    }

    // Uncork
    /*{
        let socket = unsafe { Socket::from_raw_fd(target.as_raw_fd()) };
        socket.set_tcp_cork(false)?;
        let _ = socket.into_raw_fd(); // Don't close
    }*/
    info!("Proxying raw data to each other...");

    client.set_nodelay(true)?;
    target.set_nodelay(true)?;
    client.set_nonblocking(true)?;
    target.set_nonblocking(true)?;

    let mut buf = vec![0u8; 4096 * 16];
    let mut buf_2 = Vec::with_capacity(4096 * 32);
    loop {
        if delay < 0 {
            let poller = Poller::new()?;
            let mut events = Events::new();
            unsafe { poller.add(&client, Event::readable(0))? };
            unsafe { poller.add(&target, Event::readable(0))? };
            //events.clear();
            poller.wait(&mut events, None)?;
        } else {
            std::thread::sleep(Duration::from_millis(delay as u64));
        }

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
