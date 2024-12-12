use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Peer Wire Protocol Message Types
#[derive(Debug)]
enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        piece_index: u32,
    },
    Bitfield {
        bitfield: Vec<bool>,
    },
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

pub struct PeerConnection {
    stream: TcpStream,
    peer_id: [u8; 20],
    pub bitfield: Vec<bool>,
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
}

impl PeerConnection {
    pub fn new(stream: TcpStream, peer_id: [u8; 20]) -> Self {
        // let (sr, sw) = stream.split();
        PeerConnection {
            stream,
            peer_id,
            bitfield: Vec::new(),
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
        }
    }

    pub async fn handshake(&mut self, info_hash: &[u8; 20]) -> Result<()> {
        // Protocol handshake implementation
        let protocol_str = b"BitTorrent protocol";
        let mut handshake = Vec::with_capacity(68);

        handshake.push(protocol_str.len() as u8);
        handshake.extend_from_slice(protocol_str);
        handshake.extend_from_slice(&[0; 8]); // Reserved bytes
        handshake.extend_from_slice(info_hash);
        handshake.extend_from_slice(&self.peer_id);

        self.stream.write_all(&handshake).await?;
        self.stream.flush().await?;

        Ok(())
    }

    async fn read_hanshake(&mut self) -> Result<()> {
        // Read and validate response
        let mut response: [u8; 68] = [0u8; 68];
        self.stream
            .read_exact(&mut response)
            .await
            .context("Failed to read handshake answer")?;

        let protocol_length = response[0];
        anyhow::ensure!(protocol_length == 19);

        let protocol: [u8; 19] = response[1..20];
        if response[1..20] != b"BitTorrent protocol" {}

        Ok(())
    }

    pub async fn send_message(&mut self, message: PeerMessage) -> Result<()> {
        let mut payload = Vec::new();

        match message {
            PeerMessage::Choke => payload.push(0),
            PeerMessage::Unchoke => payload.push(1),
            PeerMessage::Interested => payload.push(2),
            PeerMessage::NotInterested => payload.push(3),
            PeerMessage::Have { piece_index } => {
                payload.push(4);
                payload.extend_from_slice(&piece_index.to_be_bytes());
            }
            PeerMessage::Bitfield { bitfield } => {
                payload.push(5);
                payload.extend(bitfield.iter().map(|&b| b as u8));
            }
            PeerMessage::Request {
                index,
                begin,
                length,
            } => {
                payload.push(6);
                payload.extend_from_slice(&index.to_be_bytes());
                payload.extend_from_slice(&begin.to_be_bytes());
                payload.extend_from_slice(&length.to_be_bytes());
            }
            PeerMessage::Piece {
                index,
                begin,
                block,
            } => {
                payload.push(7);
                payload.extend_from_slice(&index.to_be_bytes());
                payload.extend_from_slice(&begin.to_be_bytes());
                payload.extend_from_slice(&block);
            }
            PeerMessage::Cancel {
                index,
                begin,
                length,
            } => {
                payload.push(8);
                payload.extend_from_slice(&index.to_be_bytes());
                payload.extend_from_slice(&begin.to_be_bytes());
                payload.extend_from_slice(&length.to_be_bytes());
            }
        }

        // Send length-prefixed message
        let length = (payload.len() as u32).to_be_bytes();
        self.stream.write_all(&length).await?;
        self.stream.write_all(&payload).await?;
        self.stream.flush().await?;

        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<PeerMessage> {
        // Read message length
        let mut length_bytes = [0u8; 4];
        self.stream.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes);

        if length == 0 {
            return Ok(PeerMessage::Choke); // Keep-alive message
        }

        // Read message type
        let mut msg_type = [0u8; 1];
        self.stream.read_exact(&mut msg_type).await?;

        match msg_type[0] {
            0 => Ok(PeerMessage::Choke),
            1 => Ok(PeerMessage::Unchoke),
            2 => Ok(PeerMessage::Interested),
            3 => Ok(PeerMessage::NotInterested),
            4 => {
                let mut piece_index_bytes = [0u8; 4];
                self.stream.read_exact(&mut piece_index_bytes).await?;
                let piece_index = u32::from_be_bytes(piece_index_bytes);
                Ok(PeerMessage::Have { piece_index })
            }
            5 => {
                let mut bitfield = vec![false; length as usize * 8];
                // Read bitfield and convert to boolean array
                for i in 0..length as usize {
                    let mut byte = [0u8; 1];
                    self.stream.read_exact(&mut byte).await?;
                    for j in 0..8 {
                        bitfield[i * 8 + j] = (byte[0] & (1 << (7 - j))) != 0;
                    }
                }
                Ok(PeerMessage::Bitfield { bitfield })
            }
            6 => {
                let mut index_bytes = [0u8; 4];
                let mut begin_bytes = [0u8; 4];
                let mut length_bytes = [0u8; 4];

                self.stream.read_exact(&mut index_bytes).await?;
                self.stream.read_exact(&mut begin_bytes).await?;
                self.stream.read_exact(&mut length_bytes).await?;

                Ok(PeerMessage::Request {
                    index: u32::from_be_bytes(index_bytes),
                    begin: u32::from_be_bytes(begin_bytes),
                    length: u32::from_be_bytes(length_bytes),
                })
            }
            7 => {
                let mut index_bytes = [0u8; 4];
                let mut begin_bytes = [0u8; 4];

                self.stream.read_exact(&mut index_bytes).await?;
                self.stream.read_exact(&mut begin_bytes).await?;

                let block_length = length as usize - 8;
                let mut block = vec![0u8; block_length];
                self.stream.read_exact(&mut block).await?;

                Ok(PeerMessage::Piece {
                    index: u32::from_be_bytes(index_bytes),
                    begin: u32::from_be_bytes(begin_bytes),
                    block,
                })
            }
            8 => {
                let mut index_bytes = [0u8; 4];
                let mut begin_bytes = [0u8; 4];
                let mut length_bytes = [0u8; 4];

                self.stream.read_exact(&mut index_bytes).await?;
                self.stream.read_exact(&mut begin_bytes).await?;
                self.stream.read_exact(&mut length_bytes).await?;

                Ok(PeerMessage::Cancel {
                    index: u32::from_be_bytes(index_bytes),
                    begin: u32::from_be_bytes(begin_bytes),
                    length: u32::from_be_bytes(length_bytes),
                })
            }
            _ => anyhow::bail!("Unknown message type"),
        }
    }

    pub async fn download_piece(&mut self, piece_index: usize) -> Result<Vec<u8>> {
        // Mark interested and wait for unchoke
        self.send_message(PeerMessage::Interested).await?;

        // Wait for unchoke
        loop {
            match self.receive_message().await? {
                PeerMessage::Unchoke => break,
                _ => continue,
            }
        }

        // Download piece in blocks
        let piece_length = 16 * 1024; // Standard block size
        let mut piece_data = Vec::new();

        for block_offset in (0..piece_length).step_by(piece_length) {
            // Request block
            self.send_message(PeerMessage::Request {
                index: piece_index as u32,
                begin: block_offset as u32,
                length: piece_length as u32,
            })
            .await?;

            // Receive block
            match self.receive_message().await? {
                PeerMessage::Piece {
                    index,
                    begin,
                    block,
                } => {
                    if index as usize == piece_index && begin as usize == block_offset {
                        piece_data.extend_from_slice(&block);
                    }
                }
                _ => continue,
            }
        }

        Ok(piece_data)
    }
}
