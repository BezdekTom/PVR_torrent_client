use std::fmt::Display;

/// Structure respresenting 20 bytes long hash, that is ofthen used in Bittorent protocol
pub struct Hash {
    hash: Vec<u8>,
}

impl Hash {
    /// Create Hash structure out of 20 bytes long vector
    pub fn new(hash: Vec<u8>) -> anyhow::Result<Self> {
        if hash.len() != 20 {
            return Err(anyhow::Error::msg(format!(
                "Expected len of hash 20, but get {}",
                hash.len()
            )));
        }

        Ok(Hash { hash })
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8(self.hash.clone()).unwrap())
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.hash
    }
}

impl Hash {
    /// Get a clone of bytes stored inside, represented as vector
    #[allow(dead_code)]
    pub fn to_vec(&self) -> Vec<u8> {
        self.hash.clone()
    }

    /// Get a bytes stored inside, as array of length 20
    pub fn to_arr(&self) -> [u8; 20] {
        let array: [u8; 20] = match self.hash.clone().try_into() {
            Ok(arr) => arr,
            Err(_) => {
                panic!("The Vec<u8> does not have exactly 20 elements");
            }
        };
        array
    }
}
