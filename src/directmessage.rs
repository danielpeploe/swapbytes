use libp2p::{request_response, swarm::NetworkBehaviour, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessage {
    pub message: String,
    pub sender: String,
}

#[derive(NetworkBehaviour)]
pub struct DirectMessageBehaviour {
    pub request_response:
        libp2p::request_response::cbor::Behaviour<DirectMessage, DirectMessageResponse>,
}

impl DirectMessageBehaviour {
    // Sends a direct message request to peer
    pub fn send_req(
        &mut self,
        peer_id: PeerId,
        request: DirectMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.request_response.send_request(&peer_id, request);
        Ok(())
    }

    // Processes a direct message request
    pub async fn handle_request(
        &mut self,
        request: DirectMessage,
        channel: request_response::ResponseChannel<DirectMessageResponse>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let whisper = request.clone();

        println!("{} [Whisper]: {}", whisper.sender, whisper.message);

        // Send response that displays message from their perspective and display in chat
        self.request_response
            .send_response(
                channel,
                DirectMessageResponse(format!("You [Whisper]: {}", whisper.message)),
            )
            .unwrap();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageResponse(pub String);
