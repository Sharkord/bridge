use crate::combo::{Combo, Mods};
use crate::keymap::{modifier_of, InputEvent, Modifier, Phase};
use crate::protocol::{self, RawBinding};
use crate::status::{self, ServerStatus};
use crate::{log_debug, log_info, log_trace, log_warn};
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub type ConnId = u64;

pub enum HubMsg {
    Register {
        id: ConnId,
        server_id: String,
        server_name: String,
        tx: UnboundedSender<String>,
    },
    Bindings {
        id: ConnId,
        bindings: Vec<RawBinding>,
    },
    Closed {
        id: ConnId,
    },
    Input(InputEvent),
    // a whole combo went down or up, as reported by the Wayland portal which
    // knows nothing about individual keys
    Shortcut {
        combo: Combo,
        phase: Phase,
    },
}

struct Bind {
    id: String,
    combo: Combo,
    active: bool,
}

struct Conn {
    server_id: String,
    server_name: String,
    // "name (id)", for the log
    label: String,
    tx: UnboundedSender<String>,
    binds: Vec<Bind>,
}

// owns every connection, so a press is matched once and fanned out to each
// server that bound it
#[derive(Default)]
pub struct Hub {
    conns: HashMap<ConnId, Conn>,
    held: HashSet<&'static str>,
    portal: Option<UnboundedSender<Vec<Combo>>>,
    // only the running bridge publishes what its window shows
    publish_status: bool,
}

// how a server is named in the log
pub fn server_label(name: &str, id: &str) -> String {
    format!("{name} ({id})")
}

fn announce(conn_label: &str, tx: &UnboundedSender<String>, bind: &mut Bind, down: bool) {
    bind.active = down;

    let phase = if down { "down" } else { "up" };

    log_info!(
        "{phase} {} ({}) -> {conn_label}",
        bind.id,
        bind.combo.canonical()
    );

    let _ = tx.send(protocol::event(&bind.id, down));
}

impl Hub {
    pub fn new(portal: Option<UnboundedSender<Vec<Combo>>>, publish_status: bool) -> Self {
        Hub {
            portal,
            publish_status,
            ..Hub::default()
        }
    }

    pub fn handle(&mut self, msg: HubMsg) {
        match msg {
            HubMsg::Register {
                id,
                server_id,
                server_name,
                tx,
            } => self.register(id, server_id, server_name, tx),
            HubMsg::Bindings { id, bindings } => self.set_bindings(id, bindings),
            HubMsg::Closed { id } => {
                self.conns.remove(&id);
                self.changed();
            }
            HubMsg::Input(event) => self.input(event),
            HubMsg::Shortcut { combo, phase } => self.shortcut(&combo, phase),
        }
    }

    fn changed(&self) {
        self.sync_portal();
        self.publish();
    }

    fn publish(&self) {
        if !self.publish_status {
            return;
        }

        let mut servers: Vec<ServerStatus> = self
            .conns
            .values()
            .map(|conn| ServerStatus {
                name: conn.server_name.clone(),
                id: conn.server_id.clone(),
                hotkeys: conn.binds.len(),
            })
            .collect();

        servers.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));

        status::write(&servers);
    }

    // the portal only needs each distinct combo once
    fn sync_portal(&self) {
        let Some(portal) = &self.portal else {
            return;
        };

        let mut combos: Vec<Combo> = self
            .conns
            .values()
            .flat_map(|conn| conn.binds.iter())
            .map(|bind| bind.combo.clone())
            .collect();

        combos.sort_by_key(|combo| combo.canonical());
        combos.dedup();

        log_debug!("asking the portal for {} shortcut(s)", combos.len());

        let _ = portal.send(combos);
    }

    fn shortcut(&mut self, combo: &Combo, phase: Phase) {
        let mut matched = false;

        for conn in self.conns.values_mut() {
            for bind in conn.binds.iter_mut() {
                if bind.combo != *combo {
                    continue;
                }

                matched = true;

                match phase {
                    Phase::Down if !bind.active => announce(&conn.label, &conn.tx, bind, true),
                    Phase::Up if bind.active => announce(&conn.label, &conn.tx, bind, false),
                    _ => log_debug!(
                        "portal {phase:?} for {} ignored, already in that state",
                        bind.id
                    ),
                }
            }
        }

        if !matched {
            log_debug!("portal shortcut {} matched no bind", combo.canonical());
        }
    }

    // the newest connection of a server wins. dropping the old sender makes its
    // task close the socket, so two tabs never both react
    fn register(
        &mut self,
        id: ConnId,
        server_id: String,
        server_name: String,
        tx: UnboundedSender<String>,
    ) {
        let replaced: Vec<ConnId> = self
            .conns
            .iter()
            .filter(|(_, conn)| conn.server_id == server_id)
            .map(|(old_id, _)| *old_id)
            .collect();

        for old_id in replaced {
            if let Some(old) = self.conns.remove(&old_id) {
                log_info!("{} was replaced by a newer connection", old.label);

                let _ = old.tx.send(protocol::replaced());
            }
        }

        let label = server_label(&server_name, &server_id);

        self.conns.insert(
            id,
            Conn {
                server_id,
                server_name,
                label,
                tx,
                binds: Vec::new(),
            },
        );

        self.changed();
    }

    fn set_bindings(&mut self, id: ConnId, bindings: Vec<RawBinding>) {
        let Some(conn) = self.conns.get_mut(&id) else {
            return;
        };

        let mut next = Vec::new();

        for raw in bindings {
            let Some(combo) = Combo::parse(&raw.combo) else {
                log_warn!(
                    "{} sent an invalid combo for {}: {:?}",
                    conn.label,
                    raw.id,
                    raw.combo
                );

                let _ = conn.tx.send(protocol::error(&raw.id, "invalid combo"));

                continue;
            };

            // a resend must not drop a key that is being held right now
            let active = conn
                .binds
                .iter()
                .any(|old| old.id == raw.id && old.combo == combo && old.active);

            next.push(Bind {
                id: raw.id,
                combo,
                active,
            });
        }

        let summary: Vec<String> = next
            .iter()
            .map(|bind| format!("{}={}", bind.id, bind.combo.canonical()))
            .collect();

        log_info!(
            "{} bound {} hotkey(s): {}",
            conn.label,
            summary.len(),
            summary.join(", ")
        );

        conn.binds = next;
        self.changed();
    }

    fn held_mods(&self) -> Mods {
        let mut mods = Mods::default();

        for token in &self.held {
            match modifier_of(token) {
                Some(Modifier::Ctrl) => mods.ctrl = true,
                Some(Modifier::Alt) => mods.alt = true,
                Some(Modifier::Shift) => mods.shift = true,
                Some(Modifier::Meta) => mods.meta = true,
                None => {}
            }
        }

        mods
    }

    fn input(&mut self, event: InputEvent) {
        log_trace!("input {} {:?}", event.token, event.phase);

        match event.phase {
            Phase::Down => self.held.insert(event.token),
            Phase::Up => self.held.remove(event.token),
        };

        let mods = self.held_mods();

        for conn in self.conns.values_mut() {
            for bind in conn.binds.iter_mut() {
                match event.phase {
                    // auto repeat sends down again while the bind is active
                    Phase::Down if !bind.active => {
                        if bind.combo.key == event.token && mods.contains(&bind.combo.mods) {
                            announce(&conn.label, &conn.tx, bind, true);
                        }
                    }
                    // releasing the key or a required modifier ends the hold,
                    // so push to talk cannot stay stuck
                    Phase::Up if bind.active => {
                        if bind.combo.key == event.token || !mods.contains(&bind.combo.mods) {
                            announce(&conn.label, &conn.tx, bind, false);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub async fn run(
    mut rx: UnboundedReceiver<HubMsg>,
    portal: Option<UnboundedSender<Vec<Combo>>>,
    publish_status: bool,
) {
    let mut hub = Hub::new(portal, publish_status);

    while let Some(msg) = rx.recv().await {
        hub.handle(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;

    fn press(token: &'static str) -> HubMsg {
        HubMsg::Input(InputEvent {
            token,
            phase: Phase::Down,
        })
    }

    fn release(token: &'static str) -> HubMsg {
        HubMsg::Input(InputEvent {
            token,
            phase: Phase::Up,
        })
    }

    fn bind(id: &str, combo: &str) -> RawBinding {
        RawBinding {
            id: id.to_string(),
            combo: combo.to_string(),
        }
    }

    fn connect(
        hub: &mut Hub,
        id: ConnId,
        server_id: &str,
        bindings: Vec<RawBinding>,
    ) -> UnboundedReceiver<String> {
        let (tx, rx) = unbounded_channel();

        hub.handle(HubMsg::Register {
            id,
            server_id: server_id.to_string(),
            server_name: "Test".to_string(),
            tx,
        });
        hub.handle(HubMsg::Bindings { id, bindings });

        rx
    }

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<String> {
        let mut out = Vec::new();

        while let Ok(msg) = rx.try_recv() {
            out.push(msg);
        }

        out
    }

    fn event(id: &str, phase: &str) -> String {
        protocol::event(id, phase == "down")
    }

    #[test]
    fn fires_down_and_up_for_a_plain_key() {
        let mut hub = Hub::default();
        let mut rx = connect(&mut hub, 1, "a", vec![bind("ptt", "KeyV")]);

        hub.handle(press("KeyV"));
        hub.handle(release("KeyV"));

        assert_eq!(
            drain(&mut rx),
            vec![event("ptt", "down"), event("ptt", "up")]
        );
    }

    #[test]
    fn needs_the_modifiers_held() {
        let mut hub = Hub::default();
        let mut rx = connect(&mut hub, 1, "a", vec![bind("mute", "ctrl+KeyM")]);

        hub.handle(press("KeyM"));
        hub.handle(release("KeyM"));

        assert!(drain(&mut rx).is_empty());

        hub.handle(press("ControlLeft"));
        hub.handle(press("KeyM"));

        assert_eq!(drain(&mut rx), vec![event("mute", "down")]);
    }

    #[test]
    fn ignores_auto_repeat() {
        let mut hub = Hub::default();
        let mut rx = connect(&mut hub, 1, "a", vec![bind("ptt", "KeyV")]);

        hub.handle(press("KeyV"));
        hub.handle(press("KeyV"));
        hub.handle(press("KeyV"));

        assert_eq!(drain(&mut rx), vec![event("ptt", "down")]);
    }

    #[test]
    fn releasing_a_required_modifier_ends_the_hold() {
        let mut hub = Hub::default();
        let mut rx = connect(&mut hub, 1, "a", vec![bind("ptt", "ctrl+KeyV")]);

        hub.handle(press("ControlLeft"));
        hub.handle(press("KeyV"));
        hub.handle(release("ControlLeft"));
        hub.handle(release("KeyV"));

        assert_eq!(
            drain(&mut rx),
            vec![event("ptt", "down"), event("ptt", "up")]
        );
    }

    #[test]
    fn fans_out_to_every_server_that_bound_the_combo() {
        let mut hub = Hub::default();
        let mut a = connect(&mut hub, 1, "a", vec![bind("mute", "F8")]);
        let mut b = connect(&mut hub, 2, "b", vec![bind("mute", "F8")]);
        let mut c = connect(&mut hub, 3, "c", vec![bind("other", "F9")]);

        hub.handle(press("F8"));

        assert_eq!(drain(&mut a), vec![event("mute", "down")]);
        assert_eq!(drain(&mut b), vec![event("mute", "down")]);
        assert!(drain(&mut c).is_empty());
    }

    #[test]
    fn newest_connection_of_a_server_replaces_the_old_one() {
        let mut hub = Hub::default();
        let mut old = connect(&mut hub, 1, "a", vec![bind("mute", "KeyM")]);
        let mut new = connect(&mut hub, 2, "a", vec![bind("mute", "KeyM")]);

        hub.handle(press("KeyM"));

        assert_eq!(drain(&mut old), vec![protocol::replaced()]);
        assert_eq!(drain(&mut new), vec![event("mute", "down")]);
    }

    #[test]
    fn reports_invalid_combos_and_keeps_the_rest() {
        let mut hub = Hub::default();
        let mut rx = connect(
            &mut hub,
            1,
            "a",
            vec![bind("bad", "ctrl+"), bind("good", "KeyG")],
        );

        assert_eq!(
            drain(&mut rx),
            vec![protocol::error("bad", "invalid combo")]
        );

        hub.handle(press("KeyG"));

        assert_eq!(drain(&mut rx), vec![event("good", "down")]);
    }

    #[test]
    fn portal_shortcuts_fan_out_by_combo() {
        let mut hub = Hub::default();
        let mut a = connect(&mut hub, 1, "a", vec![bind("mute", "ctrl+KeyM")]);
        let mut b = connect(&mut hub, 2, "b", vec![bind("other", "KeyM")]);
        let combo = Combo::parse("ctrl+KeyM").unwrap();

        hub.handle(HubMsg::Shortcut {
            combo: combo.clone(),
            phase: Phase::Down,
        });
        hub.handle(HubMsg::Shortcut {
            combo: combo.clone(),
            phase: Phase::Down,
        });
        hub.handle(HubMsg::Shortcut {
            combo,
            phase: Phase::Up,
        });

        assert_eq!(
            drain(&mut a),
            vec![event("mute", "down"), event("mute", "up")]
        );
        assert!(drain(&mut b).is_empty());
    }

    #[test]
    fn tells_the_portal_which_combos_to_bind() {
        let (portal_tx, mut portal_rx) = unbounded_channel();
        let mut hub = Hub::new(Some(portal_tx), false);
        let (tx, _rx) = unbounded_channel();

        hub.handle(HubMsg::Register {
            id: 1,
            server_id: "a".to_string(),
            server_name: "Test".to_string(),
            tx,
        });
        hub.handle(HubMsg::Bindings {
            id: 1,
            bindings: vec![bind("mute", "ctrl+KeyM"), bind("dup", "ctrl+KeyM")],
        });

        let mut last = Vec::new();

        while let Ok(combos) = portal_rx.try_recv() {
            last = combos;
        }

        assert_eq!(last, vec![Combo::parse("ctrl+KeyM").unwrap()]);

        hub.handle(HubMsg::Closed { id: 1 });

        let mut after = None;

        while let Ok(combos) = portal_rx.try_recv() {
            after = Some(combos);
        }

        assert_eq!(after, Some(Vec::new()));
    }

    #[test]
    fn resending_binds_keeps_a_held_key_active() {
        let mut hub = Hub::default();
        let mut rx = connect(&mut hub, 1, "a", vec![bind("ptt", "KeyV")]);

        hub.handle(press("KeyV"));
        hub.handle(HubMsg::Bindings {
            id: 1,
            bindings: vec![bind("ptt", "KeyV")],
        });
        hub.handle(release("KeyV"));

        assert_eq!(
            drain(&mut rx),
            vec![event("ptt", "down"), event("ptt", "up")]
        );
    }
}
