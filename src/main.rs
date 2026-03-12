use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use signaler::command::{DescriptionType, SignalMessage};
use std::collections::HashMap;

type Peers = Arc<RwLock<HashMap<SocketAddr, Peer>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("{}{}", "listening on: ", addr);
    let clients: Peers = Arc::new(RwLock::new(HashMap::new()));
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, clients.clone()));
    }
    Ok(())
}

async fn handle_connection(stream: TcpStream, addr: SocketAddr, clients: Peers) 
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let s = format!("{}{}", "new connection: ", addr.to_string());
    println!("{}", s);
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let client = Peer::new(tx, false);
    clients.write().await.insert(addr, client);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() { break; }
        }
    });

    let mut sender = format!("{} ", addr.to_string());
    while let Some(msg) = ws_receiver.next().await {
        if let Some(name) = clients.read().await.get(&addr).unwrap().name() {
            sender = format!("[{}]: ", name);
        }
        let mut s = String::new();
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<SignalMessage>(&text.clone().to_string()) {
                    Ok(cmd) => {
                        match cmd {
                            SignalMessage::Register(name) => {
                                clients.write().await.entry(addr)
                                .and_modify(|c| c.set_name(name.as_str()));
                                s = format!("{}{}{}", sender, "registered as ".to_string(), name);
                                println!("{}", s);
                            },
                            SignalMessage::SetLogger => {
                                clients.write().await.entry(addr)
                                .and_modify(|c| c.set_logger(true));
                                s = format!("{}{}", sender, "set as logger");
                                println!("{}", s);
                            },
                            SignalMessage::SessionDescription(sdp) => {
                                let target = sdp.target;
                                let kind = match sdp.kind {
                                    DescriptionType::Offer => "OFFER".to_string(),
                                    DescriptionType::Answer => "ANSWER".to_string(),
                                };
                                let client_list = clients.read().await;
                                for (addr, client) in client_list.iter() {
                                    if let Some(client_name) = client.name() {
                                        if target == client_name {
                                            send_message(&clients, Message::Text(text), *addr).await;
                                            s = format!("{}{}{}{}{}", sender, " ==".to_string(), kind, "==> ".to_string(), target);
                                            println!("{}", s);
                                            break;
                                        }
                                    }
                                }
                            },
                            SignalMessage::Echo(echo) => {
                                s = format!("{}{}", sender, echo.to_string());
                                println!("{}", s);
                                send_message(&clients, Message::Text(echo.into()), addr).await;
                            },
                            SignalMessage::Text(text_msg) => {
                                let target = text_msg.target;
                                let client_list = clients.read().await;
                                for (addr, client) in client_list.iter() {
                                    if let Some(client_name) = client.name() {
                                        if target == client_name {
                                            send_message(&clients, Message::Text(text_msg.message.into()), *addr).await;
                                            s = format!("{}{}{}", sender, " ==[message]==> ".to_string(), target);
                                            println!("{}", s);
                                            break;
                                        }
                                    }
                                }
                            },
                        }
                    },
                    Err(_) => {
                        s = format!("{}{}", sender, text.to_string());
                        println!("{}", s);
                        send_message(&clients, Message::Text(text), addr).await;
                    }
                }
            }
            Ok(Message::Binary(bin)) => {
                s = format!("{}[BYTES] {}", sender, bin.len());
                println!("{}", s);
                send_message(&clients, Message::Binary(bin), addr).await;
            }
            Ok(Message::Close(_)) => {
                s = format!("{}{}", sender, "disconnected");
                println!("{}", s);
                break;
            }
            Ok(Message::Ping(data)) => {
                s = format!("{}{}", sender, "ping");
                if let Some(tx) = clients.read().await.get(&addr) {
                    tx.send(Message::Pong(data)).ok();
                }
            }
            Ok(Message::Pong(_)) => {
                s = format!("{}{}", sender, "pong");
            }
            Ok(Message::Frame(frame)) => {
                if let Ok(text) = frame.into_text() {
                    s = format!("{}[FRAME] {}", sender, text.to_string());
                    println!("{}", s);
                    if let Some(tx) = clients.read().await.get(&addr) {
                        tx.send(Message::Text(text)).ok();
                    }
                }
            }
            Err(e) => {
                s = format!("{}{}", sender, e);
                println!("{}", s);
                break;
            }
        }
        if !s.is_empty() { broadcast_log(&clients, &s.clone()).await; }
    }

    let s = format!("{}{}", sender, "removed".to_string());
    broadcast_log(&clients, &s.clone()).await;
    println!("{}", s);
    send_task.abort();
    clients.write().await.remove(&addr);
    Ok(())
}

async fn send_message(clients: &Peers, msg: Message, receiver: SocketAddr) {
    let clients = clients.read().await;
    for (addr, tx) in clients.iter() {
        if *addr == receiver {
            tx.send(msg.clone()).ok();
        }
    }
}

async fn broadcast_log(clients: &Peers, log: &str) {
    let clients = clients.read().await;
    for (_, peer) in clients.iter() {
        if peer.is_logger() {
            peer.send(Message::Text(log.into())).ok();
        }
    }
}

fn init() {
    let mut version = String::from("version: ");
    version.push_str(std::env!("CARGO_PKG_VERSION"));
    let author= std::env!("CARGO_PKG_AUTHORS");
    println!("{}", "-=SIGNALING SERVER=-".to_string());
    println!("{}", version);
    println!("{}", author);
    println!("");
}

struct Peer {
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