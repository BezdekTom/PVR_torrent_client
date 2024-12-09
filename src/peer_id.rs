use rand::{thread_rng, Rng};

const CLIENT_PREFIX: &[u8] = b"-PVR001-";

pub struct PeerId {
    bytes: Vec<u8>,
}

impl PeerId {
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

impl ToString for PeerId {
    fn to_string(&self) -> String {
        // Converts bytes to a URL-safe percent-encoded string
        // self.bytes.iter().map(|&b| format!("%{:02X}", b)).collect()
        String::from_utf8(self.bytes.clone()).unwrap()
    }
}

impl AsRef<[u8]> for PeerId {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl PeerId {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }

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
