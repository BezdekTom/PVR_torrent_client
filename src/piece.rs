use lava_torrent::torrent::v1::Torrent;
use std::collections::HashSet;

use crate::hash::Hash;
use crate::peer::PeerConnection;

#[derive(Debug, PartialEq, Eq)]
pub struct Piece {
    peers: HashSet<usize>,
    piece_idx: usize,
    length: usize,
    hash: [u8; 20],
}

impl Ord for Piece {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.peers
            .len()
            .cmp(&other.peers.len())
            // tie-break by _random_ ordering of HashSet to avoid deterministic contention
            .then(self.peers.iter().cmp(other.peers.iter()))
            .then(self.hash.cmp(&other.hash))
            .then(self.length.cmp(&other.length))
            .then(self.piece_idx.cmp(&other.piece_idx))
    }
}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Piece {
    pub(crate) fn new(
        piece_idx: usize,
        torrent: &Torrent,
        peers: &[PeerConnection],
    ) -> anyhow::Result<Self> {
        let piece_hash = torrent.pieces[piece_idx].clone();
        let piece_size = if piece_idx == torrent.pieces.len() - 1 {
            let md = torrent.length % torrent.piece_length;
            if md == 0 {
                torrent.piece_length
            } else {
                md
            }
        } else {
            torrent.piece_length
        };

        let peers = peers
            .iter()
            .enumerate()
            .filter_map(|(peer_i, peer)| peer.has_piece(piece_idx).then_some(peer_i))
            .collect();

        Ok(Self {
            peers,
            piece_idx,
            length: piece_size as usize,
            hash: Hash::new(piece_hash)?.to_arr(),
        })
    }

    pub(crate) fn peers(&self) -> &HashSet<usize> {
        &self.peers
    }

    pub(crate) fn index(&self) -> usize {
        self.piece_idx
    }

    pub(crate) fn hash(&self) -> [u8; 20] {
        self.hash
    }

    pub(crate) fn length(&self) -> usize {
        self.length
    }
}
