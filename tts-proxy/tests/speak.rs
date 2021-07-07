use common::spawn_app;

#[macro_use]
mod common;

type Error = tts_proxy::ErrorResponse<String>;

#[actix_rt::test]
async fn test_normal_length_messages_are_processed() {
    let addr = spawn_app().unwrap();

    let response = request!(
        addr,
        tts_proxy::TtsRequest {
            speaker: "david-attenborough".into(),
            text: "This is a short text that shouldn't be rejected by the server".into()
        }
    );

    assert!(response.status().is_success());
    assert_eq!(
        response
            .bytes()
            .await
            .expect("Failed to decode the server's response"),
        "This is a short text that shouldn't be rejected by the server".replace(" ", "")
    );
}

#[actix_rt::test]
async fn test_short_and_long_messages_are_rejected() {
    let addr = spawn_app().unwrap();

    // Too long
    let response = request!(
        addr,
        tts_proxy::TtsRequest {
            speaker: "sonic".into(),
            text: "a".repeat(501 /* max length is 500 */)
        }
    );

    assert!(response.status().is_client_error());
    assert_eq!(
        response.json::<Error>().await.unwrap().error,
        "The tts message must be between 1 and 500 characters, but was 501"
    );

    // Too short
    let response = request!(
        addr,
        tts_proxy::TtsRequest {
            speaker: "sonic".into(),
            text: "".into() /* min length is 1 */
        }
    );
    assert!(response.status().is_client_error());
    assert_eq!(
        response.json::<Error>().await.unwrap().error,
        "The tts message must be between 1 and 500 characters, but was 0"
    );
}

#[actix_rt::test]
async fn test_unknown_speakers_are_rejected() {
    let addr = spawn_app().unwrap();

    let response = request!(
        addr,
        tts_proxy::TtsRequest {
            speaker: "this speaker doesn't exist".into(),
            text: "test".into(),
        }
    );

    assert!(response.status().is_client_error());
    assert_eq!(
        response.json::<Error>().await.unwrap().error,
        "Unknown speaker: `this speaker doesn't exist`"
    );
}

#[actix_rt::test]
async fn test_vocodes_api_errors_are_handled() {
    let addr = spawn_app().unwrap();

    let response = request!(
        addr,
        tts_proxy::TtsRequest {
            speaker: "sonic".into(),
            text: "500, please".into(),
        }
    );
    assert!(response.status().is_server_error());
    assert_eq!(response.content_length(), None);
}

#[actix_rt::test]
async fn test_timeouts_are_handled() {
    let addr = spawn_app().unwrap();

    let response = request!(
        addr,
        tts_proxy::TtsRequest {
            speaker: "sonic".into(),
            text: "timeout, please".into(),
        }
    );
    assert!(response.status().is_server_error());
    assert_eq!(
        response.json::<Error>().await.unwrap().error,
        "Failed to receive or parse the response from the API: Timeout while waiting for response"
    );
}

// TODO: figure out how to force reqwest to re-use the same connection
// #[actix_rt::test]
// async fn test_user_rate_limits_are_enforced() {
//     let addr = spawn_app().unwrap();
//     let client = reqwest::Client::builder()
//         .timeout(Duration::from_millis(100))
//         .local_address("127.0.0.1:0".parse::<std::net::IpAddr>().ok())
//         .build()
//         .unwrap();
//
//     let response = request!(
//         client,
//         addr,
//         tts_proxy::TtsRequest {
//             speaker: "sonic".into(),
//             text: "rate limit test".into()
//         }
//     );
//     assert!(response.status().is_success());
//
//     let response = client
//         .post(format!("{}/speak", addr))
//         .json(&tts_proxy::TtsRequest {
//             speaker: "sonic".into(),
//             text: "rate limit test".into(),
//         })
//         .send()
//         .await;
//     assert!(response.unwrap_err().is_timeout());
// }
