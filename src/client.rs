use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, 
    WebSocketStream, 
    connect_async, 
    tungstenite::Message
};
use futures_util::{
    SinkExt, 
    StreamExt, 
    stream::{SplitSink, SplitStream}
};
use anyhow::{Result, anyhow};
use crate::command::*;

type SignalSender = tokio::sync::mpsc::UnboundedSender<String>;
type SignalReceiver = tokio::sync::mpsc::UnboundedReceiver<String>;
type WebSocketWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WebSocketRead = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub struct Client {
    name: String,
    write: Option<WebSocketWrite>,
    url: String,
    rx_data: Option<SignalReceiver>,
}

impl Client {
    pub fn new(name: &str, url: &str) -> Self {
        Client {
            name: name.to_string(),
            write: None,
            url: url.to_string(),
            rx_data: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (mut write, read) = ws_stream.split();
        let msg = SignalMessage::Register(self.name.clone());
        let data = serde_json::to_string::<SignalMessage>(&msg).unwrap();
        write.send(Message::Text(data.into())).await?;
        self.write = Some(write);
        let (tx_str, rx_str) = tokio::sync::mpsc::unbounded_channel::<String>();
        self.rx_data = Some(rx_str);
        tokio::spawn(async move {
            Client::receive_data(read, tx_str).await;
        });
        return Ok(());
    }

    pub async fn wait_data(&mut self) -> Result<SessionDescription, anyhow::Error> {
        let rx = self.rx_data.as_mut()
        .ok_or_else(|| anyhow!("no offer receiver"))?;
        match rx.recv().await {
            Some(data) => match serde_json::from_str::<SignalMessage>(&data) {
                Ok(SignalMessage::SessionDescription(sd)) => Ok(sd),
                _ => Err(anyhow!("invalid message")),
            },
            None => Err(anyhow!("connection lost")),
        }
    }

    pub async fn send_data(&mut self, target: &str, data: String, kind: DescriptionType) -> Result<(), anyhow::Error> {
        if let Some(ref mut write) = self.write {
            let answer = SignalMessage::SessionDescription(
                SessionDescription {
                    sender: self.name.clone(),
                    target: target.to_string(),
                    description: data.clone(),
                    kind,
                }
            );
            let data = serde_json::to_string::<SignalMessage>(&answer).unwrap();
            write.send(Message::Text(data.into())).await?;
        } else {
            return Err(anyhow!("write is None"));
        }
        Ok(())
    }

    async fn receive_data(mut reader: WebSocketRead, sender: SignalSender) {
        while let Some(msg) = reader.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let data = text.to_string();
                    _ = sender.send(data);
                }
                Ok(Message::Close(_)) => {
                    //_ = sender.send(Command::Disconected);
                    break;
                }
                _ => {}
            }
        }
        
    }
}