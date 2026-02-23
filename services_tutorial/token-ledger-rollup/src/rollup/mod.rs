//! a rollup state.
//! TODO rename all to not be rollup: it is an abuse of language

pub use blake2b_simd as blake2b;
pub use jam_types::Hash as RollupHash;

mod state;

pub mod witness;

pub type TreeIndex = u16;

// we apply rule of empty hash being all 0, and hash by empty hash being hash.
pub const EMPTY_HASH: RollupHash = [0u8; 32];

// only 15 to be able to index hashes with u16
const TREE_DEPTH: usize = 15;

// TODO rn tree_hash
pub fn hash_key(k: &[u8]) -> RollupHash {
    hash_multiple(&[k])
}

pub fn hash_multiple(m: &[&[u8]]) -> RollupHash {
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

pub fn hash_pair(h1: &RollupHash, h2: &RollupHash) -> RollupHash {
    if h1 == &[0u8; 32] {
        return *h2;
    };
    if h2 == &[0u8; 32] {
        return *h1;
    };
    hash_multiple(&[&h1[..], &h2[..]])
}

pub fn tree_index_from_key(k: &[u8]) -> TreeIndex {
    let hash = hash_key(k);
    // u15
    let b: [u8; 2] = [hash[0], hash[1] & (255 << 1)];
    TreeIndex::from_le_bytes(b)
}
