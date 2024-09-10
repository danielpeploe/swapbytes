use crate::directmessage::DirectMessageBehaviour;
use crate::sendfile::SendFileBehaviour;

use libp2p::kad::store::MemoryStore;
use libp2p::{gossipsub, kad, mdns, swarm::NetworkBehaviour};

// Network behaviour for the chat application
#[derive(NetworkBehaviour)]
pub struct ChatBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub request_response: SendFileBehaviour,
    pub direct_message: DirectMessageBehaviour,
}
