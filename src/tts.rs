use std::{
    io::{BufReader, Cursor},
    num::NonZeroU32,
    sync::Arc,
    thread::JoinHandle,
};

use tokio::sync::watch;
use twitch::Message;

use crate::ui;

pub const TTS_REQUESTS_PER_MINUTE: u32 = 5;
pub const RETRY_ATTEMPTS: u8 = 3;
pub const API_TIMEOUT_SECONDS: u64 = 180;
pub const API_URL: &str = "https://mumble.stream/speak";

pub type TtsCtx = Arc<TtsContext>;

pub struct TtsContext {
    // NOTE: this is not the command cooldown, but the freqency at which we make requests to the API (it is rate limited).
    pub tts_limit: governor::RateLimiter<
        governor::state::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::DefaultClock,
    >,
    pub banned_words: tokio::sync::Mutex<censor::Censor>,
    pub queue: rodio::Sink,
    state_tx: watch::Sender<ui::State>,
    state_rx: watch::Receiver<ui::State>,
    client: reqwest::Client,
}

impl TtsContext {
    pub fn new(queue: rodio::Sink) -> Self {
        let (state_tx, state_rx) = tokio::sync::watch::channel(ui::State::default());
        Self {
            tts_limit: governor::RateLimiter::direct(governor::Quota::per_minute(
                NonZeroU32::new(TTS_REQUESTS_PER_MINUTE).unwrap(),
            )),
            banned_words: tokio::sync::Mutex::new(
                censor::Standard - "ass" - "cock" - "pussy" - "fuck" - "piss" - "shit",
            ),
            queue,
            state_tx,
            state_rx,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(API_TIMEOUT_SECONDS))
                .build()
                .unwrap(),
        }
    }

    pub fn update_tts_config(&self, state: ui::State) {
        let _ = self.state_tx.send(state);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TtsRequest {
    /// The text to say.
    pub text: String,
    /// The name of the speaker to use.
    pub speaker: &'static str,
}

pub async fn make_tts_request(ctx: TtsCtx, request: TtsRequest) {
    ctx.tts_limit.until_ready().await;
    log::info!("Received a filtered tts request: {:#?}", request);

    let mut last_error = None;

    for i in 0..RETRY_ATTEMPTS {
        log::debug!(
            "[{} / {}] Performing the request ...",
            i + 1,
            RETRY_ATTEMPTS
        );
        let response = match ctx.client.post(API_URL).json(&request).send().await {
            Ok(resp) => resp,
            Err(e) => {
                // Retry on connection errors.
                log::debug!(
                    "[{} / {}] Failed to receive a response: {}",
                    i,
                    RETRY_ATTEMPTS,
                    e
                );
                last_error = Some(e.to_string());
                continue;
            }
        };
        log::info!(
            "[{} / {}] Received a response from the server; STATUS = {}",
            i + 1,
            RETRY_ATTEMPTS,
            response.status()
        );
        log::debug!("{:#?}", response);

        // Retry on server errors.
        let status = response.status();
        if !status.is_success() {
            log::info!(
                "[{} / {}] Response wasn't a success, retrying",
                i + 1,
                RETRY_ATTEMPTS,
            );
            last_error = Some(format!(
                "HTTP {} - {}",
                status,
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string())
            ));
            continue;
        }
        let bytes = response.bytes().await;
        if bytes.is_err() {
            log::info!(
                "[{} / {}] Couldn't read the response bytes, retrying",
                i + 1,
                RETRY_ATTEMPTS,
            );
            continue;
        }

        match rodio::Decoder::new(BufReader::new(Cursor::new(bytes.unwrap().to_vec()))) {
            Ok(audio) => {
                log::info!("Successfully decoded the audio, queueing...");
                ctx.queue.append(audio);
                return;
            }
            Err(e) => {
                log::error!("Failed to decode the audio: {}. Retrying the request...", e);
            }
        }
    }

    // QQQQ: Requeue the request here or let the streamer know, if it was purchased for points?
    log::info!("All attempts to fullfil the request have been exhausted; ignoring the request");
    log::debug!("Last error was:\n{:#?}", last_error);
}

/// TTS command syntax:
/// ```
/// !tts <speaker>: <text>
/// ```
pub fn parse_tts_request(message: &str) -> Option<TtsRequest> {
    message
        .trim()
        .split_once(":")
        .map(|(l, r)| (l.trim(), r.trim()))
        .and_then(|(speaker, text)| {
            let text = text
                .replace(|c: char| c.is_ascii_whitespace(), " ")
                .chars()
                .filter(|c| {
                    c.is_ascii_digit()
                        || c.is_ascii_alphabetic()
                        || c.is_ascii_whitespace()
                        || [',', '.', '!', '?', '$', '\''].contains(c)
                })
                .collect::<String>();

            let speaker = crate::speakers::TTS_SPEAKERS.get(speaker)?;
            Some(TtsRequest { text, speaker })
        })
}

pub fn start_tts_thread(
    ctx: TtsCtx,
    rt: Arc<tokio::runtime::Runtime>,
    mut stop_recv: tokio::sync::mpsc::Receiver<()>,
) -> JoinHandle<()> {
    // TODO: Do pub/sub here

    std::thread::spawn({
        move || {
            log::info!("Started the TTS thread.");
            rt.block_on( async {
                let mut conn = twitch::connect(twitch::Config::default()).await.unwrap();
                let mut state = ui::State::default();
                let mut state_rx = ctx.state_rx.clone();

                loop {

                    tokio::select! {
                        _ = stop_recv.recv() => break,
                        Ok(_) = state_rx.changed() => {
                            let new_state = state_rx.borrow().clone();

                            log::info!(
                                "TTS config has been changed: {{\n    enabled: {},\n    command: {},\n    channel: {}\n}}", 
                                new_state.enable_tts,
                                new_state.command_name,
                                new_state.channel
                            );

                            if new_state.channel != state.channel && !new_state.channel.is_empty() {
                                conn.sender.part(&state.channel).await.expect("Failed to leave the channel");
                                log::info!("Left channel `{}`", state.channel);

                                conn.sender.join(&new_state.channel).await.expect("Failed to join the new channel");
                                log::info!("Joined channel: `{}`", new_state.channel);
                            }

                            state = new_state;
                        },
                        result = conn.reader.next() => match result {
                            Ok(message) => match message {
                                Message::Ping(ping) => conn.sender.pong(ping.arg()).await.unwrap(),
                                Message::Privmsg(message) => {
                                    // TODO: avoid this allocation
                                    if state.enable_tts && message.text().starts_with(&format!("!{} ", state.command_name)) {
                                        if let Some(request) = parse_tts_request(&message.text()[state.command_name.len() + 2..]) {
                                            if !ctx.banned_words.lock().await.check(&request.text) {
                                                tokio::spawn(make_tts_request(ctx.clone(), request));
                                            }
                                        }
                                    }

                                }
                                _ => (),
                            },
                            Err(err) => {
                                panic!("{}", err);
                            }
                        }
                    }
                }
            });
        }
    })
}
