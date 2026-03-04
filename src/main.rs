use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
//use tokio_tungstenite::tungstenite::http::version;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use signaler::command::{DescriptionType, SignalMessage};
use std::collections::HashMap;
use colored::*;


//type Clients = Arc<RwLock<HashMap<SocketAddr, tokio::sync::mpsc::UnboundedSender<Message>>>>;
type Peers = Arc<RwLock<HashMap<SocketAddr, Peer>>>;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("{}{}", "listening on: ".to_string().bold().bright_green(), addr.bold().bright_green());

    let clients: Peers = Arc::new(RwLock::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, clients.clone()));
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    clients: Peers,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}{}", "new connection: ".green().bold(), addr.to_string().green().bold());

    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let client=  Peer::new(tx);
    clients.write().await.insert(addr, client);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut sender = format!("{} ", addr.to_string());
    while let Some(msg) = ws_receiver.next().await {
        if let Some(name) = clients.read().await.get(&addr).unwrap().name() {
            sender = format!("[{}]: ", name);
        }
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<SignalMessage>(&text.clone().to_string()) {
                    Ok(cmd) => {
                        match cmd {
                            SignalMessage::Register(name) => {
                                clients.write().await.entry(addr).and_modify(|c| c.set_name(name.as_str()));
                                println!("{}{}{}", sender.bold(), "registered as ".to_string().yellow(), name.bold().yellow());
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
                                            println!("{}{}{}{}{}", sender.bold(), " ==".to_string(), kind, "==> ".to_string(), target.bold());
                                            break;
                                        }
                                    }
                                }
                            },
                            SignalMessage::Echo(s) => {
                                println!("{}{}", sender.bold(), s.to_string());
                                send_message(&clients, Message::Text(s.into()), addr).await;
                            },
                            SignalMessage::Text(text_msg) => {
                                let target = text_msg.target;
                                let client_list = clients.read().await;
                                for (addr, client) in client_list.iter() {
                                    if let Some(client_name) = client.name() {
                                        if target == client_name {
                                            send_message(&clients, Message::Text(text_msg.message.into()), *addr).await;
                                            println!("{}{}{}", sender.bold(), " ==[message]==> ".to_string(), target.bold());
                                            break;
                                        }
                                    }
                                }
                            },
                        }
                    },
                    Err(_) => {
                        println!("{}{}", sender.bold(), text.to_string());
                        send_message(&clients, Message::Text(text), addr).await;
                    }
                }
                
            }
            Ok(Message::Binary(bin)) => {
                println!("{}[BYTES] {}", sender.bold(), bin.len());
                send_message(&clients, Message::Binary(bin), addr).await;
            }
            Ok(Message::Close(_)) => {
                println!("{}{}", sender.bold(), "disconnected".red().bold());
                break;
            }
            Ok(Message::Ping(data)) => {
                if let Some(tx) = clients.read().await.get(&addr) {
                    tx.send(Message::Pong(data)).ok();
                }
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Frame(frame)) => {
                if let Ok(text) = frame.into_text() {
                    println!("{}[FRAME] {}", sender.bold(), text.to_string());
                    if let Some(tx) = clients.read().await.get(&addr) {
                        tx.send(Message::Text(text)).ok();
                    }
                }
            }
            Err(e) => {
                eprintln!("{}{}", sender.bold().red(), e);
                break;
            }
        }
    }

    send_task.abort();
    clients.write().await.remove(&addr);
    println!("{}{}", sender.bold(), "removed".to_string().bright_red());

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

fn init() {
    let mut version = String::from("version: ");
    version.push_str(std::env!("CARGO_PKG_VERSION"));
    let author= std::env!("CARGO_PKG_AUTHORS");
    println!("{}", "-=SIGNALING SERVER=-".to_string().bold().underline().cyan());
    println!("{}", version.cyan());
    println!("{}", author.blue());
    println!("");
}

struct Peer {
    name: Option<String>,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl Peer {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<Message>) -> Self {
        Self {
            name: None,
            sender,
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
}