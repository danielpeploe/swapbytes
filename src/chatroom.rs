use crate::commands;
use crate::directmessage::{DirectMessageBehaviour, DirectMessageBehaviourEvent};
use crate::network::{ChatBehaviour, ChatBehaviourEvent};
use crate::sendfile;
use crate::sendfile::SendFileBehaviourEvent;

use futures::stream::StreamExt;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::Mode;
use libp2p::kad::QueryId;
use libp2p::request_response;
use libp2p::StreamProtocol;
use libp2p::{gossipsub, kad, mdns, noise, swarm::SwarmEvent, tcp, yamux, PeerId};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::{io, io::AsyncBufReadExt, select};

// Function to initialize the swarm with required configs
fn initialize_swarm() -> Result<libp2p::Swarm<ChatBehaviour>, Box<dyn Error>> {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            Ok(ChatBehaviour {
                mdns: mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?,
                gossipsub: gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub::Config::default(),
                )?,
                kademlia: kad::Behaviour::new(
                    key.public().to_peer_id(),
                    MemoryStore::new(key.public().to_peer_id()),
                ),
                request_response: sendfile::SendFileBehaviour {
                    request_response: libp2p::request_response::cbor::Behaviour::new(
                        [(
                            StreamProtocol::new("/file-exchange/1"),
                            request_response::ProtocolSupport::Full,
                        )],
                        request_response::Config::default(),
                    ),
                },
                direct_message: DirectMessageBehaviour {
                    request_response: libp2p::request_response::cbor::Behaviour::new(
                        [(
                            StreamProtocol::new("/direct-message/1"),
                            request_response::ProtocolSupport::Full,
                        )],
                        request_response::Config::default(),
                    ),
                },
            })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Subscribe to the chat topic for gossipsub
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&gossipsub::IdentTopic::new("chat"))?;

    // Set Kademlia mode to server and set up listening on addresses
    swarm.behaviour_mut().kademlia.set_mode(Some(Mode::Server));
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    Ok(swarm)
}

// Handle publishing messages to the gossipsub topic
fn handle_publish_message(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    topic: &gossipsub::IdentTopic,
    line: &str,
) {
    if let Err(e) = swarm
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), line.as_bytes())
    {
        println!("Gossipsub publish error: {:?}", e);
    }
}

// Handle discovered peers through mDNS
fn handle_mdns_discovered(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    discovered_peers: Vec<(PeerId, libp2p::Multiaddr)>,
    nickname: &str,
    self_peer_id: &PeerId,
) {
    for (peer_id, _multiaddr) in discovered_peers {
        println!("New peer discovered: {peer_id}");

        // Add discovered peers to gossipsub and kademlia
        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, _multiaddr);

        // Store the peer's nickname in kademlia
        let name_store = kad::Record {
            key: kad::RecordKey::new(&self_peer_id.to_string()),
            value: nickname.as_bytes().to_vec(),
            publisher: None,
            expires: None,
        };

        if let Err(e) = swarm
            .behaviour_mut()
            .kademlia
            .put_record(name_store, kad::Quorum::One)
        {
            eprintln!("Failed to put record: {:?}", e);
        }
    }
}

// Handle peer expiration from mDNS
fn handle_mdns_expired(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    expired_peers: Vec<(PeerId, libp2p::Multiaddr)>,
) {
    for (peer_id, _multiaddr) in expired_peers {
        // Remove expired peers from gossipsub and kademlia
        println!("mDNS peer has expired: {peer_id}");
        swarm
            .behaviour_mut()
            .gossipsub
            .remove_explicit_peer(&peer_id);
        swarm
            .behaviour_mut()
            .kademlia
            .remove_address(&peer_id, &_multiaddr);
    }
}

// Handle the response of incoming messages from gossipsub
fn handle_gossipsub_message(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    peer_id: PeerId,
    msg_data: Vec<u8>,
    pending_queries: &mut HashMap<QueryId, (PeerId, String)>,
) {
    if let Ok(msg) = String::from_utf8(msg_data) {
        let query_id = swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&peer_id.to_string()));
        pending_queries.insert(query_id, (peer_id, msg));
    }
}

// Process outbound queries and manage record lookups
fn process_outbound_query(
    query_id: QueryId,
    result: kad::QueryResult,
    pending_queries: &mut HashMap<QueryId, (PeerId, String)>,
) {
    match result {
        kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(kad::PeerRecord {
            record: kad::Record { value, .. },
            ..
        }))) => {
            if let Some((peer_id, msg)) = pending_queries.remove(&query_id) {
                // Store query for future reference
                if let Ok(nickname) = std::str::from_utf8(&value) {
                    println!("{nickname}: {msg}");
                } else {
                    println!("{peer_id}: {msg}");
                }
            }
        }
        kad::QueryResult::GetRecord(Err(err)) => {
            println!("Failed to GET record. Err: {err:?}");
        }
        kad::QueryResult::PutRecord(Ok(kad::PutRecordOk { key: _ })) => {}
        kad::QueryResult::PutRecord(Err(err)) => {
            println!("Failed to PUT record. Err: {err:?}");
        }
        _ => {}
    }
}

// Handle file requests and responses for file transfers
async fn handle_send_file_event(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    send_file_event: SendFileBehaviourEvent,
) -> Result<(), Box<dyn Error>> {
    match send_file_event {
        SendFileBehaviourEvent::RequestResponse(request_response::Event::Message {
            message,
            ..
        }) => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                sendfile::SendFileBehaviour::process_req(
                    &mut swarm.behaviour_mut().request_response,
                    request,
                    channel,
                )
                .await?;
            }
            request_response::Message::Response { response, .. } => {
                let filename = format!(
                    "files/bytestream/{}",
                    response.filename.replace(&['/', '\\'][..], "_") // prevent path traversal
                );

                // Create bytebeam directory if it has been deleted
                if let Some(parent) = Path::new(&filename).parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Write the file and save to disc
                match File::create(&filename).await {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(&response.data).await {
                            println!("Failed to write to file: {}", e);
                        } else {
                            println!("Saved file to: {:?}", filename);
                        }
                    }
                    Err(e) => {
                        println!("Failed to create file: {}", e);
                    }
                }
                println!("Response: {:?}", response);
            }
        },
        SendFileBehaviourEvent::RequestResponse(request_response::Event::OutboundFailure {
            peer,
            request_id: _,
            error,
        }) => {
            println!("Failed to ByteBeam to peer: {:?}: {:?}", peer, error);
        }
        SendFileBehaviourEvent::RequestResponse(request_response::Event::InboundFailure {
            peer,
            request_id: _,
            error,
        }) => {
            println!(
                "ByteBeam was unsuccessful to peer: {:?}. Err: {:?}",
                peer, error
            );
        }
        SendFileBehaviourEvent::RequestResponse(request_response::Event::ResponseSent {
            peer,
            request_id: _,
        }) => {
            println!("ByteBeam was successful to peer: {:?}", peer);
        }
    }
    Ok(())
}

// Handle direct message events for whispering
async fn handle_direct_message_event(
    swarm: &mut libp2p::Swarm<ChatBehaviour>,
    direct_message_event: DirectMessageBehaviourEvent,
) -> Result<(), Box<dyn Error>> {
    match direct_message_event {
        DirectMessageBehaviourEvent::RequestResponse(request_response::Event::Message {
            message,
            ..
        }) => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                DirectMessageBehaviour::handle_request(
                    &mut swarm.behaviour_mut().direct_message,
                    request,
                    channel,
                )
                .await?;
            }
            request_response::Message::Response { response, .. } => {
                let message = response.0;
                println!("{}", message);
            }
        },
        _ => {}
    }
    Ok(())
}

// Main chat loop function
pub async fn start_chat(nickname: String) -> Result<(), Box<dyn Error>> {
    let mut swarm = initialize_swarm()?;
    let self_peer_id = swarm.local_peer_id().clone();
    let topic = gossipsub::IdentTopic::new("chat");
    let mut pending_queries: HashMap<QueryId, (PeerId, String)> = HashMap::new();
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    println!("Enter chat messages one line at a time:");

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                if line.starts_with("/") {
                    if let Err(err) = commands::process_command(line, &mut swarm, self_peer_id) {
                        println!("Error processing command: {:?}", err);
                    }
                } else {
                    handle_publish_message(&mut swarm, &topic, &line);
                }
            }

            // Handle swarm events
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                    handle_mdns_discovered(&mut swarm, peers, &nickname, &self_peer_id);
                }
                SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                    handle_mdns_expired(&mut swarm, peers);
                }
                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: _,
                    message,
                })) => {
                    handle_gossipsub_message(&mut swarm, peer_id, message.data, &mut pending_queries);
                }
                SwarmEvent::Behaviour(ChatBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { id, result, .. })) => {
                    process_outbound_query(id, result, &mut pending_queries);
                }
                SwarmEvent::Behaviour(ChatBehaviourEvent::RequestResponse(send_file_event)) => {
                    handle_send_file_event(&mut swarm, send_file_event).await?;
                }
                SwarmEvent::Behaviour(ChatBehaviourEvent::DirectMessage(direct_message_event)) => {
                    handle_direct_message_event(&mut swarm, direct_message_event).await?;
                }
                _ => {}
            }
        }
    }
}
