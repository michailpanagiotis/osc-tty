use clap::Parser;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::{IpAddr, SocketAddr, UdpSocket};

#[derive(Parser, Debug)]
#[command(name = "osc-send")]
#[command(about = "Send OSC messages to a host:port", long_about = None)]
struct Args {
    /// Host and port in format "host:port" (e.g., "127.0.0.1:7777")
    #[arg(required = true)]
    host_port: String,

    /// OSC address path (e.g., "/volume", "/synth/freq")
    #[arg(required = true)]
    address: String,

    /// Optional value to send with the OSC message
    #[arg()]
    value: Option<String>,
}

fn parse_host_port(host_port: &str) -> Result<(IpAddr, u16), String> {
    let parts: Vec<&str> = host_port.split(':').collect();

    if parts.len() != 2 {
        return Err(format!("Invalid host:port format '{}'. Expected format: 'ip:port' (e.g., '127.0.0.1:7777')", host_port));
    }

    let host = parts[0].parse::<IpAddr>()
        .map_err(|_| format!("Invalid IP address: '{}'", parts[0]))?;

    let port = parts[1].parse::<u16>()
        .map_err(|_| format!("Invalid port number: '{}'. Port must be between 0 and 65535", parts[1]))?;

    Ok((host, port))
}

fn validate_osc_address(address: &str) -> Result<(), String> {
    if !address.starts_with('/') {
        return Err(format!("Invalid OSC address '{}': must start with '/'", address));
    }

    if address.len() == 1 {
        return Err(format!("Invalid OSC address '/': must contain at least one path component"));
    }

    Ok(())
}

fn parse_value(value: &str) -> OscType {
    // Try to parse as int first (whole numbers)
    if let Ok(i) = value.parse::<i32>() {
        return OscType::Int(i);
    }

    // Try to parse as float (decimal numbers)
    if let Ok(f) = value.parse::<f32>() {
        return OscType::Float(f);
    }

    // Fall back to string
    OscType::String(value.to_string())
}

fn create_osc_message(address: String, value: Option<String>) -> Result<OscPacket, String> {
    validate_osc_address(&address)?;

    let args = if let Some(val) = value {
        vec![parse_value(&val)]
    } else {
        vec![]
    };

    let msg = OscMessage { addr: address, args };

    Ok(OscPacket::Message(msg))
}

fn send_osc_packet(host: IpAddr, port: u16, packet: &OscPacket) -> Result<(), String> {
    // Encode the OSC packet to bytes
    let msg_buf = encoder::encode(packet)
        .map_err(|e| format!("Failed to encode OSC packet: {}", e))?;

    // Create UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("Failed to create UDP socket: {}", e))?;

    // Create target socket address
    let target_addr = SocketAddr::new(host, port);

    // Send the encoded message
    socket.send_to(&msg_buf, target_addr)
        .map_err(|e| format!("Failed to send OSC message: {}", e))?;

    Ok(())
}

fn main() {
    let args = Args::parse();

    let (host, port) = match parse_host_port(&args.host_port) {
        Ok((h, p)) => (h, p),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let packet = match create_osc_message(args.address.clone(), args.value.clone()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = send_osc_packet(host, port, &packet) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    println!("Sent OSC message to {}:{}", host, port);
}
