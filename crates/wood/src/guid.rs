//! A version 4 style GUID for a new part. Not cryptographic: it only has to be unique among the
//! parts of one project, and it is made from the clock and the standard library's random keys.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn word() -> u64 {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let mut h = RandomState::new().build_hasher();
    h.write_u64(n);
    h.write_u64(t);
    h.finish()
}

/// `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx` in lower case.
pub fn new_guid() -> String {
    let (a, b) = (word(), word());
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&a.to_be_bytes());
    bytes[8..].copy_from_slice(&b.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let x: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &x[0..8], &x[8..12], &x[12..16], &x[16..20], &x[20..32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_and_uniqueness() {
        let a = new_guid();
        let b = new_guid();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.as_bytes()[14], b'4');
        assert!(a.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
    }
}
