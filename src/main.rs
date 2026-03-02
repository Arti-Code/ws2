use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
//use tokio_tungstenite::tungstenite::http::version;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use ws::command::MyMessage;
use std::collections::HashMap;
use colored::*;


//type Clients = Arc<RwLock<HashMap<SocketAddr, tokio::sync::mpsc::UnboundedSender<Message>>>>;
type Clients = Arc<RwLock<HashMap<SocketAddr, Client>>>;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut version = String::from("version: ");
    version.push_str(std::env!("CARGO_PKG_VERSION"));
    let author= std::env!("CARGO_PKG_AUTHORS");
    println!("{}", "-=SIGNALING SERVER=-".to_string().bold().underline().cyan());
    println!("{}", version.cyan());
    println!("{}", author.blue());
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("");
    println!("{}{}", "listening on: ".to_string().bold().bright_green(), addr.bold().bright_green());

    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, clients.clone()));
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    clients: Clients,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}{}", "new connection: ".green().bold(), addr.to_string().green().bold());

    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let client=  Client::new(tx);
    // Store the client
    clients.write().await.insert(addr, client);

    // Spawn task to handle outgoing messages
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    let mut sender = format!("[{}]: ", addr.to_string());
    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        if let Some(name) = clients.read().await.get(&addr).unwrap().name() {
            sender = format!("[{}]: ", name);
        }
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<MyMessage>(&text.clone().to_string()) {
                    Ok(cmd) => {
                        match cmd {
                            MyMessage::Register(name) => {
                                clients.write().await.entry(addr).and_modify(|c| c.set_name(name.as_str()));
                                println!("{}{}{}", sender.bold(), "registered as ".to_string().yellow(), name.bold().yellow());
                            },
                            MyMessage::Offer(sdp) => {
                                let target = sdp.target;
                                let client_list = clients.read().await;
                                for (addr, client) in client_list.iter() {
                                    if let Some(client_name) = client.name() {
                                        if target == client_name {
                                            send_message(&clients, Message::Text(text), *addr).await;
                                            println!("[OFFER]: {}{}{}", sender.bold(), " ==> ".to_string(), target.bold());
                                            break;
                                        }
                                    }
                                }
                            },
                            MyMessage::Answer(sdp) => {
                                let target = sdp.target;
                                let client_list = clients.read().await;
                                for (addr, client) in client_list.iter() {
                                    if let Some(client_name) = client.name() {
                                        if target == client_name {
                                            send_message(&clients, Message::Text(text), *addr).await;
                                            println!("[ANSWER]: {}{}{}", sender.bold(), " ==> ".to_string(), target.bold());
                                            break;
                                        }
                                    }
                                }
                            },
                        }
                    },
                    Err(_) => {
                        println!("{}{}", sender.bold(), text.to_string());
                        broadcast_message(&clients, Message::Text(text), addr).await;
                    }
                }
                
            }
            Ok(Message::Binary(bin)) => {
                println!("{}[BYTES] {}", sender.bold(), bin.len());
                broadcast_message(&clients, Message::Binary(bin), addr).await;
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

    // Clean up
    send_task.abort();
    clients.write().await.remove(&addr);
    println!("{}{}", sender.bold(), "removed".to_string().bright_red());

    Ok(())
}

async fn broadcast_message(clients: &Clients, msg: Message, sender: SocketAddr) {
    let clients = clients.read().await;
    for (addr, tx) in clients.iter() {
        if *addr == sender {
            tx.send(msg.clone()).ok();
        }
    }
}

async fn send_message(clients: &Clients, msg: Message, receiver: SocketAddr) {
    let clients = clients.read().await;
    for (addr, tx) in clients.iter() {
        if *addr == receiver {
            tx.send(msg.clone()).ok();
        }
    }
}


struct Client {
    name: Option<String>,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl Client {
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