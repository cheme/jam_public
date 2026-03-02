//! External client can be seen as a sidechain or parachain
//! to some extent, but really is just building data over
//! data lake and running state transition in refinement.

pub use blake2b_simd as blake2b;
pub use jam_types::Hash;

pub mod state;

mod transition;

pub use transition::{state_transition, Operations};

pub type TreeIndex = u16;

// we apply rule of empty hash being all 0, and hash by empty hash being hash.
pub const EMPTY_HASH: Hash = [0u8; 32];

// only 15 to be able to index hashes with u16
const TREE_DEPTH: usize = 15;

pub fn hash_multiple(m: &[&[u8]]) -> Hash {
    if m.is_empty() || m.iter().map(AsRef::as_ref).all(<[u8]>::is_empty) {
        return EMPTY_HASH;
    }
    let mut hasher = blake2b::State::new();
    for v in m {
        hasher.update(v);
    }
    let mut res = EMPTY_HASH;
    res.copy_from_slice(&hasher.finalize().as_bytes()[0..32]);
    res
}

pub fn hash_pair(h1: &Hash, h2: &Hash) -> Hash {
    if h1 == &[0u8; 32] {
        return *h2;
    };
    if h2 == &[0u8; 32] {
        return *h1;
    };
    hash_multiple(&[&h1[..], &h2[..]])
}

pub fn tree_index_from_key(k: &[u8]) -> TreeIndex {
    let hash = hash_multiple(&[k]);
    // u15
    let b: [u8; 2] = [hash[0], hash[1] & (255 << 1)];
    TreeIndex::from_le_bytes(b)
}

/// Hash to include in merkle structure for a given value.
trait MerkleValue {
    fn merkle_value(&self) -> Hash;
}

fn hash_sequence<V: MerkleValue>(values: &[V]) -> Hash {
    let mut hasher = blake2b::State::new();
    for v in values {
        hasher.update(v.merkle_value().as_slice());
    }
    let mut res = EMPTY_HASH;
    res.copy_from_slice(&hasher.finalize().as_bytes()[0..32]);
    res
}
