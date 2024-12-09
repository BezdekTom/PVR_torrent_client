pub struct InfoHash {
    bytes: Vec<u8>,
}

impl InfoHash {
    pub fn new(info_hash_bytes: Vec<u8>) -> anyhow::Result<Self> {
        if info_hash_bytes.len() != 20 {
            return Err(anyhow::Error::msg(format!(
                "Invalid length of Info Hash bytes, expected 20 but get {}",
                info_hash_bytes.len()
            )));
        }
        Ok(InfoHash {
            bytes: info_hash_bytes,
        })
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
