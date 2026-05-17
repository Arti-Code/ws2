use tokio_tungstenite::tungstenite::Message;




pub struct Peer {
    name: Option<String>,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
    logger: bool,
}

impl Peer {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<Message>, logger: bool) -> Self {
        Self {
            name: None,
            sender,
            logger,
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn send(&self, message: Message) -> Result<(), tokio::sync::mpsc::error::SendError<Message>> {
        self.sender.send(message)
    }

    pub fn is_logger(&self) -> bool {
        self.logger
    }

    pub fn set_logger(&mut self, logger: bool) {
        self.logger = logger;
    }
}