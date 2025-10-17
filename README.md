# osc-tty

A utility that wraps CLI commands with OSC (Open Sound Control) message listening.
Received OSC messages are parsed, formatted, and sent to the subprocess's stdin.

## Features

- Wraps any command with OSC UDP listener
- URL-decodes OSC addresses
- Configurable debouncing to prevent message flooding
- Forwards both user stdin and OSC messages to subprocess
- Debug logging support via `RUST_LOG` environment variable

## Usage

### Basic Usage

```bash
osc-tty --port <PORT> [OPTIONS] -- <COMMAND> [ARGS...]
```

### Examples

#### Default behavior (100ms debounce)
```bash
osc-tty --port 7777 cat
```

#### Custom debounce time (250ms)
```bash
osc-tty --port 7777 --debounce 250 cat
```

#### Disable debouncing (immediate processing)
```bash
osc-tty --port 7777 --debounce 0 cat
```

#### With debug logging enabled
```bash
RUST_LOG=osc_tty=debug osc-tty --port 7777 cat
```

## Examples with Audio Clients

These examples demonstrate using osc-tty with audio processing applications. Start osc-tty with the target application in one terminal, then use another terminal to send OSC messages to control it.

### mod-host

mod-host is an LV2 plugin host that accepts commands via stdin.

```bash
# Terminal 1 - Start osc-tty wrapping mod-host:
osc-tty --port 7777 mod-host -i
```

```bash
# Terminal 2 - Send OSC messages to control mod-host:
osc-send 127.0.0.1:7777 /param_set/0/gain 2.50
```

The OSC message `/param_set/0/gain 2.50` is converted to `param_set 0 gain 2.50` and sent to mod-host's stdin.

### jalv

jalv is an LV2 plugin host with Jack audio support.

```bash
# Terminal 1 - Start osc-tty wrapping jalv:
osc-tty --port 7777 jalv http://two-play.com/plugins/toob-nam
```

```bash
# Terminal 2 - Send OSC messages to adjust plugin parameters:
osc-send 127.0.0.1:7777 /set/inputGain 0.7
```

The OSC message `/set/inputGain 0.7` is converted to `set inputGain 0.7` and sent to jalv's stdin.

### sfizz_jack

sfizz_jack is a SFZ sampler with Jack audio support.

```bash
# Terminal 1 - Start osc-tty wrapping sfizz_jack:
osc-tty --port 7777 sfizz_jack
```

```bash
# Terminal 2 - Send OSC messages to load instruments:
osc-send 127.0.0.1:7777 /load_instrument instrument.sfz
```

The OSC message `/load_instrument instrument.sfz` is converted to `load_instrument "instrument.sfz"` and sent to sfizz_jack's stdin.

## Command-line Options

- `-p, --port <PORT>` - Port number to listen for OSC messages (required)
- `-d, --debounce <MS>` - Debounce time in milliseconds (default: 100, set to 0 to disable)
- `<COMMAND> [ARGS...]` - Command to run with its arguments

## OSC Message Format

OSC messages are converted to space-separated strings:
- Address path components are separated by spaces (instead of `/`)
- Arguments follow the address components

### Examples

| OSC Message | Output to stdin |
|-------------|-----------------|
| `/volume 0.5` | `volume 0.5` |
| `/midi/note 60 127` | `midi note 60 127` |
| `/synth/osc/freq 440.0` | `synth osc freq 440.0` |

## Debouncing

Debouncing prevents rapid successive messages from overwhelming the subprocess:

- When a message arrives, it's queued with a scheduled processing time
- If another message with the same address arrives before the debounce time elapses, the timer resets
- Only when the debounce time passes without a new message does it get sent to the subprocess
- Set `--debounce 0` to disable debouncing for immediate processing

## Installation

### Install Locally

To install the osc-tty binary to `~/.cargo/bin`:

```bash
cargo install --path .
```

This will compile with optimizations and install to your Cargo bin directory (usually `~/.cargo/bin`), which should already be in your PATH if you have Rust installed.

### Uninstall

```bash
cargo uninstall osc-tty
```

## Building

### Build for Development

```bash
cargo build
```

### Build for Release

```bash
cargo build --release
```

The binary will be at `target/release/osc-tty`.

## Running from Source

```bash
cargo run -- --port 7777 cat
```
