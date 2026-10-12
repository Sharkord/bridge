use crate::hub::HubMsg;
use crate::keymap::{key_token, InputEvent, Phase};
use crate::{log_error, log_info};
use rdev::EventType;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::UnboundedSender;

// rdev blocks forever, so it gets its own thread and talks to the hub through
// the channel. it only observes: the focused app still receives the keys
pub fn spawn(hub: UnboundedSender<HubMsg>) {
    std::thread::spawn(move || {
        let seen_input = AtomicBool::new(false);

        log_info!("starting the global input listener");

        let result = rdev::listen(move |event| {
            let input = match event.event_type {
                EventType::KeyPress(key) => key_token(key).map(|token| (token, Phase::Down)),
                EventType::KeyRelease(key) => key_token(key).map(|token| (token, Phase::Up)),
                _ => None,
            };

            if input.is_some() && !seen_input.swap(true, Ordering::Relaxed) {
                log_info!("the global input listener is receiving events");
            }

            if let Some((token, phase)) = input {
                let _ = hub.send(HubMsg::Input(InputEvent { token, phase }));
            }
        });

        if let Err(error) = result {
            log_error!("the global input listener stopped: {error:?}");
        }
    });
}
