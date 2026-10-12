use crate::autostart;
use crate::icon;
use crate::status::{self, ServerStatus};
use crate::{token, PORT};
use eframe::egui;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(500);
const COPIED_FOR: Duration = Duration::from_millis(1500);
const ALIVE_EVERY: Duration = Duration::from_secs(2);
const ALIVE_TIMEOUT: Duration = Duration::from_millis(300);

// the window has no business outliving the bridge that opened it
const MISSES_BEFORE_CLOSING: u8 = 2;

struct BridgeWindow {
    token: String,
    servers: Vec<ServerStatus>,
    last_read: Instant,
    copied_at: Option<Instant>,
    autostart: bool,
    autostart_error: Option<String>,
    last_alive_check: Instant,
    missed_checks: u8,
}

fn bridge_is_listening(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    TcpStream::connect_timeout(&address, ALIVE_TIMEOUT).is_ok()
}

// egui's default text is dim grey, which reads badly on a dark background, so
// both themes get an explicit text color and a lighter secondary one
fn text_colors(dark: bool) -> (egui::Color32, egui::Color32) {
    if dark {
        (egui::Color32::from_gray(235), egui::Color32::from_gray(170))
    } else {
        (egui::Color32::from_gray(25), egui::Color32::from_gray(95))
    }
}

fn apply_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::proportional(22.0)),
            (egui::TextStyle::Body, egui::FontId::proportional(15.0)),
            (egui::TextStyle::Button, egui::FontId::proportional(15.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(15.0)),
            (egui::TextStyle::Small, egui::FontId::proportional(12.5)),
        ]
        .into();
    });
}

impl BridgeWindow {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_style(&cc.egui_ctx);

        Self {
            token: token::load_or_create()
                .map(|(token, _)| token)
                .unwrap_or_default(),
            servers: status::read(),
            last_read: Instant::now(),
            copied_at: None,
            autostart: autostart::is_enabled(),
            autostart_error: None,
            last_alive_check: Instant::now(),
            missed_checks: 0,
        }
    }

    fn token_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pairing token");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.add(egui::Label::new(egui::RichText::new(&self.token).monospace()).selectable(true));

            let copied = self.copied_at.is_some_and(|at| at.elapsed() < COPIED_FOR);
            let label = if copied { "Copied" } else { "Copy" };

            if ui.button(label).clicked() {
                ui.ctx().copy_text(self.token.clone());

                self.copied_at = Some(Instant::now());
            }
        });
    }

    fn servers_section(&self, ui: &mut egui::Ui, muted: egui::Color32) {
        ui.heading("Connected servers");
        ui.add_space(6.0);

        if self.servers.is_empty() {
            ui.label(egui::RichText::new("No Sharkord server is connected yet.").color(muted));

            return;
        }

        for server in &self.servers {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&server.name).strong());
                ui.label(egui::RichText::new(format!("{} hotkey(s)", server.hotkeys)).color(muted));
            });
            ui.label(egui::RichText::new(&server.id).small().color(muted));
            ui.add_space(6.0);
        }
    }

    fn startup_toggle(&mut self, ui: &mut egui::Ui) {
        if ui
            .checkbox(&mut self.autostart, "Run on system startup")
            .changed()
        {
            match autostart::set_enabled(self.autostart) {
                Ok(()) => self.autostart_error = None,
                Err(error) => {
                    // show what is really set, not what was just clicked
                    self.autostart = autostart::is_enabled();
                    self.autostart_error = Some(format!("Could not change it: {error}"));
                }
            }
        }

        if let Some(error) = &self.autostart_error {
            ui.label(
                egui::RichText::new(error)
                    .small()
                    .color(egui::Color32::from_rgb(235, 100, 100)),
            );
        }
    }

    fn footer(ui: &mut egui::Ui, muted: egui::Color32) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Listening on 127.0.0.1:{PORT}"))
                    .small()
                    .color(muted),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(muted),
                );
            });
        });
    }
}

impl eframe::App for BridgeWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(POLL);

        let (text, muted) = text_colors(ctx.style().visuals.dark_mode);

        ctx.style_mut(|style| style.visuals.override_text_color = Some(text));

        if self.last_alive_check.elapsed() >= ALIVE_EVERY {
            self.last_alive_check = Instant::now();

            if bridge_is_listening(PORT) {
                self.missed_checks = 0;
            } else {
                self.missed_checks += 1;
            }

            if self.missed_checks >= MISSES_BEFORE_CLOSING {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if self.last_read.elapsed() >= POLL {
            self.servers = status::read();
            self.last_read = Instant::now();
        }

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(4.0);
            self.startup_toggle(ui);
            ui.add_space(2.0);
            Self::footer(ui, muted);
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.token_section(ui);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);
            self.servers_section(ui, muted);
        });
    }
}

pub fn run() {
    let icon = icon::load();
    let viewport = egui::ViewportBuilder::default()
        .with_title("Bridge")
        .with_app_id("dev.sharkord.Bridge")
        .with_inner_size([460.0, 340.0])
        .with_min_inner_size([400.0, 260.0])
        .with_icon(egui::IconData {
            rgba: icon.rgba,
            width: icon.width,
            height: icon.height,
        });

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "Bridge",
        options,
        Box::new(|cc| Ok(Box::new(BridgeWindow::new(cc)))),
    ) {
        eprintln!("could not open the window: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn notices_whether_something_is_listening() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(bridge_is_listening(port));

        drop(listener);

        assert!(!bridge_is_listening(port));
    }
}
