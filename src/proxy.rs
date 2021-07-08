use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use warp::hyper::{Method, Request};

use crate::{get_config_dir_path, get_proxy_binary_path};

pub enum Message {
    Startup,
    Shutdown,
    Refresh,
    RegisterAutoStart,
    RemoveAutoStart,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub is_proxy_running: bool,
    pub is_autostart_enabled: bool,
}

pub struct ProxyState {
    response: Response,
    sender: Sender<Message>,
    receiver: Receiver<Result<Response, String>>,
}

impl std::ops::Deref for ProxyState {
    type Target = Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl ProxyState {
    pub fn send(&self, message: Message) {
        let _ = self.sender.send(message);
    }

    pub fn update(&mut self) -> Result<(), String> {
        if let Ok(result) = self.receiver.try_recv() {
            self.response = result?;
        }
        Ok(())
    }
}

type Client = warp::hyper::Client<warp::hyper::client::HttpConnector>;

async fn is_running(client: &Client, port: u16) -> bool {
    let r = Request::builder()
        .method(Method::GET)
        .uri(format!("http://localhost:{}/health_check", port))
        .body(warp::hyper::Body::empty())
        .unwrap();
    client
        .request(r)
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn shutdown(client: &Client, port: u16) {
    let r = Request::builder()
        .method(Method::GET)
        .uri(format!("http://localhost:{}/shutdown", port))
        .body(warp::hyper::Body::empty())
        .unwrap();
    let _ = client.request(r).await;
}

// TODO: proper configuration
fn start_proxy_daemon(port: u16) -> anyhow::Result<()> {
    eprintln!("Starting the proxy daemon...");
    let binary = get_proxy_binary_path();
    let mut child = std::process::Command::new(binary)
        .args(&["--retry-attempts", "3"])
        .args(&["--port", &port.to_string()])
        .arg("--log-directory")
        .arg(get_config_dir_path())
        .arg("--daemonize")
        .spawn()?;
    match child.try_wait()? {
        Some(status) => Err(anyhow::anyhow!(format!(
            "daemon exited with the status {:?}",
            status
        ))),
        None => Ok(()),
    }
}

pub fn start_proxy_control_thread(rt: Arc<tokio::runtime::Runtime>, port: u16) -> ProxyState {
    let (message_tx, message_rx) = crossbeam_channel::unbounded();
    let (response_tx, response_rx) = crossbeam_channel::unbounded();

    std::thread::spawn(move || {
        rt.block_on(async move {
            let client = warp::hyper::Client::new();
            let last_autostart_state = false;
            loop {
                let msg = match message_rx.try_recv() {
                    Ok(msg) => msg,
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {
                        std::hint::spin_loop();
                        continue;
                    }
                };

                let resp = match msg {
                    Message::Refresh => Ok(Response {
                        is_proxy_running: is_running(&client, port).await,
                        is_autostart_enabled: false, // TODO: update here
                    }),
                    Message::Startup => {
                        if is_running(&client, port).await {
                            Ok(Response {
                                is_proxy_running: true,
                                is_autostart_enabled: last_autostart_state,
                            })
                        } else if let Err(e) = start_proxy_daemon(port) {
                            Err(e.to_string())
                        } else {
                            Ok(Response {
                                is_proxy_running: false,
                                is_autostart_enabled: last_autostart_state,
                            })
                        }
                    }
                    Message::Shutdown => {
                        shutdown(&client, port).await;
                        Ok(Response {
                            is_proxy_running: false,
                            is_autostart_enabled: last_autostart_state,
                        })
                    }
                    Message::RegisterAutoStart => todo!(),
                    Message::RemoveAutoStart => todo!(),
                };

                let _ = response_tx.send(resp);
            }
        })
    });

    // Load the current state of the proxy
    message_tx.send(Message::Refresh).unwrap();

    ProxyState {
        response: Response {
            is_proxy_running: false,
            is_autostart_enabled: false,
        },
        sender: message_tx,
        receiver: response_rx,
    }
}
