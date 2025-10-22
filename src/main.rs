mod osc_handler;
mod udp_listener;
mod subcommand;

use clap::Parser;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use subcommand::Subcommand;
use udp_listener::start_udp_listener;
use signal_hook::consts::signal::*;
use signal_hook::flag as signal_flag;
use log::debug;

#[derive(Parser, Debug)]
#[command(name = "wrapper")]
#[command(about = "Wrapper CLI utility", long_about = None)]
struct Args {
    /// Port number to use
    #[arg(short, long)]
    port: u16,

    /// Debounce time in milliseconds (0 to disable)
    #[arg(short, long, default_value = "100")]
    debounce: u64,

    /// Command to run (with its arguments)
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let (program, program_args) = args.command.split_first()
        .expect("Command is required");

    // Create atomic flag for signal received
    let signal_received = Arc::new(AtomicBool::new(false));
    let signal_received_handler = Arc::clone(&signal_received);

    // Register signal handlers that set the flag
    signal_flag::register(SIGINT, Arc::clone(&signal_received_handler))
        .expect("Failed to register SIGINT handler");
    signal_flag::register(SIGTERM, Arc::clone(&signal_received_handler))
        .expect("Failed to register SIGTERM handler");

    // Spawn subprocess
    let subcommand = Arc::new(Subcommand::spawn(program, program_args)
        .expect("Failed to spawn subcommand"));

    let child_stdin_osc = subcommand.get_stdin();

    // Clone for signal checker
    let subcommand_signal = Arc::clone(&subcommand);
    let signal_received_checker = Arc::clone(&signal_received);

    // Thread to check for signals and handle them
    let _signal_thread = thread::spawn(move || {
        loop {
            thread::sleep(std::time::Duration::from_millis(100));

            if signal_received_checker.load(Ordering::Relaxed) {
                debug!("Received signal, initiating graceful shutdown...");
                subcommand_signal.terminate_gracefully();

                // Continue checking in case we receive more signals
                signal_received_checker.store(false, Ordering::Relaxed);
            }
        }
    });

    // Thread to listen for OSC messages over UDP
    let port = args.port;
    let debounce = args.debounce;
    let _osc_thread = thread::spawn(move || {
        start_udp_listener(port, debounce, child_stdin_osc);
    });

    // Give the subprocess a moment to display its initial prompt
    thread::sleep(std::time::Duration::from_millis(100));

    // Start forwarding stdin to the subprocess
    subcommand.start_stdin_forwarder();

    // Wait for child process to exit
    let exit_code = subcommand.wait_for_exit();

    // Exit immediately with the same status code
    std::process::exit(exit_code);
}
