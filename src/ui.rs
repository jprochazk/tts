use eframe::{egui, epi};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use tokio::sync::Mutex;

use crate::{bc, get_html_file_path, msg};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub token: Option<String>,
    pub command_name: String,
    pub channel: String,
    pub enable_tts: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            token: None,
            command_name: "tts".to_string(),
            channel: "".to_string(),
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
    pub fn save(&self) -> String {
        let config = serde_json::to_string(self).expect("Failed to serialize config");
        format!(
            "// Do not modify this file.\nexport const Config = JSON.parse(\n`{}`\n);",
            config
        )
    }
}

pub struct App {
    msg: msg::Receiver,
    rt: Arc<tokio::runtime::Runtime>,
    bc: Arc<Mutex<bc::Broadcaster>>,
    state: State,

    _url_text: String,
}

impl App {
    pub fn new(
        rt: Arc<tokio::runtime::Runtime>,
        msg: msg::Receiver,
        bc: Arc<Mutex<bc::Broadcaster>>,
        state: State,
    ) -> App {
        App {
            msg,
            rt,
            bc,
            state,
            _url_text: get_html_file_path().display().to_string(),
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
        let config = self.state.save();
        std::fs::write(crate::get_config_file_path(), &config)
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
                static LAST_CLICK: AtomicI64 = AtomicI64::new(0);
                if ui.button("Save").clicked() {
                    let now = chrono::Utc::now().timestamp_millis();
                    println!("Now: {}", now);
                    LAST_CLICK.store(now, Ordering::SeqCst);
                    self.save_config();
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

            if ui
                .add(egui::TextEdit::singleline(&mut self.state.channel).hint_text("Channel name"))
                .changed()
            {
                self.state.channel.truncate(128);
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.state.command_name)
                    .hint_text("TTS command name"),
            );

            if ui
                .checkbox(&mut self.state.enable_tts, "Enable TTS Command")
                .changed()
            {
                // TODO: immediately tell frontend to stop + mute any message
                println!("Enable TTS changed");
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("File URL:");
                if ui
                    .add(egui::TextEdit::singleline(&mut self._url_text).enabled(false))
                    .clicked()
                {
                    // copy to clipboard
                }
            });
            ui.label("(click to copy)");
        });
    }
}

pub fn start(
    rt: Arc<tokio::runtime::Runtime>,
    msg: msg::Receiver,
    bc: Arc<Mutex<bc::Broadcaster>>,
    state: State,
) {
    eframe::run_native(
        Box::new(App::new(rt, msg, bc, state)),
        eframe::NativeOptions {
            initial_window_size: Some(egui::Vec2::new(400., 350.)),
            ..Default::default()
        },
    );
}
