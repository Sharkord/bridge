use crate::{log_debug, log_info, log_warn};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;

const WATCH_EVERY: Duration = Duration::from_millis(500);

static CHILD: Mutex<Option<Child>> = Mutex::new(None);

// reaps the window once it closes, otherwise the finished process stays in the
// process table until the next time the window is opened
fn watch() {
    std::thread::spawn(|| loop {
        std::thread::sleep(WATCH_EVERY);

        let Ok(mut child) = CHILD.lock() else {
            return;
        };

        let finished = match child.as_mut() {
            Some(running) => !matches!(running.try_wait(), Ok(None)),
            None => true,
        };

        if finished {
            *child = None;

            return;
        }
    });
}

// the window is this same executable started with --window. it only exists
// while it is open, so the bridge costs nothing extra while it sits in the tray
pub fn open() {
    let Ok(mut child) = CHILD.lock() else {
        return;
    };

    if let Some(running) = child.as_mut() {
        if matches!(running.try_wait(), Ok(None)) {
            log_debug!("the window is already open");

            return;
        }
    }

    let spawned = std::env::current_exe().and_then(|exe| Command::new(exe).arg("--window").spawn());

    match spawned {
        Ok(started) => {
            log_info!("opened the window");

            *child = Some(started);

            watch();
        }
        Err(error) => log_warn!("could not open the window: {error}"),
    }
}

// the window is a separate process, so quitting has to take it down too. if the
// bridge goes away some other way, the window notices and closes itself
pub fn quit() -> ! {
    if let Ok(mut child) = CHILD.lock() {
        if let Some(mut running) = child.take() {
            let _ = running.kill();
            let _ = running.wait();
        }
    }

    log_info!("quitting");

    std::process::exit(0)
}
