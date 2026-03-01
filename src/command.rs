use std::fmt::Display;

use rand::{RngExt, distr::Alphanumeric, rng};
use serde::{Deserialize, Serialize};




#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MyMessage {
    Register(String),
    Offer(SessionDescription),
    Answer(SessionDescription),
}


pub fn generate_description() -> String {
    let rng = rng();
    let data: String = rng.sample_iter(Alphanumeric).take(16).map(|char| char as char).collect();
    return data;
}