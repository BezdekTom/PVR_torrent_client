use std::path::Path;

type Hash = [u8; 20];

pub struct TorrentFile {
    announce: String,
    info_hash: Hash,
    piece_hashes: Vec<Hash>,
    piece_length: usize,
    length: usize,
    name: String,
}

// impl TorrentFile {
//     pub fn new(file_path: Path) -> anyhow::Result<Self> {
//         lava_torrent
//     }
// }
