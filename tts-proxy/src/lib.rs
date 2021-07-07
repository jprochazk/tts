use std::net::TcpListener;
use std::time::Duration;

use actix_web::client::Client;
use actix_web::dev::Server;
use actix_web::http;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};

use context::*;

pub mod config;
mod context;
mod speakers;

/// The maximum length of a tts message in bytes.
const MAX_TTS_MESSAGE_LENGTH: usize = 500;
/// The URL of the actual TTS API.
const API_URL: &str = "https://mumble.stream/speak";

/// This struct is used for receiving and forwarding TTS requests.
/// The struct follows the schema of the original API.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TtsRequest {
    /// The text to say.
    text: String,
    /// The name of the speaker to use.
    speaker: String,
}

/// A helper struct for returning JSON error responses.
#[derive(Serialize)]
struct ErrorResponse<S: AsRef<str>> {
    error: S,
}

/// Runs the application using the supplied configuration.
pub fn run(config: config::Config) -> Result<Server, std::io::Error> {
    let address = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(address)?;
    let ctx = web::Data::new(ProxyContext::default());
    let server = HttpServer::new(move || {
        App::new()
            .app_data(ctx.clone())
            .route("/health_check", web::get().to(health_check))
            .route("/speak", web::post().to(speak))
    })
    .listen(listener)?
    .run();
    Ok(server)
}

/// A heartbeat endpoint that always returns 200 OK.
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// TODO: check if remote_addr() and realip_remote_addre() are subject to spoofing.
/// A reverse-proxy-style endpoint that accepts a TTS request, validates it, and forwards to the vo.codes TTS API. The response from the API is streamed back to the client.
/// This endpoint mirrors the original API, and thus could be potentially used as a drop-in replacement.
///
/// Every client connection must be identifiable by IP in order to enforce rate-limiting. If the limit is reached, the server will pause the future until the quota is restored.
/// Additionally, there is a global limit on the number of concurrent connections. If this limit is reached, the client will be immediately rejected with a Retry-After header
/// set to the number of seconds after which the client may retry.
async fn speak(req: HttpRequest, payload: web::Json<TtsRequest>, ctx: CtxData) -> HttpResponse {
    println!(
        "Received a request from {:?}: {:#?}",
        req.connection_info().remote_addr(),
        payload
    );

    // Make sure that the client has supplied an actual remote IP address.
    let conn = req.connection_info();
    let ip = match conn.remote_addr() {
        Some(ip) => ip,
        None => {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(ErrorResponse {
                error: "Couldn't determine remote IP",
            });
        }
    };

    // Validate the request before checking the quota to avoid blocking the client for no reason.
    let message = clean_tts_message(&payload.text);
    if message.is_empty() || message.len() > MAX_TTS_MESSAGE_LENGTH {
        return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(ErrorResponse {
            error: format!(
                "The tts message must be between 1 and 500 characters, but was {}",
                message.len()
            ),
        });
    }
    let speaker_id = match speakers::TTS.get_speaker_id(&payload.speaker[..]) {
        Some(id) => id,
        None => {
            return HttpResponse::build(http::StatusCode::NOT_FOUND).json(ErrorResponse {
                error: format!("Unknown speaker: {}", payload.speaker),
            });
        }
    };

    // QQQ: Is there a way avoid this allocation?
    // Attempt to accommodate the request given that the both the user and global quotas allow.
    let _guard = match ctx.try_accommodate_request(ip.to_owned()).await {
        Some(guard) => guard,
        None => {
            return HttpResponse::build(http::StatusCode::SERVICE_UNAVAILABLE)
            .set_header("Retry-After", "1")
            .json(ErrorResponse { error: "The service is currently overloaded. Please re-submit your request after Retry-After seconds." });
        }
    };

    let response = match Client::builder()
        .timeout(Duration::from_secs(ctx.config.api_timeout_seconds as _))
        .finish()
        .post(API_URL)
        .send_json(&TtsRequest {
            text: message,
            speaker: speaker_id.to_string(),
        })
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("error: {}", e);
            return HttpResponse::build(http::StatusCode::INTERNAL_SERVER_ERROR).json(
                ErrorResponse {
                    error: "Failed to receive or parse the response from the API",
                },
            );
        }
    };

    HttpResponse::build(response.status()).streaming(response)
}

/// Cleans the given TTS message, removing any non-ascii-alphanumeric characters with the exception of ascii whitespace and certain punctuation (`,.?!$'`).
fn clean_tts_message(message: &str) -> String {
    message
        .replace(|c: char| c.is_ascii_whitespace(), " ")
        .chars()
        .filter(|c| {
            c.is_ascii_digit()
                || c.is_ascii_alphabetic()
                || c.is_ascii_whitespace()
                || [',', '.', '!', '?', '$', '\''].contains(c)
        })
        .collect::<String>()
}
