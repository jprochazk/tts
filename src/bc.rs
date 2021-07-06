use anyhow::Result;
use futures::{stream::SplitSink, SinkExt};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

pub type BroadcastSender = SplitSink<warp::ws::WebSocket, warp::ws::Message>;

#[derive(Default)]
pub struct Broadcaster {
    senders: HashMap<u32, BroadcastSender>,
}
impl Broadcaster {
    pub async fn broadcast(&mut self, message: Message) -> Result<()> {
        let message = String::from(message);
        for sender in self.senders.values_mut() {
            sender.send(warp::ws::Message::text(&message)).await?;
        }
        Ok(())
    }

    pub fn add(&mut self, sender: BroadcastSender) -> u32 {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        self.senders.insert(id, sender);
        id
    }

    pub fn remove(&mut self, id: u32) -> Option<BroadcastSender> {
        self.senders.remove(&id)
    }
}

pub enum Message {
    Config(String),
    Reload,
}

impl From<Message> for String {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Config(data) => format!("{{\"type\":\"config\",\"data\":{}}}", data),
            Message::Reload => "{{\"type\":\"reload\"}}".to_string(),
        }
    }
}
