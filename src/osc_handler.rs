use std::io::Write;
use rosc::OscPacket;
use log::{debug, warn};
use percent_encoding::percent_decode_str;

fn decode_and_validate_address(addr: &str) -> Option<Vec<String>> {
    // Decode percent-encoded address
    let decoded = match percent_decode_str(addr).decode_utf8() {
        Ok(s) => s.to_string(),
        Err(e) => {
            warn!("Failed to decode OSC address '{}': {}", addr, e);
            return None;
        }
    };

    // OSC addresses must start with '/'
    if !decoded.starts_with('/') {
        warn!("Invalid OSC address '{}': must start with '/'", decoded);
        return None;
    }

    // Extract path components (excluding the leading '/')
    let path_components: Vec<String> = decoded[1..]
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if path_components.is_empty() {
        warn!("Invalid OSC address '{}': no path components", decoded);
        return None;
    }

    Some(path_components)
}

pub fn handle_osc_packet(packet: &OscPacket, indent: usize, child_stdin: &mut dyn Write) -> bool {
    let prefix = "  ".repeat(indent);
    match packet {
        OscPacket::Message(msg) => {
            // Validate and decode the address
            match decode_and_validate_address(&msg.addr) {
                Some(path_components) => {
                    let full_addr = format!("/{}", path_components.join("/"));
                    debug!("{}OSC Message:", prefix);
                    debug!("{}  Address: {}", prefix, full_addr);
                    debug!("{}  Args: {:?}", prefix, msg.args);

                    // Prepare message: all path components + arguments
                    let mut message_parts = path_components;

                    // Convert OSC arguments to strings, treating strings as filesystem paths
                    for arg in &msg.args {
                        let arg_str = match arg {
                            rosc::OscType::Int(i) => i.to_string(),
                            rosc::OscType::Float(f) => f.to_string(),
                            rosc::OscType::String(s) => {
                                // Treat string as filesystem path - escape internal quotes and wrap in quotes
                                let escaped = s.replace("\"", "\\\"");
                                format!("\"{}\"", escaped)
                            },
                            rosc::OscType::Blob(b) => format!("<blob {} bytes>", b.len()),
                            rosc::OscType::Long(l) => l.to_string(),
                            rosc::OscType::Double(d) => d.to_string(),
                            rosc::OscType::Char(c) => c.to_string(),
                            rosc::OscType::Bool(b) => b.to_string(),
                            rosc::OscType::Nil => "nil".to_string(),
                            rosc::OscType::Inf => "inf".to_string(),
                            _ => format!("{:?}", arg),
                        };
                        message_parts.push(arg_str);
                    }

                    let message = message_parts.join(" ");

                    // Send message to child's stdin
                    if let Err(e) = writeln!(child_stdin, "{}", message) {
                        debug!("Failed to write to child stdin: {}", e);
                        return false;
                    }
                    if let Err(e) = child_stdin.flush() {
                        debug!("Failed to flush child stdin: {}", e);
                        return false;
                    }

                    true
                }
                None => {
                    false
                }
            }
        }
        OscPacket::Bundle(bundle) => {
            debug!("{}OSC Bundle:", prefix);
            debug!("{}  Time tag: {:?}", prefix, bundle.timetag);
            debug!("{}  {} message(s)", prefix, bundle.content.len());
            let mut valid_count = 0;
            for (i, content) in bundle.content.iter().enumerate() {
                debug!("{}  Message {}:", prefix, i + 1);
                if handle_osc_packet(content, indent + 1, child_stdin) {
                    valid_count += 1;
                }
            }
            valid_count > 0
        }
    }
}
