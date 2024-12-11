pub struct Hash {
    hash: Vec<u8>,
}

impl Hash {
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

impl ToString for Hash {
    fn to_string(&self) -> String {
        // Converts bytes to a URL-safe percent-encoded string
        // self.bytes.iter().map(|&b| format!("%{:02X}", b)).collect()
        String::from_utf8(self.hash.clone()).unwrap()
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.hash
    }
}

impl Hash {
    pub fn to_vec(&self) -> Vec<u8> {
        self.hash.clone()
    }

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
