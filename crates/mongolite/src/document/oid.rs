use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static MACHINE_ID: [u8; 5] = generate_machine_id();
static COUNTER: AtomicU32 = AtomicU32::new(0);

const fn generate_machine_id() -> [u8; 5] {
    let seed = {
        let mut hash: u32 = 0;
        let bytes = b"mongolite";
        let mut i = 0;
        while i < bytes.len() {
            hash = hash.wrapping_mul(31).wrapping_add(bytes[i] as u32);
            i += 1;
        }
        hash
    };

    [
        ((seed >> 24) & 0xFF) as u8,
        ((seed >> 16) & 0xFF) as u8,
        ((seed >> 8) & 0xFF) as u8,
        (seed & 0xFF) as u8,
        ((seed.wrapping_mul(7)) & 0xFF) as u8,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId([u8; 12]);

impl ObjectId {
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_secs() as u32;

        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let counter_bytes = counter.to_be_bytes();

        let mut bytes = [0u8; 12];

        bytes[0..4].copy_from_slice(&timestamp.to_be_bytes());
        bytes[4..9].copy_from_slice(&MACHINE_ID);
        bytes[9..12].copy_from_slice(&counter_bytes[1..4]);

        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 12] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn from_hex(hex: &str) -> Result<Self, crate::error::Error> {
        if hex.len() != 24 {
            return Err(crate::error::Error::Bson(format!(
                "invalid ObjectId hex length: expected 24, got {}",
                hex.len()
            )));
        }

        let mut bytes = [0u8; 12];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let high = hex_char_to_u8(chunk[0])?;
            let low = hex_char_to_u8(chunk[1])?;
            bytes[i] = (high << 4) | low;
        }

        Ok(Self(bytes))
    }

    pub fn timestamp(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

fn hex_char_to_u8(c: u8) -> Result<u8, crate::error::Error> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(crate::error::Error::Bson(format!(
            "invalid hex character: {}",
            c as char
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();
        let id3 = ObjectId::new();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_from_bytes_to_bytes_roundtrip() {
        let original = ObjectId::new();
        let bytes = *original.as_bytes();
        let restored = ObjectId::from_bytes(bytes);

        assert_eq!(original, restored);
    }

    #[test]
    fn test_hex_encoding() {
        let id = ObjectId::new();
        let hex = id.to_hex();

        assert_eq!(hex.len(), 24);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hex_roundtrip() {
        let id = ObjectId::new();
        let hex = id.to_hex();
        let restored = ObjectId::from_hex(&hex).unwrap();

        assert_eq!(id, restored);
    }

    #[test]
    fn test_from_hex_invalid_length() {
        let result = ObjectId::from_hex("1234");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_hex_invalid_characters() {
        let result = ObjectId::from_hex("zzzzzzzzzzzzzzzzzzzzzzzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_timestamp_extraction() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        let id = ObjectId::new();

        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        let ts = id.timestamp();
        assert!(ts >= before && ts <= after);
    }

    #[test]
    fn test_display_trait() {
        let id = ObjectId::new();
        let display = format!("{}", id);
        let hex = id.to_hex();

        assert_eq!(display, hex);
    }

    #[test]
    fn test_default_trait() {
        let id: ObjectId = Default::default();
        let new_id = ObjectId::new();

        assert_eq!(id.timestamp(), new_id.timestamp());
    }

    #[test]
    fn test_many_unique_ids() {
        let mut ids = std::collections::HashSet::new();

        for _ in 0..10000 {
            let id = ObjectId::new();
            assert!(ids.insert(id), "duplicate ObjectId generated");
        }

        assert_eq!(ids.len(), 10000);
    }
}
