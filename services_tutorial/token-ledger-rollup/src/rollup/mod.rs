//! a rollup state.
//! TODO rename all to not be rollup: it is an abuse of language

pub use jam_types::Hash as RollupHash;

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
mod client;

pub mod witness;

pub type TreeIndex = u16;

// only 15 to be able to index hashes with u16
const MAX_TREE_DEPTH: usize = 15;

// TODO rn tree_hash
pub fn hash_key(k: &[u8]) -> RollupHash {
    unimplemented!();
}

pub fn tree_index(k: &[u8]) -> TreeIndex {
    let hash = hash_key(k);

    let b: [u8; 2] = [hash[0], hash[1]];
    TreeIndex::from_le_bytes(b)
}
