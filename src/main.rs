#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bc;
mod msg;
mod server;
mod ui;

use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use bc::Broadcaster;
use tokio::sync::Mutex;
use ui::State;

pub fn get_config_dir_path() -> PathBuf {
    let mut path = home::home_dir().expect("Failed to access CWD");
    path.push(".obs_tts_config");
    path
}

pub fn get_config_file_path() -> PathBuf {
    let mut path = home::home_dir().expect("Failed to access CWD");
    path.push(".obs_tts_config");
    path.push("config.js");
    path
}

pub fn get_html_file_path() -> PathBuf {
    let mut path = home::home_dir().expect("Failed to access CWD");
    path.push(".obs_tts_config");
    path.push("tts.html");
    path
}

fn load_state() -> State {
    if let Ok(file) = std::fs::read_to_string(get_config_file_path()) {
        State::load(file)
    } else {
        State::default()
    }
}

fn init_config_dir() {
    let path = get_config_dir_path();
    if !path.exists() {
        std::fs::create_dir(path).unwrap();
    }

    std::fs::write(get_html_file_path(), include_str!("./tts.html")).unwrap();
}

fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = backtrace::Backtrace::new();
        let panic_log = {
            let payload = info.payload();
            if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                String::new()
            }
        };

        let mut path = get_config_dir_path();
        path.push("crashes");
        let _ = std::fs::create_dir_all(&path);
        path.push(
            chrono::Utc::now()
                .format("crash__%d_%m_%Y__%H_%M_%S.txt")
                .to_string(),
        );
        let _ = std::fs::write(
            path,
            format!("Message: {}\nBacktrace:\n{:?}", panic_log, backtrace),
        );
    }));
}

fn main() -> Result<()> {
    init_panic_hook();
    init_config_dir();
    let state = load_state();
    let broadcaster = Arc::new(Mutex::new(Broadcaster::default()));
    let (stop, stop_recv) = tokio::sync::oneshot::channel::<()>();
    let (msg_send, msg_recv) = msg::channel();

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build runtime"),
    );
    let server = std::thread::spawn({
        let broadcaster = broadcaster.clone();
        let rt = rt.clone();
        move || {
            rt.block_on(async {
                tokio::select! {
                    _ = server::start(msg_send, broadcaster) => {}
                    _ = stop_recv => {}
                }
                Result::<()>::Ok(())
            })
        }
    });
    ui::start(rt, msg_recv, broadcaster, state);

    stop.send(()).unwrap();
    server.join().unwrap().unwrap();

    Ok(())
}
