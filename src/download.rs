use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use anyhow::Result;
use lava_torrent::torrent::v1::Torrent;
use lava_torrent::tracker::Peer;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio::task::{self, JoinHandle};
use tokio::time::timeout;

use crate::hash::Hash;
use crate::peer_comunication::peer_connection::{downloading_pieces_from_pear, TIMEOUT};
use crate::peer_id::PeerId;
use crate::piece::{pieces_from_torrent, Piece, PieceData};
use crate::writer::PieceFileWriter;

/// Structure that represents downloading torrent file from peers, and its saving to file
pub struct TorrentDownloader {
    info_hash: [u8; 20],
    total_pieces: usize,
    torrent: Torrent,
    piece_pool: Arc<Mutex<HashMap<usize, Piece>>>,
    download_count: Arc<AtomicUsize>,
}

impl TorrentDownloader {
    /// Create a new torrent downloader based on given torrent file
    pub fn new(torrent: Torrent) -> Result<Self> {
        let piece_pool = pieces_from_torrent(&torrent)?
            .into_iter()
            .enumerate()
            .collect();
        Ok(TorrentDownloader {
            info_hash: Hash::new(torrent.info_hash_bytes())?.to_arr(),
            total_pieces: torrent.pieces.len(),
            torrent,
            piece_pool: Arc::new(Mutex::new(piece_pool)),
            download_count: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Download a file from peers, and save it to given folder.
    /// Given PeerId is used to comunicate with other peers.
    /// Download sender is used to accept indexes of already downloaded pieces.
    pub async fn download_torrent(
        &self,
        peers: Vec<Peer>,
        peer_id: &PeerId,
        folder_path: String,
        downloaded_sender: Sender<usize>,
    ) -> Result<()> {
        let (connection_tasks, receiver) = self.make_peers_connections(peers, peer_id).await?;

        let writer_handle = self
            .init_writer(folder_path, receiver, downloaded_sender)
            .await?;

        // Wait for writer to finish
        writer_handle.await??;

        // Wait for all pieces to download
        for task in connection_tasks {
            task.abort();
            let _ = task.await;
        }

        Ok(())
    }

    /// Do TCP connection to given peers, and start bittorent protocol with them.
    async fn make_peers_connections(
        &self,
        peers: Vec<Peer>,
        peer_id: &PeerId,
    ) -> Result<(Vec<JoinHandle<Result<()>>>, Receiver<PieceData>)> {
        // Establish connections to peers concurrently
        let (sender, receiver) = mpsc::channel(1024);
        let connection_tasks: Vec<_> = peers
            .into_iter()
            .map(|peer| {
                let info_hash = self.info_hash;
                let peer_id_arr = peer_id.to_arr();
                let sender_clone = sender.clone();
                let piece_count = self.torrent.pieces.len();
                let piece_pool = self.piece_pool.clone();
                let downloaded_count = self.download_count.clone();

                task::spawn(async move {
                    match timeout(TIMEOUT, TcpStream::connect(&peer.addr)).await {
                        Ok(Ok(stream)) => {
                            downloading_pieces_from_pear(
                                stream,
                                info_hash,
                                peer_id_arr,
                                piece_count,
                                sender_clone,
                                piece_pool,
                                downloaded_count,
                            )
                            .await
                        }
                        _ => anyhow::bail!("Unable to open tcp connection"),
                    }
                })
            })
            .collect();

        Ok((connection_tasks, receiver))
    }

    /// Init writer in new tokio task.
    /// This writer will save already downloaded pieces to final file.
    async fn init_writer(
        &self,
        folder_path: String,
        piece_channel: Receiver<PieceData>,
        downloaded_sender: Sender<usize>,
    ) -> Result<JoinHandle<Result<()>>> {
        let file_path = PathBuf::from(folder_path).join(&self.torrent.name);
        let total_pieces = self.total_pieces;
        let piece_length = self.torrent.piece_length as usize;
        let file_size = self.torrent.length as u64;
        let handle = task::spawn(async move {
            let piece_writer = PieceFileWriter::new(
                file_path,
                total_pieces,
                piece_length,
                file_size,
                piece_channel,
                downloaded_sender,
            )
            .await;
            piece_writer?.write_file().await
        });

        Ok(handle)
    }
}
