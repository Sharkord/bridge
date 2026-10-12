# Sharkord Bridge (Experimental)

A tiny service that lets Sharkord, running in a browser, react to hotkeys
while the browser is not focused.

It only reports the keys Sharkord asked for. It never forwards anything else.

Docs: <https://sharkord.com/docs/bridge/overview>

## Download

Windows and Linux builds are on the [releases page](https://github.com/Sharkord/bridge/releases/latest).

## Run

Start it and it sits in your system tray. Click the icon to see your pairing token and the connected servers, then paste the token in Sharkord under **User Settings, Hotkeys**.

## Build from source

You need Rust 1.89 or newer. On Linux you also need `pkg-config` and the X11 headers:

```bash
# Debian, Ubuntu
sudo apt install pkg-config libx11-dev libxtst-dev libxi-dev
```

Then:

```bash
cargo build --release
```

The executable is `target/release/sharkord-bridge`. Arch, Fedora and Windows instructions are in the [docs](https://sharkord.com/docs/bridge/build-from-source).

## Logs

Set `SHARKORD_BRIDGE_LOG` to `error`, `warn`, `info` (default), `debug` or `trace`. The log is also written to `bridge.log` in the config directory.

## Test

```bash
cargo test
```
