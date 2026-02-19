//! rollup state on client size.

use super::RollupHash as Hash;
use super::{hash_key, tree_index, TreeIndex};
use alloc::collections::btree_map::BTreeMap;
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
use alloc::vec::Vec;
use codec::{Decode, Encode};

// only 15 to be able to index hashes with u16
const MAX_TREE_DEPTH: usize = 15;

// very small state size, expect hash collisions (jut fail on hash collision: we store key so we
// can see if hash collision)_
const TREE_SIZE: usize = 1 << MAX_TREE_DEPTH;
const TREE_HASHES: usize = (TREE_SIZE * 2) - 1;

// We use some small bounded tree for the sake of having the simplest implementation.
// We also hold tree in memory and just clone when we want to keep history.
// This is only client side, never under jam (only partial state under jam refine or accumulate).
// Simple binary tree with merkle hash 0 indicating empty, we do not attempt to reduce size proof
// so we do not really have to care with the size of the state: any key access involve a 15 hash
// proof, purpose is not to be efficient here.
#[derive(Clone)]
pub struct MerkleTree {
    hashes: Vec<Hash>,
}

impl MerkleTree {
		pub fn root(&self) -> Hash {
			return self.hashes[0];
		}

		pub fn insert(&mut self, ix: TreeIndex, value_hash: Hash) {
			unimplemented!();
		}
}

impl Default for MerkleTree {
    fn default() -> Self {
        let mut hashes = Vec::with_capacity(TREE_HASHES);
        hashes.resize(TREE_HASHES, Default::default());
        Self { hashes }
    }
}

#[derive(Default)]
pub struct State {
    root: Hash,
    accounts: StateTree<Account>,
    tokens: StateTree<Token>,
}

impl State {}

#[derive(Clone)]
pub struct StateTree<V: Clone> {
    indexes: BTreeMap<Vec<u8>, TreeIndex>,
    values: BTreeMap<TreeIndex, Value<V>>,
    tree: MerkleTree,
}

impl<V: Clone> Default for StateTree<V> {
    fn default() -> Self {
        Self {
            indexes: Default::default(),
            values: Default::default(),
            tree: Default::default(),
        }
    }
}

impl<V: Clone + Encode> StateTree<V> {
    // TODO rem?
    fn get_value(&self, k: &[u8]) -> Option<&Value<V>> {
        let i = self.indexes.get(k)?;
        self.values.get(i)
    }

    pub fn get(&self, k: &[u8]) -> Option<&V> {
        let v = self.get_value(k)?;
        if v.item.key.as_slice() != k {
            return None;
        };
        Some(&v.value)
    }

    // fail on key collision by returning false
    pub fn insert(&mut self, k: Vec<u8>, v: V) -> bool {
        let ix = tree_index(&k);
        if let Some(existing) = self.values.get_mut(&ix) {
            if existing.item.key.as_slice() != k {
                return false;
            };
            existing.item.encoded = v.encode();
            existing.value = v;
						self.tree.insert(ix, hash_key(&existing.item.encode()));
        } else {
					let item = TreeItem {
                        key: k.clone(),
                        encoded: v.encode(),
                    };
						self.tree.insert(ix, hash_key(&item.encode()));
            self.values.insert(
                ix,
                Value {
                    item,
                    value: v,
                },
            );
            self.indexes.insert(k, ix);
        }

        true
    }
}

#[derive(Clone)]
pub struct Account {}

#[derive(Clone)]
pub struct Token {}

#[derive(Clone, Encode, Decode)]
pub struct Value<V> {
    value: V,
    item: TreeItem,
}

#[derive(Clone, Encode, Decode)]
pub struct TreeItem {
    encoded: Vec<u8>,
    key: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state() {
        let empty: State = Default::default();

        //			assert_eq!([0; 32], empty.root);
    }
    #[test]
    fn check_constants() {
        assert_eq!(MAX_TREE_DEPTH + 1, std::mem::size_of::<TreeIndex>() * 8);
    }
}
