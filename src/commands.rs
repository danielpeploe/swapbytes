use crate::directmessage::DirectMessage;
use crate::network::ChatBehaviour;
use crate::sendfile::FileRequest;

use libp2p::PeerId;
use regex::Regex;
use std::error::Error;
use std::str::FromStr;

// Processes commands entered by the user
pub fn process_command(
    line: String,
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    self_peer_id: PeerId,
) -> Result<(), Box<dyn Error>> {
    // Get the command and arguments
    let args = Regex::new(r#""[^"]*"|\S+"#)
        .unwrap()
        .captures_iter(&line)
        .map(|cap| cap.get(0).unwrap().as_str().to_string())
        .collect::<Vec<String>>();

    let cmd = args[0].as_str();

    match cmd {
        "/bytestream" => {
            if args.len() != 3 {
                println!("Usage: /bytestream <peer_id> <filename>");
                return Ok(());
            }
            process_req(swarm, args)?;
        }
        "/whisper" => {
            if args.len() < 3 {
                println!("Usage: /whisper <peer_id> <message>");
                return Ok(());
            }
            process_whisper(swarm, args, self_peer_id)?;
        }
        "/id" => {
            println!("Your peer id: {}", self_peer_id);
        }
        _ => {
            println!("Invalid command: {}", cmd);
        }
    }
    Ok({})
}

// Handles the /whisper command for sending a private message
fn process_whisper(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    args: Vec<String>,
    self_peer_id: PeerId,
) -> Result<(), Box<dyn Error + 'static>> {
    let other_peer_id = match PeerId::from_str(&args[1]) {
        Ok(peer_id) => peer_id,
        Err(err) => {
            eprintln!("Peer '{}' does not exist. Err: {}", &args[1], err);
            return Ok(());
        }
    };

    // Create message and send it
    match swarm.behaviour_mut().direct_message.send_req(
        other_peer_id,
        DirectMessage {
            sender: self_peer_id.to_string(),
            message: args[2..].join(" "),
        },
    ) {
        Ok(_) => (),
        Err(e) => eprintln!("Whisper failed. Err: {:?}", e),
    }
    Ok(())
}

// Handles the /bytestream command for requesting a file transfer
fn process_req(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let other_peer_id = match PeerId::from_str(&args[1]) {
        Ok(peer_id) => peer_id,
        Err(err) => {
            eprintln!("Peer '{}' does not exist. Err: {}", &args[1], err);
            return Ok(());
        }
    };

    // Send the request
    swarm
        .behaviour_mut()
        .request_response
        .send_req(other_peer_id, FileRequest(args[2].to_string()))?;
    println!(
        "Sent bytestream for {} to peer: {}",
        &args[2], other_peer_id
    );
    Ok(())
}
