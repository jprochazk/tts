use chrono::{DateTime, Duration, Utc};
use eframe::{egui, epi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{bc, msg};

#[derive(Clone, Serialize, Deserialize)]
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
    rt: Arc<tokio::runtime::Runtime>,
    tts: crate::tts::TtsCtx,
    msg: msg::Receiver,
    bc: Arc<Mutex<bc::Broadcaster>>,
    state: State,

    _channel_name_timer: Option<Timer>,
    _clipboard_text_timer: Timer,
    _save_text_timer: Timer,
}

impl App {
    pub fn new(
        rt: Arc<tokio::runtime::Runtime>,
        tts: crate::tts::TtsCtx,
        msg: msg::Receiver,
        bc: Arc<Mutex<bc::Broadcaster>>,
        state: State,
    ) -> App {
        App {
            rt,
            tts,
            msg,
            bc,
            state,

            _channel_name_timer: None,
            _clipboard_text_timer: Timer::new(),
            _save_text_timer: Timer::new(),
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
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.state.channel)
                                .hint_text("Channel name"),
                        )
                        .changed()
                    {
                        self.state.channel.truncate(128);
                        if !self.state.channel.is_empty() {
                            self._channel_name_timer = Some(Timer::new());
                        }
                    }

                    if self
                        ._channel_name_timer
                        .as_ref()
                        .map(|t| t.elapsed_milliseconds(1000))
                        .unwrap_or(false)
                    {
                        self.tts.update_tts_config(self.state.clone());
                        self._channel_name_timer = None;
                    }

                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.state.command_name)
                                .hint_text("TTS command name"),
                        )
                        .changed()
                        && self.state.enable_tts
                    {
                        self.tts.update_tts_config(self.state.clone());
                    }

                    ui.add(
                        egui::TextEdit::singleline(&mut self.state.command_cooldown)
                            .hint_text("Command cooldown"),
                    );

                    if ui
                        .checkbox(&mut self.state.enable_tts, "Enable TTS Command")
                        .changed()
                    {
                        self.tts.update_tts_config(self.state.clone());
                    }
                });

                ui.separator();

                ui.vertical_centered_justified(|ui| {
                    ui.heading(format!("TTS Queue ({} pending)", self.tts.queue.len()));

                    // Play/Pause
                    let is_paused = self.tts.queue.is_paused(); // an atomic load, so it's okay to call in the ui loop
                    if ui
                        .button(if is_paused {
                            "Resume TTS ▶"
                        } else {
                            "Pause TTS ⏸"
                        })
                        .clicked()
                    {
                        if is_paused {
                            self.tts.queue.play();
                        } else {
                            self.tts.queue.pause();
                        }
                    }

                    ui.separator();

                    if ui.button("Stop TTS ⏹").clicked() {
                        self.tts.queue.stop();
                    }
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("TODO: bannedwords.txt checkbox");
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
    tts: crate::tts::TtsCtx,
    msg: msg::Receiver,
    bc: Arc<Mutex<bc::Broadcaster>>,
    state: State,
) {
    log::info!("Started the ui thread.");
    eframe::run_native(
        Box::new(App::new(rt, tts, msg, bc, state)),
        eframe::NativeOptions {
            initial_window_size: Some(egui::Vec2::new(400., 350.)),
            ..Default::default()
        },
    );
}
