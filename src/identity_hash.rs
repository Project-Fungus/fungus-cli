//! `Hasher`, `HashMap`, and `HashSet` which use the passed-in value directly as the hash.
//! This is useful for using precomputed hashes as keys to a `HashMap` or `HashSet`.
//!
//! # Example
//!
//! ```rust
//! # fn main() {
//! use fungus_cli::identity_hash::{IdentityHashMap, IdentityHashSet};
//! let mut map: IdentityHashMap<u32> = IdentityHashMap::default();
//! map.insert(22, 44);
//! let mut set: IdentityHashSet = IdentityHashSet::default();
//! set.insert(22);
//! # }

use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, Hasher},
};

/// Hasher which simply returns the passed-in value. To be used exclusively with u64 values, panics otherwise.
///
/// # Panics
///
/// Panics if any method other than `write_u64` is called.
#[derive(Default)]
pub struct IdentityHasher {
    hash: u64,
}

impl Hasher for IdentityHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        panic!("IdentityHasher should only be used with u64 values")
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }
}

pub type IdentityHashMap<V> = HashMap<u64, V, BuildHasherDefault<IdentityHasher>>;
pub type IdentityHashSet = HashSet<u64, BuildHasherDefault<IdentityHasher>>;
