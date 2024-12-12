use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use lava_torrent::tracker::Peer;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio::task;

use crate::peer::PeerConnection;
use crate::peer_id::PeerId;

struct TorrentDownloader {
    info_hash: [u8; 20],
    total_pieces: usize,
    piece_length: usize,
    pieces_downloaded: Arc<RwLock<HashSet<usize>>>,
    pieces_data: Arc<RwLock<HashMap<usize, Vec<u8>>>>,
}

impl TorrentDownloader {
    async fn make_peers_connections(
        &self,
        peers: Vec<Peer>,
        peer_id: &PeerId,
    ) -> Result<Vec<PeerConnection>> {
        // Establish connections to peers concurrently
        let connection_tasks: Vec<_> = peers
            .into_iter()
            .map(|peer| {
                let info_hash = self.info_hash;
                let peer_id_arr = peer_id.to_arr();

                task::spawn(async move {
                    match TcpStream::connect(&peer.addr).await {
                        Ok(stream) => {
                            let mut peer_conn = PeerConnection::new(stream, peer_id_arr);

                            if let Err(_) = peer_conn.handshake(&info_hash).await {
                                return None;
                            }

                            Some(peer_conn)
                        }
                        Err(_) => None,
                    }
                })
            })
            .collect();

        // Collect successful connections
        let mut active_connections = Vec::new();
        for task in connection_tasks {
            if let Ok(Some(connection)) = task.await {
                active_connections.push(connection);
            }
        }
        Ok(active_connections)
    }

    async fn download_torrent(&self, peers: Vec<Peer>, peer_id: &PeerId) -> Result<()> {
        let connections = Arc::new(Mutex::new(
            self.make_peers_connections(peers, peer_id).await?,
        ));

        // Download pieces concurrently
        let download_tasks: Vec<_> = (0..self.total_pieces)
            .map(|piece_index| {
                let connections = Arc::clone(&connections);
                let piece_downloaded = Arc::clone(&self.pieces_downloaded);
                let piece_data = Arc::clone(&self.pieces_data);

                task::spawn(async move {
                    // Find a peer with the piece
                    for conn in connections.lock().await.iter_mut() {
                        if conn.bitfield[piece_index] {
                            match conn.download_piece(piece_index).await {
                                Ok(downloaded_piece_data) => {
                                    piece_downloaded.write().await.insert(piece_index);

                                    piece_data
                                        .write()
                                        .await
                                        .insert(piece_index, downloaded_piece_data);
                                    return Ok(());
                                }
                                Err(_) => continue,
                            }
                        }
                    }
                    anyhow::bail!("No peer has piece {}", piece_index)
                })
            })
            .collect();

        // Wait for all pieces to download
        for task in download_tasks {
            task.await??;
        }

        Ok(())
    }
}
