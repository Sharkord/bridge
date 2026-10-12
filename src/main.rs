#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use sharkord_bridge::combo::Combo;
use sharkord_bridge::hub::HubMsg;
use sharkord_bridge::{
    gui, hub, input, log, log_error, log_info, server, status, token, tray, PORT,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

// on Wayland global keys come from the desktop portal, in addition to the X11
// listener which still sees whatever XWayland apps (most games) have focus
#[cfg(target_os = "linux")]
fn start_portal(hub: &UnboundedSender<HubMsg>) -> Option<UnboundedSender<Vec<Combo>>> {
    if sharkord_bridge::portal::is_wayland_session() {
        return Some(sharkord_bridge::portal::spawn(hub.clone()));
    }

    None
}

#[cfg(not(target_os = "linux"))]
fn start_portal(_hub: &UnboundedSender<HubMsg>) -> Option<UnboundedSender<Vec<Combo>>> {
    None
}

// the same executable is also the window: the tray starts it with --window
fn main() {
    if std::env::args().any(|arg| arg == "--window") {
        gui::run();

        return;
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("could not start the async runtime");

    runtime.block_on(run());
}

async fn run() {
    let log_file = log::init();

    log_info!(
        "Sharkord Bridge {} starting on {} (log level {:?})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        log::level()
    );

    if let Some(path) = &log_file {
        log_info!("writing the log to {}", path.display());
    }

    log_info!(
        "session: wayland={} x11={}",
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var_os("DISPLAY").is_some()
    );

    let (token, path) = match token::load_or_create() {
        Ok(result) => result,
        Err(error) => {
            log_error!("could not read or create the token: {error}");
            std::process::exit(1);
        }
    };

    // a second copy cannot bind the port, which doubles as the single instance check
    let listener = match TcpListener::bind(("127.0.0.1", PORT)).await {
        Ok(listener) => listener,
        Err(error) => {
            log_error!(
                "could not listen on 127.0.0.1:{PORT} ({error}), is the bridge already running?"
            );
            std::process::exit(1);
        }
    };

    log_info!("listening on 127.0.0.1:{PORT}");
    log_info!("pairing token stored in {}", path.display());

    // printed, never logged: the log file is meant to be shared when debugging
    println!("Pairing token: {token}");

    // a stale list from a previous run would show servers that are long gone
    status::write(&[]);

    let (hub_tx, hub_rx) = unbounded_channel();

    let portal = start_portal(&hub_tx);

    input::spawn(hub_tx.clone());
    tray::spawn(token.clone());
    tokio::spawn(hub::run(hub_rx, portal, true));

    server::serve(listener, Arc::from(token), hub_tx).await;
}
