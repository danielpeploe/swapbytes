mod chatroom;
mod commands;
mod directmessage;
mod network;
mod sendfile;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Entry point for the application. User is not connected until they enter a nickname.
    println!("Welcome to SwapBytes! Please enter your nickname to continue.");
    println!("Enter your name:");
    let mut input_name = String::new();
    std::io::stdin().read_line(&mut input_name)?;
    let nickname = input_name.trim().to_string();

    chatroom::start_chat(nickname).await?;

    Ok(())
}
