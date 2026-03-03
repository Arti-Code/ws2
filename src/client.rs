use std::time::Duration;
use tokio::{net::TcpStream, sync::mpsc::UnboundedReceiver};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::{Message, Utf8Bytes}};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use anyhow::{Result, anyhow};
use crate::command::*;

pub struct Client {
    name: String,
    write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    url: String,
    rx_offer: Option<UnboundedReceiver<SessionDescription>>,
    rx_answer: Option<UnboundedReceiver<SessionDescription>>,
    rx: Option<UnboundedReceiver<SessionDescription>>,
    rx_data: Option<UnboundedReceiver<String>>,
}

impl Client {
    pub fn new(name: &str, url: &str) -> Self {
        Client {
            name: name.to_string(),
            write: None,
            url: url.to_string(),
            rx_offer: None,
            rx_answer: None,
            rx: None,
            rx_data: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
    //-> Result<tokio::sync::mpsc::UnboundedReceiver<MyMessage>, anyhow::Error> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (mut write, read) = ws_stream.split();
        let msg = MyMessage::Register(self.name.clone());
        let data = serde_json::to_string::<MyMessage>(&msg).unwrap();
        write.send(Message::Text(data.into())).await?;
        self.write = Some(write);
        //let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SessionDescription>();
        let (tx_str, rx_str) = tokio::sync::mpsc::unbounded_channel::<String>();
        //self.rx = Some(rx);
        self.rx_data = Some(rx_str);

        //let (offer_tx, offer_rx) = tokio::sync::mpsc::unbounded_channel::<SessionDescription>();
        //let (answer_tx, answer_rx) = tokio::sync::mpsc::unbounded_channel::<SessionDescription>();
        //let tx2 = tx.clone();
        //self.rx_answer = Some(answer_rx);
        //self.rx_offer = Some(offer_rx);
        tokio::spawn(async move {
            Client::receive_data(read, tx_str).await;
        });
        //_=  tx.send(Command::Connected);
        return Ok(());
    }

    pub async fn wait_data(&mut self) -> Result<SessionDescription, anyhow::Error> {
        let rx = self.rx_data.as_mut().ok_or_else(|| anyhow!("no offer receiver"))?;
        match rx.recv().await {
            Some(data) => match serde_json::from_str::<MyMessage>(&data) {
                Ok(MyMessage::Offer(offer)) => Ok(offer),
                Ok(MyMessage::Answer(answer)) => Ok(answer),
                _ => Err(anyhow!("invalid message")),
            },
            None => Err(anyhow!("offer channel closed")),
        }
    }

    pub async fn wait_offer(&mut self) -> Result<SessionDescription, anyhow::Error> {
        let rx = self.rx_offer.as_mut().ok_or_else(|| anyhow!("no offer receiver"))?;
        match rx.recv().await {
            Some(sdp) => Ok(sdp),
            None => Err(anyhow!("offer channel closed")),
        }
    }

    pub async fn wait_answer(&mut self) -> Result<SessionDescription, anyhow::Error> {
        let rx = self.rx_answer.as_mut().ok_or_else(|| anyhow!("no answer receiver"))?;
        match rx.recv().await {
            Some(sdp) => Ok(sdp),
            None => Err(anyhow!("offer channel closed")),
        }
    }

    pub async fn wait_sdp(&mut self) -> Result<SessionDescription, anyhow::Error> {
        let rx = self.rx.as_mut()
        .ok_or_else(|| anyhow!("no sdp receiver"))?;
        match rx.recv().await {
            Some(sdp) => Ok(sdp),
            None => Err(anyhow!("channel closed")),
        }
    }

    /* pub async fn wait_offer(&self) -> Result<SessionDescription, anyhow::Error> {
        while let Some(ref sdp) = self.rx_offer.unwrap().recv().await {
            return Ok(sdp.clone());
        }
    } */

    pub async fn offer(&mut self, target: &str) -> Result<(), anyhow::Error> {
        if let Some(ref mut write) = self.write {
            let offer = MyMessage::Offer(SessionDescription {
                sender: self.name.clone(),
                target: target.to_string(),
                description: generate_description(32),
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
                description: generate_description(32),
            });
            let data = serde_json::to_string::<MyMessage>(&answer).unwrap();
            write.send(Message::Text(data.into())).await?;
        } else {
            return Err(anyhow!("write is None"));
        }
        Ok(())
    }

    pub async fn send_data(&mut self, target: &str, data: String) -> Result<(), anyhow::Error> {
        if let Some(ref mut write) = self.write {
            let answer = MyMessage::Answer(SessionDescription {
                sender: self.name.clone(),
                target: target.to_string(),
                description: data.clone(),
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

    async fn receive_data(
        mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, 
        sender: tokio::sync::mpsc::UnboundedSender<String>,
    ) {
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

    async fn _receive(
        mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, 
        sender: tokio::sync::mpsc::UnboundedSender<SessionDescription>,
        //tx_offer: tokio::sync::mpsc::UnboundedSender<SessionDescription>,
        //tx_answer: tokio::sync::mpsc::UnboundedSender<SessionDescription>,
    ) {
        while let Some(msg) = reader.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let data = text.to_string();
                    match serde_json::from_str::<MyMessage>(&data) {
                        Err(_) => {},
                        Ok(msg) => {
                            match msg {
                                MyMessage::Register(_) => {},
                                MyMessage::Answer(sdp) | MyMessage::Offer(sdp) => {
                                    _ = sender.send(sdp);
                                },
                            }
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

    /* async fn receive(
        mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, 
        //sender: tokio::sync::mpsc::UnboundedSender<MyMessage>,
        tx_offer: tokio::sync::mpsc::UnboundedSender<SessionDescription>,
        tx_answer: tokio::sync::mpsc::UnboundedSender<SessionDescription>,
    ) {
        while let Some(msg) = reader.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let data = text.to_string();
                    match serde_json::from_str::<MyMessage>(&data) {
                        Err(_) => {},
                        Ok(msg) => {
                            match msg {
                                MyMessage::Register(_) => {},
                                MyMessage::Answer(sdp) => {
                                    _ = tx_answer.send(sdp);
                                },
                                MyMessage::Offer(sdp) => {
                                    _ = tx_offer.send(sdp);
                                },
                            }
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
        
    } */
}