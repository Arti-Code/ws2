use rand::{RngExt, distr::Alphanumeric, rng};
use serde::{Deserialize, Serialize};


/* #[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Text(String),
    Connected,
    Disconected,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Connected => write!(f, "connected!"),
            Command::Disconected => write!(f, "disconected"),
            Command::Text(s) => write!(f, "{}", s),
        }
    }
} */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DescriptionType {
    Offer,
    Answer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Data,
    Video,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDescription {
    pub sender: String,
    pub target: String,
    pub description: String,
    pub kind: DescriptionType,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    pub sender: String,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalMessage {
    Register(String),
    SetLogger,
    SessionDescription(SessionDescription),
    Echo(String),
    Text(TextMessage),
    Ping(u64),
    Pong(u64),
    PeerList(Vec<String>),
    GetPeerList(String),
    //Offer(SessionDescription),
    //Answer(SessionDescription),
}


pub fn generate_description(n: usize) -> String {
    let rng = rng();
    let data: String = rng.sample_iter(Alphanumeric).take(n)
    .map(|char| char as char).collect();
    return data;
}

impl SignalMessage {
    pub fn new_offer(sender: String, target: String, connection_type: ConnectionType) -> Self {
        let description = generate_description(100);
        SignalMessage::SessionDescription(SessionDescription {
            sender,
            target,
            description,
            kind: DescriptionType::Offer,
            connection_type,
        })
    }

    pub fn new_answer(sender: String, target: String, connection_type: ConnectionType) -> Self {
        let description = generate_description(100);
        SignalMessage::SessionDescription(SessionDescription {
            sender,
            target,
            description,
            kind: DescriptionType::Answer,
            connection_type,

        })
    }

    pub fn new_text(sender: String, target: String, message: String) -> Self {
        SignalMessage::Text(TextMessage {
            sender,
            target,
            message,
        })
    }

    pub fn new_ping() -> Self {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        SignalMessage::Ping(timestamp)
    }

    pub fn new_pong(timestamp: u64) -> Self {
        SignalMessage::Pong(timestamp)
    }

    pub fn new_register(name: String) -> Self {
        SignalMessage::Register(name)
    }

    pub fn new_set_logger() -> Self {
        SignalMessage::SetLogger
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}