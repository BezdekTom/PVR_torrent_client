pub const BITTORRENT_PROTOCOL: [u8; 19] = *b"BitTorrent protocol";

/// Structure representing bittorent handshake/
pub struct Handshake {
    pub length: u8,
    pub bittorrent: [u8; 19],
    pub reserve: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    /// Creates bittorent hadshake based on `info_hash` of file, and `peer_id` of client doing the handshake.
    pub fn new(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Self {
        Handshake {
            length: 19,
            bittorrent: BITTORRENT_PROTOCOL,
            reserve: [0; 8],
            info_hash: *info_hash,
            peer_id: *peer_id,
        }
    }

    /// Get bytes from handshake as array of `68` bytes.
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

    /// Set informations in handshake, based on array of `68` bytes.
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
