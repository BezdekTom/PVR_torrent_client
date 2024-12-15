use std::fmt::Display;

use rand::{thread_rng, Rng};

/// Prefix defined by me for peer-id of peers using my torrent client
const CLIENT_PREFIX: &[u8] = b"-PVR001-";

/// Structure that represents peer-id, which is used as idetificator in torrent protocol comunication.
pub struct PeerId {
    bytes: Vec<u8>,
}

impl PeerId {
    /// Generates random peer-id with prefix `-PVR001-`, that is correct acording to bittorent protocol.
    pub fn generate() -> Self {
        let mut rng = thread_rng();

        // Generate a 20-byte peer ID following the convention:
        let mut peer_id = Vec::with_capacity(20);

        // Add client prefix
        peer_id.extend_from_slice(CLIENT_PREFIX);

        // Generate remaining random bytes
        while peer_id.len() < 20 {
            // Use a mix of alphanumeric characters
            let random_char = rng.gen_range(b'a'..=b'z');
            peer_id.push(random_char);
        }

        // Ensure exactly 20 bytes
        assert_eq!(peer_id.len(), 20, "Peer ID must be exactly 20 bytes");

        PeerId { bytes: peer_id }
    }
}

impl Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8(self.bytes.clone()).unwrap())
    }
}

impl AsRef<[u8]> for PeerId {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl PeerId {
    /// Returns clone of iner bytes, represented as vectore.
    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Returns iner bytes as array of length 20.
    pub fn to_arr(&self) -> [u8; 20] {
        let array: [u8; 20] = match self.bytes.clone().try_into() {
            Ok(arr) => arr,
            Err(_) => {
                panic!("The Vec<u8> does not have exactly 20 elements");
            }
        };
        array
    }
}
