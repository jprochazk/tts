use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use warp::{Filter, Reply};
pub mod config;

/// The URL of the actual TTS API.
pub const API_URL: &str = "https://mumble.stream/speak";

/// This struct is used for receiving and forwarding TTS requests.
/// The struct follows the schema of the original API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TtsRequest {
    /// The text to say.
    pub text: String,
    /// The name of the speaker to use.
    pub speaker: String,
}

/// A helper struct for returning JSON error responses.
#[derive(Serialize, Deserialize)]
pub struct ErrorResponse<S: AsRef<str>> {
    pub error: S,
    pub retries: u8,
}

async fn speak(
    request: TtsRequest,
    client: reqwest::Client,
    config: Arc<config::Config>,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("Received a new proxy request: {:#?}", request);
    let mut last_error = None;

    for i in 0..config.retry_attempts.get() {
        debug!(
            "[{} / {}] Forwarding the request ...",
            i + 1,
            config.retry_attempts
        );
        let response = match client.post(&config.api_url).json(&request).send().await {
            Ok(resp) => resp,
            Err(e) => {
                // Retry on connection errors.
                debug!(
                    "[{} / {}] Failed to receive a response: {}",
                    i, config.retry_attempts, e
                );
                last_error = Some(e.to_string());
                continue;
            }
        };
        info!(
            "[{} / {}] Received a response from the server; STATUS = {}",
            i + 1,
            config.retry_attempts,
            response.status()
        );
        debug!("{:#?}", response);

        // Retry on server errors.
        if response.status().is_server_error() {
            info!(
                "[{} / {}] Response wasn't a success, retrying",
                i + 1,
                config.retry_attempts
            );
            last_error = Some(format!(
                "HTTP {} - {}",
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string())
            ));
            continue;
        }

        // Otherwise, re-broadcast the response.
        let (headers, status) = (response.headers().clone(), response.status());
        let body = warp::hyper::Body::wrap_stream(response.bytes_stream());
        let mut response = warp::reply::Response::new(body);
        *response.headers_mut() = headers;
        *response.status_mut() = status;
        return Ok(response);
    }

    info!("All attempts to fullfil the request have been exhausted, issuing a rejection.",);
    let mut response = warp::reply::json(&ErrorResponse {
        error: format!(
            "Failed to process the request: `{}`",
            last_error.expect("Reached")
        ),
        retries: config.retry_attempts.get(),
    })
    .into_response();
    *response.status_mut() = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
    Ok(response)
}

pub async fn start(config: config::Config) {
    let client = Client::builder()
        .timeout(Duration::from_secs(config.api_timeout_seconds as _))
        .build()
        .unwrap();
    let client = warp::any().map(move || client.clone());

    let port = config.port;
    let config = Arc::new(config);
    let config = warp::any().map(move || config.clone());

    let health_check = warp::path("health_check").map(warp::reply);
    let speak = warp::path("speak")
        .and(warp::post())
        .and(warp::body::json())
        .and(client)
        .and(config)
        .and_then(speak);

    warp::serve(health_check.or(speak))
        .run(([127u8, 0, 0, 1], port))
        .await;
}
