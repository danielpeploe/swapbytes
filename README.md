# Swapbytes

SwapBytes is a peer-to-peer chat application that allows users to communicate and share files over a decentralized network using the libp2p rust library.

Users can share and receive files using the 'ByteBeam' and 'ByteStream'.

## Features

- Peer discovery using mDNS
- Message broadcasting using Gossipsub
- File sharing using a request-response protocol
- Direct messaging using a request-response protocol

## How to run

- Use the command `cargo run`

## How to use

Upon starting the application. Enter a nickname.
Once a peer has been discovered you can begin chatting!

## Commands

- /bytestream <peer_id> <filename>: Request a file from a specified peer
- /whisper <peer_id> <message>: Privately direct message a specified peer
- /id: Reveal your peer id

## Examples

### Receiving a file

When a peer has been discovered you will see the message New peer discovered: <peer_id>

Use the command for bytestream (/bytestream <peer_id> <filename>) and if the peer has the file located in their 'ByteBeam' it will be beamed to you!

Example:

- New peer discovered: 12D3KooWEUkDTFYoUpbADwjXQP2PGRFFzerrHQ9EFuu3F9GTpurL
- /bytestream 12D3KooWEUkDTFYoUpbADwjXQP2PGRFFzerrHQ9EFuu3F9GTpurL test.txt
- Sent bytestream for test.txt to peer: 12D3KooWEUkDTFYoUpbADwjXQP2PGRFFzerrHQ9EFuu3F9GTpurL
- Saved file to: "files/bytestream/test.txt"
- Response: FileResponse { filename: "test.txt", data: [84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 116, 101, 115, 116, 32, 102, 105, 108, 101, 46] }

### Whispering to a peer

To privately message a peer use the whisper command (/whisper <peer_id> <message>) and the peer will be privately messaged in the chat. Other members will not be able to see the message!

Example:

- New peer discovered: 12D3KooWDmacAcS2EvSfv1mXPaqS4Lx4qQ9pRMZFXPU3ZxbM9oux
- /whisper 12D3KooWDmacAcS2EvSfv1mXPaqS4Lx4qQ9pRMZFXPU3ZxbM9oux Hello. This is private
- You [Whisper]: Hello. This is private

### Finding out your peer id

Incase you need to remind someone of your peer id. Use the peer id command (/id). You id will be revealed to just you. From here you can (privately) message it a peer.

- Your peer id: 12D3KooWCcffVLfowr9Tf5XrH8zrFuWTtRs77hyMyZbJzcKbFK9f
