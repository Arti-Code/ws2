use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::{Message, Utf8Bytes}};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use anyhow::{Result, anyhow};
use crate::command::*;

pub struct Client {
    name: String,
    write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    url: String,
}

impl Client {
    pub fn new(name: &str, url: &str) -> Self {
        Client {
            name: name.to_string(),
            write: None,
            url: url.to_string(),
        }
    }

    pub async fn connect(&mut self) 
    -> Result<tokio::sync::mpsc::UnboundedReceiver<MyMessage>, anyhow::Error> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (mut write, read) = ws_stream.split();
        let msg = MyMessage::Register(self.name.clone());
        let data = serde_json::to_string::<MyMessage>(&msg).unwrap();
        write.send(Message::Text(data.into())).await?;
        self.write = Some(write);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<MyMessage>();
        let tx2 = tx.clone();
        tokio::spawn(async move {
            Client::receive(read, tx2).await;
        });
        //_=  tx.send(Command::Connected);
        return Ok(rx);
    }

    pub async fn offer(&mut self, target: &str) -> Result<(), anyhow::Error> {
        if let Some(ref mut write) = self.write {
            let offer = MyMessage::Offer(SessionDescription {
                sender: self.name.clone(),
                target: target.to_string(),
                description: generate_description(),
            });
            let data = serde_json::to_string::<MyMessage>(&offer).unwrap();
            write.send(Message::Text(data.into())).await?;
        } else {
            return Err(anyhow!("write is None"));
        }
        Ok(())
    }

    pub async fn answer(&mut self, target: &str) -> Result<(), anyhow::Error> {
        if let Some(ref mut write) = self.write {
            let answer = MyMessage::Answer(SessionDescription {
                sender: self.name.clone(),
                target: target.to_string(),
                description: generate_description(),
            });
            let data = serde_json::to_string::<MyMessage>(&answer).unwrap();
            write.send(Message::Text(data.into())).await?;
        } else {
            return Err(anyhow!("write is None"));
        }
        Ok(())
    }

    pub async fn pinging(&mut self, interval: u32) -> Result<(), anyhow::Error> {
        if let Some(ref mut write) = self.write {
            loop {
                write.send(Message::Text(Utf8Bytes::from_static("ping!"))).await?;
                tokio::time::sleep(Duration::from_secs(interval as u64)).await;
            }
        } else {
            return Err(anyhow!("write is None"));
        }
        //Ok(())
    }

    async fn receive(mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, sender: tokio::sync::mpsc::UnboundedSender<MyMessage>) {
        while let Some(msg) = reader.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let data = text.to_string();
                    match serde_json::from_str::<MyMessage>(&data) {
                        Err(_) => {},
                        Ok(msg) => {
                            _ = sender.send(msg);
                        }
                    }
                    //_ = sender.send(Command::Text(text.to_string()));
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