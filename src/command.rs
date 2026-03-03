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
pub struct SessionDescription {
    pub sender: String,
    pub target: String,
    pub description: String,
    pub kind: DescriptionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalMessage {
    Register(String),
    SessionDescription(SessionDescription),
    //Offer(SessionDescription),
    //Answer(SessionDescription),
}


pub fn generate_description(n: usize) -> String {
    let rng = rng();
    let data: String = rng.sample_iter(Alphanumeric).take(n)
    .map(|char| char as char).collect();
    return data;
}

