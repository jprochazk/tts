pub enum Message {
    Auth { token: String },
    Error { what: String },
}

pub type Sender = crossbeam_channel::Sender<Message>;
pub type Receiver = crossbeam_channel::Receiver<Message>;

pub fn channel() -> (Sender, Receiver) {
    crossbeam_channel::bounded(1)
}
