/// Struct representing bitfield, where each bit in the field contains logical information, `true(1)` or `false(0)`.
#[derive(Debug, Clone)]
pub struct Bitfield {
    bytes: Vec<u8>,
}

impl Bitfield {
    /// Creates new bitfiled from given vector of bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Bitfield { bytes }
    }

    /// Create bitfield containing all `false(0)`, with capacity for all pieces given by `piece_count`.
    pub fn empty_with_piece_capacity(piece_count: usize) -> Self {
        Bitfield {
            bytes: vec![0u8; (piece_count + 7) / (u8::BITS as usize)],
        }
    }

    /// Returs bitfield as bytes.
    pub fn as_bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    /// Return information if the piece with index `piece_index` is set in bitfield as `1`, which mean that bit on index `piece_idx` is set to `1`.
    /// If the index is out of `bitfield`, returs `false`.
    pub fn has_piece(&self, piece_idx: usize) -> bool {
        let byte_idx = piece_idx / (u8::BITS as usize);
        let bit_idx = (piece_idx % (u8::BITS as usize)) as u32;
        let Some(&byte) = self.bytes.get(byte_idx) else {
            return false;
        };
        byte & 1u8.rotate_right(bit_idx + 1) != 0
    }

    /// Set bit on `piece_idx` is set to `1`.
    pub fn set_piece(&mut self, piece_idx: usize) {
        let byte_idx = piece_idx / (u8::BITS as usize);
        let bit_idx = (piece_idx % (u8::BITS as usize)) as u32;
        let byte = self.bytes.get_mut(byte_idx);
        if let Some(byte) = byte {
            *byte |= 1u8.rotate_right(bit_idx + 1);
        };
    }

    /// Enable iteration over all pieces that are set to `1` in bitfield.
    pub fn pieces(&self) -> impl Iterator<Item = usize> + '_ {
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
fn bitfield_has_set() {
    let mut bf = Bitfield::new(vec![0b10101010, 0b01010101]);
    assert!(bf.has_piece(0));
    assert!(!bf.has_piece(1));
    assert!(!bf.has_piece(7));
    assert!(!bf.has_piece(8));
    assert!(bf.has_piece(15));

    bf.set_piece(1);
    assert!(bf.has_piece(0));
    assert!(bf.has_piece(1));
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
