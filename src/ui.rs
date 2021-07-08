use chrono::{DateTime, Duration, Utc};
use eframe::{egui, epi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{bc, get_html_file_path, msg, proxy::ProxyState};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub token: Option<String>,
    pub channel: String,
    pub command_name: String,
    pub command_cooldown: String,
    pub enable_tts: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            token: None,
            channel: "".to_string(),
            command_name: "tts".to_string(),
            command_cooldown: "0".to_string(),
            enable_tts: true,
        }
    }
}

impl State {
    pub fn load(from: String) -> State {
        if let Some(content) = from
            .split('\n')
            .nth(2)
            .and_then(|c| c.strip_prefix('`'))
            .and_then(|c| c.strip_suffix('`'))
        {
            println!("{}", content);
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }
        State::default()
    }
    pub fn save(data: &str) -> String {
        format!(
            "// Do not modify this file.\nexport const Config = JSON.parse(\n`{}`\n);",
            data
        )
    }
}

struct Timer(DateTime<Utc>);
impl Timer {
    fn new() -> Timer {
        Timer(Utc::now())
    }

    fn elapsed_milliseconds(&self, n: i64) -> bool {
        Utc::now() - self.0 > Duration::milliseconds(n)
    }

    fn reset(&mut self) {
        self.0 = Utc::now();
    }
}

pub struct App {
    msg: msg::Receiver,
    rt: Arc<tokio::runtime::Runtime>,
    bc: Arc<Mutex<bc::Broadcaster>>,
    state: State,
    proxy: ProxyState,

    _url_text: String,
    _clipboard_text_timer: Timer,
    _save_text_timer: Timer,
    _proxy_timer: Option<Timer>,
}

impl App {
    pub fn new(
        rt: Arc<tokio::runtime::Runtime>,
        msg: msg::Receiver,
        bc: Arc<Mutex<bc::Broadcaster>>,
        state: State,
        proxy: ProxyState,
    ) -> App {
        App {
            msg,
            rt,
            bc,
            state,
            proxy,
            _url_text: get_html_file_path().display().to_string(),
            _clipboard_text_timer: Timer::new(),
            _save_text_timer: Timer::new(),
            _proxy_timer: None,
        }
    }
}

macro_rules! redirect_uri {
    () => {
        "http://localhost:3030/twitch_token"
    };
}
macro_rules! client_id {
    () => {
        "sac4q5ahwnw4j9u9cilt9n7h04r8xl"
    };
}

const AUTH_URI: &str = concat!(
    "https://id.twitch.tv/oauth2/authorize",
    "?client_id=",
    client_id!(),
    "&redirect_uri=",
    redirect_uri!(),
    "&response_type=token",
    "&scope=chat:read%20bits:read%20channel:read:redemptions%20channel:read:subscriptions",
    "&force_verify=true"
);

impl App {
    fn save_config(&self) {
        let config = serde_json::to_string(&self.state).expect("Failed to serialize config");
        std::fs::write(crate::get_config_file_path(), State::save(&config))
            .expect("Failed to write config to a file");
        self.rt.block_on(async {
            self.bc
                .lock()
                .await
                .broadcast(bc::Message::Config(config))
                .await
                .expect("Failed to broadcast config");
        });
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "OBS TTS"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        while let Ok(msg) = self.msg.try_recv() {
            match msg {
                msg::Message::Auth { token } => {
                    self.state.token = Some(token);
                }
                msg::Message::Error { what: _ } => {
                    todo!()
                }
            }
        }
        if let Err(e) = self.proxy.update() {
            todo!("error: {}", e)
        }

        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    self.save_config();
                    self._save_text_timer.reset();
                }
                if !self._save_text_timer.elapsed_milliseconds(1500) {
                    ui.label("🆗");
                }
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                match self.state.token.as_ref() {
                    Some(_) => {
                        ui.label("Authenticated");
                    }
                    None => {
                        if ui.button("Authenticate").clicked() {
                            let _ = open::that(AUTH_URI);
                        }
                    }
                }
                if self.state.token.is_some() && ui.button("Reset").clicked() {
                    self.state.token = None;
                }

                match &self._proxy_timer {
                    Some(timer) if timer.elapsed_milliseconds(1000) => {
                        self._proxy_timer = None;
                        self.proxy.send(crate::proxy::Message::Refresh);
                    }
                    _ => (),
                }

                ui.add(
                    egui::Label::new(format!(
                        "Proxy status: {}",
                        if self._proxy_timer.is_some() {
                            "Refreshing..."
                        } else if self.proxy.is_proxy_running {
                            "Running"
                        } else {
                            "Stopped"
                        }
                    ))
                    .text_color(if self._proxy_timer.is_some() {
                        egui::Color32::YELLOW
                    } else if self.proxy.is_proxy_running {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    }),
                );
                if self._proxy_timer.is_some() {
                    ui.horizontal(|ui| {
                        ui.set_enabled(false);
                        let _ = ui.button("Start");
                    });
                } else if self.proxy.is_proxy_running {
                    if ui.button("Stop").clicked() {
                        self.proxy.send(crate::proxy::Message::Shutdown)
                    }
                } else if ui.button("Start").clicked() {
                    self.proxy.send(crate::proxy::Message::Startup);
                    self._proxy_timer = Some(Timer::new());
                }
            });

            ui.separator();

            ui.vertical(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.state.channel)
                            .hint_text("Channel name"),
                    )
                    .changed()
                {
                    self.state.channel.truncate(128);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.command_name)
                        .hint_text("TTS command name"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.command_cooldown)
                        .hint_text("Command cooldown"),
                );
                ui.checkbox(&mut self.state.enable_tts, "Enable TTS Command");

                let mut autostart = self.proxy.is_autostart_enabled;
                if ui
                    .checkbox(&mut autostart, "Enable Proxy Auto Startup")
                    .changed()
                {
                    self.proxy.send(if autostart {
                        crate::proxy::Message::RegisterAutoStart
                    } else {
                        crate::proxy::Message::RemoveAutoStart
                    })
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("File URL:");
                if ui
                    .add(egui::TextEdit::singleline(&mut self._url_text).enabled(false))
                    .clicked()
                {
                    use clipboard::*;
                    if let Result::<ClipboardContext, _>::Ok(mut ctx) = ClipboardProvider::new() {
                        println!("{:?}", ctx.get_contents());
                        let _ = ctx.set_contents(self._url_text.clone());
                        self._clipboard_text_timer.reset();
                    }
                }
            });
            ui.label(if self._clipboard_text_timer.elapsed_milliseconds(1500) {
                "(click to copy)"
            } else {
                "(copied!)"
            });
        });
    }
}

pub fn start(
    rt: Arc<tokio::runtime::Runtime>,
    msg: msg::Receiver,
    bc: Arc<Mutex<bc::Broadcaster>>,
    state: State,
    proxy: ProxyState,
) {
    eframe::run_native(
        Box::new(App::new(rt, msg, bc, state, proxy)),
        eframe::NativeOptions {
            initial_window_size: Some(egui::Vec2::new(400., 350.)),
            ..Default::default()
        },
    );
}
