use libp2p::swarm::NetworkBehaviour;
use libp2p::{request_response, PeerId};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(NetworkBehaviour)]
pub struct SendFileBehaviour {
    pub request_response: libp2p::request_response::cbor::Behaviour<FileRequest, FileResponse>,
}

impl SendFileBehaviour {
    // Sends a file request to peer
    pub fn send_req(
        &mut self,
        peer_id: PeerId,
        request: FileRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.request_response.send_request(&peer_id, request);
        Ok(())
    }

    // Processes a file request
    pub async fn process_req(
        &mut self,
        request: FileRequest,
        channel: request_response::ResponseChannel<FileResponse>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let filename = request.0.clone();
        println!("Received request for file: {}", filename);

        // Read file and send response
        match self.read_file(filename.clone()).await {
            Ok(bytes) => {
                let res = FileResponse {
                    filename,
                    data: bytes,
                };
                if let Err(e) = self.request_response.send_response(channel, res) {
                    eprintln!("Error sending response: {:?}", e);
                }
            }
            Err(e) => {
                eprintln!("Error selecting file: {:?}", e);
            }
        }
        Ok(())
    }

    // Reads the file
    async fn read_file(&self, filename: String) -> Result<Vec<u8>, std::io::Error> {
        let path = Path::new("files").join("bytebeam").join(&filename);
        let mut bytes = Vec::new();

        // Check if the file exists and is a valid file
        if !path.exists() || !path.is_file() {
            eprintln!("File does not exist or not valid - {:?}", path);
            return Ok(bytes);
        }

        // Open and read the file asynchronously
        let file_handle = File::open(&path).await;

        match file_handle {
            Ok(mut handle) => {
                if let Err(read_error) = handle.read_to_end(&mut bytes).await {
                    eprintln!("Failed to read file: {:?} - {}", path, read_error);
                }
            }
            Err(open_error) => {
                eprintln!("Failed to open file: {:?} - {}", path, open_error);
            }
        }
        Ok(bytes)
    }
}

// file exchange protocol for our app
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRequest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResponse {
    pub filename: String,
    pub data: Vec<u8>,
}
