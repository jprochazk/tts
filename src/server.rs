use futures::StreamExt;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{bc, msg};

async fn event_loop(ws: warp::ws::WebSocket, bc: Arc<Mutex<bc::Broadcaster>>) {
    let (tx, mut rx) = ws.split();
    let id = { bc.lock().await.add(tx) };
    while let Some(result) = rx.next().await {
        match result {
            Ok(_) => (),
            Err(e) => println!("Error (id={}): {:?}", id, e),
        };
    }
    bc.lock().await.remove(id);
}

pub async fn start(msg: msg::Sender, bc: Arc<Mutex<bc::Broadcaster>>) {
    use warp::Filter;

    let msg = warp::any().map(move || msg.clone());
    let bc = warp::any().map(move || bc.clone());

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

    let events = warp::path("events").and(warp::ws()).and(bc).map(
        move |ws: warp::ws::Ws, bc: Arc<Mutex<bc::Broadcaster>>| {
            ws.on_upgrade(move |ws| event_loop(ws, bc))
        },
    );

    warp::serve(twitch_token.or(twitch_response).or(events))
        .run(([127, 0, 0, 1], 3030))
        .await;
}
