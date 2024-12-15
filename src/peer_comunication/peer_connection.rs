use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::peer_comunication::bitfield::Bitfield;
use crate::peer_comunication::handshake::{Handshake, BITTORRENT_PROTOCOL};
use crate::peer_comunication::peer_msg::PeerMessage;
use crate::piece::{Piece, PieceData};

pub(crate) const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BLOCK_SIZE: usize = 1024; //16384;

/// Structure representing all informations about P2P connection with one peer.
#[allow(unused)]
pub struct PeerConnection {
    stream: TcpStream,
    peer_id: [u8; 20],
    bitfield: Mutex<Bitfield>,
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
    piece_channel: Sender<PieceData>,
    total_pieces: usize,
    piece_pool: Arc<Mutex<HashMap<usize, Piece>>>,
}

impl PeerConnection {
    /// Create a new bittorent conection with peer, with wich TCP connection was already done.
    /// Exchange handshake with other pear, and try to get bitfield of pieces from second
    pub async fn new(
        mut stream: TcpStream,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        piece_count: usize,
        sender: Sender<PieceData>,
        piece_pool: Arc<Mutex<HashMap<usize, Piece>>>,
    ) -> Result<Self> {
        // Protocol handshake implementation
        let mut handshake = Handshake::new(&info_hash, &peer_id);
        let _ = timeout(TIMEOUT, stream.write_all(&handshake.get_bytes()))
            .await
            .context("Failed to write handshake")?;
        let _ = timeout(TIMEOUT, stream.flush())
            .await
            .context("Failed to flush handshake")?;

        let mut response: [u8; 68] = [0u8; 68];
        timeout(TIMEOUT, stream.read_exact(&mut response))
            .await
            .context("Failed to read handshake answer")??;

        handshake.set_bytes(&response);
        anyhow::ensure!(handshake.length == 19);
        anyhow::ensure!(handshake.bittorrent == BITTORRENT_PROTOCOL);
        anyhow::ensure!(handshake.info_hash == info_hash);

        let mut peer_conn = PeerConnection {
            stream,
            peer_id: handshake.peer_id,
            bitfield: Mutex::new(Bitfield::empty_with_piece_capacity(piece_count)),
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
            piece_channel: sender,
            total_pieces: piece_count,
            piece_pool,
        };

        peer_conn.try_get_bitfield().await?;

        Ok(peer_conn)
    }

    /// Get information about at least one piece that other peer has, and waiting for `unchoke` message.
    pub async fn try_get_bitfield(&mut self) -> Result<()> {
        timeout(TIMEOUT, self.send_message(PeerMessage::Interested)).await??;

        let mut get_piece_info = false;
        loop {
            match self.receive_message().await? {
                PeerMessage::Unchoke => {
                    if get_piece_info {
                        break;
                    }
                }
                PeerMessage::Bitfield { .. } => {
                    get_piece_info = true;
                    if !self.am_choking {
                        break;
                    }
                    continue;
                }
                PeerMessage::Have { .. } => {
                    get_piece_info = true;
                    if !self.am_choking {
                        break;
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Send message to other peer.
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
                payload.extend(bitfield.as_bytes());
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

        // Send prepared payload
        let length = (payload.len() as u32).to_be_bytes();
        let mut msg = Vec::new();
        msg.extend_from_slice(&length);
        msg.extend(payload);
        self.stream.write_all(&msg).await?;
        self.stream.flush().await?;

        Ok(())
    }

    /// Receive message from other peer.
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
            0 => {
                self.am_choking = true;
                Ok(PeerMessage::Choke)
            }
            1 => {
                self.am_choking = false;
                Ok(PeerMessage::Unchoke)
            }
            2 => {
                self.peer_interested = true;
                Ok(PeerMessage::Interested)
            }
            3 => {
                self.peer_interested = false;
                Ok(PeerMessage::NotInterested)
            }
            4 => {
                let mut piece_index_bytes = [0u8; 4];
                self.stream.read_exact(&mut piece_index_bytes).await?;
                let piece_index = u32::from_be_bytes(piece_index_bytes);
                self.bitfield.lock().await.set_piece(piece_index as usize);
                Ok(PeerMessage::Have { piece_index })
            }
            5 => {
                let mut bitfield = Vec::new();
                for _ in 0..length as usize {
                    let mut byte = [0u8; 1];
                    self.stream.read_exact(&mut byte).await?;
                    bitfield.push(byte[0]);
                }
                self.bitfield = Mutex::new(Bitfield::new(bitfield.clone()));
                Ok(PeerMessage::Bitfield {
                    bitfield: Bitfield::new(bitfield),
                })
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

                let block_length = length as usize - 9;
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

    /// Download one piece with given index from other peer.
    pub async fn download_piece(&mut self, piece: Piece) -> Result<()> {
        let piece_index = piece.index();
        let piece_length = piece.length();

        if self.am_choking {
            // Wait for unchoke
            loop {
                match timeout(Duration::from_secs(60), self.receive_message()).await?? {
                    PeerMessage::Unchoke => break,
                    _ => {
                        continue;
                    }
                }
            }
        }

        // Download piece in blocks
        let mut piece_data = Vec::new();
        for block_offset in (0..piece_length).step_by(MAX_BLOCK_SIZE) {
            let remaining = piece.length() - block_offset;
            let this_block_len = MAX_BLOCK_SIZE.min(remaining);

            // Sending request for block
            timeout(
                TIMEOUT,
                self.send_message(PeerMessage::Request {
                    index: piece_index as u32,
                    begin: block_offset as u32,
                    length: this_block_len as u32,
                }),
            )
            .await??;

            // Receiving block
            loop {
                match timeout(Duration::from_secs(60), self.receive_message()).await?? {
                    PeerMessage::Piece {
                        index,
                        begin,
                        block,
                    } => {
                        if index as usize == piece_index && begin as usize == block_offset {
                            piece_data.extend_from_slice(&block);
                            break;
                        }
                    }
                    PeerMessage::Choke => {
                        // anyhow::bail!("Choked");
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }

        /* TODO: Check piece hash and compare it, to verify that the downloaded piece is correct */

        // Send whole downloaded piece to writer
        self.piece_channel
            .send(PieceData {
                piece_idx: piece.index(),
                data: piece_data,
            })
            .await?;
        Ok(())
    }
}

/// Function that manage downloading pieces that are not downloaded, if peer give us information that it has this piece.
pub async fn downloading_pieces_from_pear(
    stream: TcpStream,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    piece_count: usize,
    sender: Sender<PieceData>,
    piece_pool: Arc<Mutex<HashMap<usize, Piece>>>,
    pieces_downloaded: Arc<AtomicUsize>,
) -> Result<()> {
    let pool = piece_pool.clone();
    let mut peer_conncetion =
        PeerConnection::new(stream, info_hash, peer_id, piece_count, sender, piece_pool).await?;

    // Loop while there is at least one undownloaded piece.
    loop {
        let pieces_have = peer_conncetion.bitfield.lock().await.clone();
        for piece_idx in pieces_have.pieces() {
            let mut lock_pool = pool.lock().await;
            if let Some(piece) = lock_pool.remove(&piece_idx) {
                drop(lock_pool);
                match peer_conncetion.download_piece(piece.clone()).await {
                    Ok(()) => {
                        pieces_downloaded.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    Err(_) => {
                        pool.lock().await.insert(piece_idx, piece);
                    }
                }
            }
        }
        // Try to get information, that this peer has new piece
        // Maximal waiting time without getting new piece info is 2min30s
        // Is usefull to try download piece even if new info didn't come
        let mut new_piece_tries = 0;
        loop {
            new_piece_tries += 1;
            match timeout(Duration::from_secs(30), peer_conncetion.receive_message()).await? {
                Ok(PeerMessage::Have { .. }) | Ok(PeerMessage::Bitfield { .. }) => {
                    break;
                }
                _ => {}
            }
            if new_piece_tries == 5 {
                break;
            }
        }

        // End if all pieces are downloaded
        if pieces_downloaded.load(std::sync::atomic::Ordering::SeqCst) == piece_count {
            break;
        }
    }

    Ok(())
}
