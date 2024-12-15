use crate::hash::Hash;
use lava_torrent::torrent::v1::Torrent;

/// Structure representing data of one downloaded piece of downloaded file.
/// Contains `piece index` and `piece data`.
#[derive(Debug, Clone)]
pub struct PieceData {
    pub piece_idx: usize,
    pub data: Vec<u8>,
}

/// Structure representing information necessary for downloading one piece from peer.
/// Containd `piece index`, `piece length` and `piece data hash`.
#[derive(Debug, Clone)]
pub struct Piece {
    piece_idx: usize,
    length: usize,
    hash: [u8; 20],
}

impl Piece {
    /// Create new `Piece`, based on `piece index` and informatins from `torrent file`.
    pub(crate) fn new(piece_idx: usize, torrent: &Torrent) -> anyhow::Result<Self> {
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

        Ok(Self {
            // peers,
            piece_idx,
            length: piece_size as usize,
            hash: Hash::new(piece_hash)?.to_arr(),
        })
    }

    /// Returns `piece index`.
    pub(crate) fn index(&self) -> usize {
        self.piece_idx
    }

    /// Returns hash of piece data.
    #[allow(dead_code)]
    pub(crate) fn hash(&self) -> [u8; 20] {
        self.hash
    }

    /// Returns `piece length` in bytes.
    pub(crate) fn length(&self) -> usize {
        self.length
    }
}

/// Returns information about all pieces that should be downloaded, based on `torrent file`.
pub fn pieces_from_torrent(torrent: &Torrent) -> anyhow::Result<Vec<Piece>> {
    let mut pieces = Vec::new();
    for i in 0..torrent.pieces.len() {
        pieces.push(Piece::new(i, torrent)?);
    }

    Ok(pieces)
}
