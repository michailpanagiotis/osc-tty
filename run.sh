#!/bin/bash
cargo build --bin osc-tty
# ./target/debug/osc-tty --port 7777 --debounce 2000 mod-host -i
RUST_LOG=osc_tty=debug ./target/debug/osc-tty --port 7777 --debounce 2000 cat
# RUST_LOG=osc_tty=debug ./target/debug/osc-tty --port 7777 mod-host -i
