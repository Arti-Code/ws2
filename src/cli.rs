
use std::env::args;
use std::time::Duration;

use anyhow::Result;
use ws::client::Client;
use ws::command::*;
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = args().collect();
    let name = match args.get(1) {
        None => "UNKNOWN",
        Some(name) => name,
    };
    let target = match args.get(2) {
        None => "UNKNOWN",
        Some(target) => target,
    };
    /* let url = match args.get(1) {
        None => "ws://127.0.0.1:8080",
        Some(addr) => addr,
    }; */
    let url = "ws://127.0.0.1:8080";
    let mut client = Client::new(name, url);
    let mut recv = client.connect().await.expect("connection error");

    
    tokio::spawn(async move {
        let sdp = client.wait_offer().await.unwrap();
        println!("{} from {} ({})", "[OFFER]: ".bold().cyan(), sdp.sender, sdp.description);
    }).await;
    /* tokio::spawn(async move {
        while let Some(msg) = recv.recv().await {
            match msg {
                MyMessage::Offer(sdp) => {
                    println!("{} from {} ({})", "[OFFER]: ".bold().cyan(), sdp.sender, sdp.description);
                },
                MyMessage::Answer(sdp) => {
                    println!("{} from {} ({})", "[ANSWER]: ".bold().green(), sdp.sender, sdp.description);
                }
                MyMessage::Register(_) => {},
                
            }
        }
    }); */
    
    //client.pinging(3).await?;
    loop {
        client.offer(target).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;

    }

    //Ok(())
}