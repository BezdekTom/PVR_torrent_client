use anyhow::Context;
use lava_torrent::tracker::Peer;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{hash::Hash, peer_id::PeerId};

pub struct PeerConnection {
    peer: Peer,
    peer_tcp: TcpStream,
    bitfield: Bitfield,
    choked: bool,
}

impl PeerConnection {
    pub async fn new(peer: Peer, info_hash: &Hash, peer_id: &PeerId) -> anyhow::Result<Self> {
        let mut peer_tcp = TcpStream::connect(peer.addr)
            .await
            .context("Connecting to peer")?;

        let mut handshake = Handshake::new(info_hash, peer_id);
        let mut handshake_bytes = handshake.get_bytes();
        peer_tcp
            .write_all(&handshake_bytes)
            .await
            .context("Write handshake")?;
        peer_tcp
            .read_exact(&mut handshake_bytes)
            .await
            .context("Read handshake")?;
        handshake.set_bytes(&handshake_bytes);

        anyhow::ensure!(handshake.length == 19);
        anyhow::ensure!(&handshake.bittorrent == b"BitTorrent protocol");

        //TODO: maybe not all is done

        Ok(PeerConnection {
            peer,
            peer_tcp,
            bitfield: todo!(),
            choked: true,
        })
    }

    pub fn has_piece(&self, piece_idx: usize) -> bool {
        self.bitfield.has_piece(piece_idx)
    }
}

struct Bitfield {
    bytes: Vec<u8>,
}

impl Bitfield {
    fn new(bytes: Vec<u8>) -> Self {
        Bitfield { bytes }
    }

    fn has_piece(&self, piece_idx: usize) -> bool {
        let byte_idx = piece_idx / (u8::BITS as usize);
        let bit_idx = (piece_idx % (u8::BITS as usize)) as u32;
        let Some(&byte) = self.bytes.get(byte_idx) else {
            return false;
        };
        byte & 1u8.rotate_right(bit_idx + 1) != 0
    }

    fn pieces(&self) -> impl Iterator<Item = usize> + '_ {
        self.bytes.iter().enumerate().flat_map(|(byte_idx, byte)| {
            (0..u8::BITS).filter_map(move |bit_idx| {
                let piece_idx = byte_idx * (u8::BITS as usize) + (bit_idx as usize);
                let mask = 1u8.rotate_right(bit_idx + 1);
                (byte & mask != 0).then_some(piece_idx)
            })
        })
    }
}

#[test]
fn bitfield_has() {
    let bf = Bitfield::new(vec![0b10101010, 0b01010101]);
    assert!(bf.has_piece(0));
    assert!(!bf.has_piece(1));
    assert!(!bf.has_piece(7));
    assert!(!bf.has_piece(8));
    assert!(bf.has_piece(15));
}

#[test]
fn bitfield_iter() {
    let bf = Bitfield::new(vec![0b10101010, 0b01010101]);
    let mut pieces = bf.pieces();
    assert_eq!(pieces.next(), Some(0));
    assert_eq!(pieces.next(), Some(2));
    assert_eq!(pieces.next(), Some(4));
    assert_eq!(pieces.next(), Some(6));
    assert_eq!(pieces.next(), Some(9));
    assert_eq!(pieces.next(), Some(11));
    assert_eq!(pieces.next(), Some(13));
    assert_eq!(pieces.next(), Some(15));
    assert_eq!(pieces.next(), None);
}

struct Handshake {
    pub length: u8,
    pub bittorrent: [u8; 19],
    pub reserve: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: &Hash, peer_id: &PeerId) -> Self {
        Handshake {
            length: 19,
            bittorrent: *b"BitTorrent protocol",
            reserve: [0; 8],
            info_hash: info_hash.to_arr(),
            peer_id: peer_id.to_arr(),
        }
    }

    pub fn get_bytes(&self) -> [u8; 68] {
        let mut arr = [0u8; 68];
        arr[0] = self.length;

        // Copy the bittorrent field
        arr[1..20].copy_from_slice(&self.bittorrent);

        // Copy the reserve field
        arr[20..28].copy_from_slice(&self.reserve);

        // Copy the info_hash field
        arr[28..48].copy_from_slice(&self.info_hash);

        // Copy the peer_id field
        arr[48..68].copy_from_slice(&self.peer_id);

        arr
    }

    pub fn set_bytes(&mut self, bytes: &[u8; 68]) {
        self.length = bytes[0];

        // Copy the bittorrent field
        self.bittorrent.copy_from_slice(&bytes[1..20]);

        // Copy the reserve field
        self.reserve.copy_from_slice(&bytes[20..28]);

        // Copy the info_hash field
        self.info_hash.copy_from_slice(&bytes[28..48]);

        // Copy the peer_id field
        self.peer_id.copy_from_slice(&bytes[48..68]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageTag {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub tag: MessageTag,
    pub payload: Vec<u8>,
}
