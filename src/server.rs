use crate::msg;
use std::collections::HashMap;

pub async fn start(msg: msg::Sender) {
    use warp::Filter;

    let msg = warp::any().map(move || msg.clone());

    let twitch_token = warp::path("twitch_token")
        .and(warp::query::<HashMap<String, String>>())
        .and(msg.clone())
        .map(
            move |mut query: HashMap<String, String>, msg: msg::Sender| {
                if let Some(what) = query.remove("error_description") {
                    let _ = msg.send(msg::Message::Error { what });
                }

                warp::reply::html(include_str!("./twitch_token.html"))
            },
        );

    let twitch_response = warp::path!("twitch_response" / String).and(msg).map(
        move |token: String, msg: msg::Sender| {
            let _ = msg.send(msg::Message::Auth { token });

            warp::reply::html(include_str!("./twitch_response.html"))
        },
    );

    warp::serve(twitch_token.or(twitch_response))
        .run(([127, 0, 0, 1], 3030))
        .await;
}
