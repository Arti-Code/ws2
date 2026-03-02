
use anyhow::Result;
use ws::client::Client;
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_title();
    println!("");
    let name = input_name();
    let target = input_target();
    let url = "ws://127.0.0.1:8080";
    let mut client = Client::new(&name, url);
    client.connect().await.expect("connection error");
    client.offer(&target).await?;
    let msg = format!("[OFFER] ==> {}", target);
    println!("{}", msg.bold().cyan());
    let answer = client.wait_sdp().await.unwrap();
    let msg = format!("[ANSWER] <== {}", answer.sender);
    println!("{}", msg.bold().green());
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    Ok(())
}

fn input_name() -> String {
    println!("enter your name:");
    let mut name = String::new();
    std::io::stdin().read_line(&mut name).expect("Failed to read line");
    name.trim().to_string()
}

fn input_target() -> String {
    println!("enter target:");
    let mut name = String::new();
    std::io::stdin().read_line(&mut name).expect("Failed to read line");
    name.trim().to_string()
}

fn print_title() {
    println!("{}", "WebRTC Offer Client".bold().cyan());
}