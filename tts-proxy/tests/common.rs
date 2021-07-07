use std::{
    net::TcpListener,
    num::{NonZeroU32, NonZeroUsize},
    time::Duration,
};

use actix_http::http;
use actix_web::{
    web::{self, Bytes},
    App, HttpResponse, HttpServer,
};

async fn mock_speak(data: web::Json<tts_proxy::TtsRequest>) -> HttpResponse {
    if data.text == "500, please" {
        return HttpResponse::build(http::StatusCode::INTERNAL_SERVER_ERROR).finish();
    }
    if data.text == "timeout, please" {
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let bytes = data
        .text
        .split_whitespace()
        .map(|s| s.bytes().collect::<Vec<_>>());

    HttpResponse::Ok().streaming(futures::stream::iter(
        bytes
            .collect::<Vec<_>>()
            .into_iter()
            .map(|b| Ok::<_, actix_web::http::Error>(Bytes::from(b))),
    ))
}

fn run_mock_api() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to a random port");
    let port = listener.local_addr().unwrap().port();
    let server = HttpServer::new(|| App::new().route("/speak", web::post().to(mock_speak)))
        .listen(listener)
        .unwrap()
        .run();
    let _ = tokio::spawn(server);
    port
}

pub fn spawn_app() -> std::io::Result<String> {
    let port = run_mock_api();

    let config = tts_proxy::config::Config {
        api_url: format!("http://localhost:{}/speak", port),
        request_limit_per_minute_per_ip: NonZeroU32::new(1).unwrap(),
        max_concurrent_requests: NonZeroUsize::new(3).unwrap(),
        api_timeout_seconds: 1,
    };

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to a random port");
    let app_port = listener.local_addr().unwrap().port();
    let server = tts_proxy::run(config, listener).unwrap();
    let _ = tokio::spawn(server);

    Ok(format!("http://localhost:{}", app_port))
}

#[macro_export]
macro_rules! request {
    ($addr:ident, $data:expr) => {{
        let client = reqwest::Client::new();
        $crate::request!(client, $addr, $data)
    }};
    ($client:ident, $addr:ident, $data:expr) => {
        $client
            .post(format!("{}/speak", $addr))
            .json(&$data)
            .send()
            .await
            .expect("Failed to make the request")
    };
}
