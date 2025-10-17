use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::process::ChildStdin;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rosc::{decoder, OscPacket, OscMessage};
use log::debug;
use crate::osc_handler::handle_osc_packet;

fn handle_packet_for_debounce(packet: OscPacket, pending: &mut HashMap<String, (OscMessage, Instant)>, debounce_ms: u64) {
    match packet {
        OscPacket::Message(msg) => {
            let now = Instant::now();
            let scheduled_time = now + Duration::from_millis(debounce_ms);

            // Update or insert the pending message with new scheduling time
            pending.insert(msg.addr.clone(), (msg, scheduled_time));
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_packet_for_debounce(packet, pending, debounce_ms);
            }
        }
    }
}

pub fn start_udp_listener(port: u16, debounce_ms: u64, child_stdin: Arc<Mutex<ChildStdin>>) {
    let socket = match UdpSocket::bind(format!("127.0.0.1:{}", port)) {
        Ok(s) => {
            eprintln!("OSC listener started on port {}", port);
            s
        }
        Err(e) => {
            eprintln!("Failed to bind OSC listener on port {}: {}", port, e);
            return;
        }
    };

    let mut buf = [0u8; 4096];

    if debounce_ms == 0 {
        // No debouncing - process messages immediately
        eprintln!("Debouncing disabled");
        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    debug!("Received OSC packet from {}", addr);
                    match decoder::decode_udp(&buf[..size]) {
                        Ok((_, packet)) => {
                            debug!("Decoded OSC packet, processing immediately");
                            let mut stdin = child_stdin.lock().unwrap();
                            handle_osc_packet(&packet, 0, &mut *stdin);
                        }
                        Err(e) => {
                            debug!("Failed to decode OSC packet: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from OSC socket: {}", e);
                    break;
                }
            }
        }
    } else {
        // Debouncing enabled
        eprintln!("Debouncing enabled: {}ms", debounce_ms);

        // Set socket timeout to allow checking pending messages periodically
        socket
            .set_read_timeout(Some(Duration::from_millis(10)))
            .expect("Failed to set socket timeout");

        let mut pending: HashMap<String, (OscMessage, Instant)> = HashMap::new();

        loop {
            // Check for pending messages that are ready to process
            let now = Instant::now();
            let ready: Vec<String> = pending
                .iter()
                .filter(|(_, (_, scheduled_time))| now >= *scheduled_time)
                .map(|(addr, _)| addr.clone())
                .collect();

            for addr in ready {
                if let Some((msg, _)) = pending.remove(&addr) {
                    debug!("Processing debounced message: {}", msg.addr);
                    let packet = OscPacket::Message(msg);
                    let mut stdin = child_stdin.lock().unwrap();
                    handle_osc_packet(&packet, 0, &mut *stdin);
                }
            }

            // Try to receive new messages
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    debug!("Received OSC packet from {}", addr);
                    match decoder::decode_udp(&buf[..size]) {
                        Ok((_, packet)) => {
                            debug!("Decoded OSC packet, adding to debounce queue");
                            handle_packet_for_debounce(packet, &mut pending, debounce_ms);
                        }
                        Err(e) => {
                            debug!("Failed to decode OSC packet: {}", e);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Timeout, continue to check pending messages
                    continue;
                }
                Err(e) => {
                    eprintln!("Error reading from OSC socket: {}", e);
                    break;
                }
            }
        }
    }
}
