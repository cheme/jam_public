//! rollup state on client size.

use super::RollupHash as Hash;
use super::{hash_key, hash_multiple, hash_pair, tree_index_from_key, TreeIndex, TREE_DEPTH};
use alloc::collections::btree_map::BTreeMap;
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
use alloc::vec::Vec;
use codec::{Decode, Encode};
use token_ledger_common::{AccountId, TokenId};

// TODO init from file then write at end by using seek first
#[cfg(feature = "std")]
use std::io::{Read, Seek, Write};

// very small state size, expect hash collisions (jut fail on hash collision: we store key so we
// can see if hash collision)_
const TREE_SIZE: usize = 1 << TREE_DEPTH;
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
    pub fn root(&self) -> &Hash {
        return &self.hashes[TREE_HASHES - 1];
    }

    pub fn insert(&mut self, ix: TreeIndex, value_hash: Hash) {
        let mut hash = value_hash;
        let mut at = ix as usize;
        let mut offset = 0;
        for depth in 0..TREE_DEPTH {
            self.hashes[offset + at] = value_hash;
            if at % 2 == 0 {
                hash = hash_pair(&hash, &self.hashes[offset + at + 1]);
            } else {
                hash = hash_pair(&self.hashes[offset + at - 1], &hash);
            }
            offset += 1 << (TREE_DEPTH - depth);
            at = at / 2;
        }
        self.hashes[offset + at] = value_hash;
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        let mut hashes = Vec::with_capacity(TREE_HASHES);
        hashes.resize(TREE_HASHES, Default::default());
        Self { hashes }
    }
}

// TODO consider removal, just std is client, not std is service
#[repr(u8)]
pub enum Mode {
    // we store log and persist in file
    // we also produce state root
    Client,
    // we run on state (likely initialized from witness)
    // we do not store log
    Service,
}

#[derive(Default)]
pub struct State<const M: u8> {
    root: Hash,
    balances: StateTree<Balance, M>,
    // using a second tree cause why not? Any way
    // to record a witness is fine (zk, custom merkle,
    // generic trie over encoded data...).
    tokens: StateTree<Token, M>,
}

impl<const M: u8> State<M> {
    fn insert_balance(&mut self, account: AccountId, token_id: TokenId, balance: u64) {
        let to_key = token_ledger_common::balance_key(token_id, &account);
        if !self.balances.insert(to_key.to_vec(), balance) {
            unimplemented!("error on key collision");
        }

        self.update_hash();
    }

    fn get_balance(&mut self, account: AccountId, token_id: TokenId) -> Option<u64> {
        let to_key = token_ledger_common::balance_key(token_id, &account);
        self.balances.get(to_key.as_slice()).cloned()
    }

    fn update_hash(&mut self) {
        self.root = hash_pair(self.balances.root(), self.tokens.root());
    }
}

pub trait ValueTraits: Clone + Decode + Encode + HashValue {}

impl<V: Clone + Decode + Encode + HashValue> ValueTraits for V {}

// just write all added key value and replay when opening
pub struct SerializedState {}

pub struct StateTree<V: ValueTraits, const M: u8> {
    // TODO rem (TreeIndex is always hashextract of key...), yet avoid checking for existing key
    indexes: BTreeMap<Vec<u8>, TreeIndex>,
    values: BTreeMap<TreeIndex, Value<V>>,
    tree: MerkleTree,
    #[cfg(feature = "std")]
    log_file: Option<std::fs::File>,
    #[cfg(feature = "std")]
    log: std::collections::VecDeque<(Vec<u8>, Vec<u8>)>,
}

impl<V: ValueTraits> Drop for StateTree<V> {
    fn drop(&mut self) {
        #[cfg(feature = "std")]
        self.flush_log();
    }
}

impl<V: ValueTraits, const M: u8> Default for StateTree<V, M> {
    fn default() -> Self {
        Self {
            indexes: Default::default(),
            values: Default::default(),
            tree: Default::default(),
            #[cfg(feature = "std")]
            log: std::collections::VecDeque::new(),
            #[cfg(feature = "std")]
            log_file: None,
        }
    }
}

impl<V: ValueTraits, const M: u8> StateTree<V, M> {
    #[cfg(feature = "std")]
    fn from_file(mut file: std::fs::File) -> Self {
        let mut result = Self::default();
        let mut buf_reader = codec::IoReader(std::io::BufReader::new(&mut file));
        while let Ok(item) = <(Vec<u8>, Vec<u8>)>::decode(&mut buf_reader) {
            let v = V::decode(&mut item.1.as_slice()).unwrap();
            result.insert(item.0, v);
        }
        result.log_file = Some(file);
        result
    }
    #[cfg(feature = "std")]
    fn flush_log(&mut self) {
        if M != Mode::Client as u8 {
            return;
        }
        let Some(file) = self.log_file.as_mut() else {return };
        file.seek(std::io::SeekFrom::End(0)).unwrap();
        while let Some(item) = self.log.pop_front() {
            file.write_all(&item.encode()).unwrap();
        }
        file.flush().unwrap();
    }

    // TODO rem?
    fn get_value(&self, k: &[u8]) -> Option<&Value<V>> {
        let i = self.indexes.get(k)?;
        self.values.get(i)
    }

    pub fn get(&self, k: &[u8]) -> Option<&V> {
        let v = self.get_value(k)?;
        if v.key.as_slice() != k {
            return None;
        };
        Some(&v.value)
    }

    // fail on key collision by returning false
    pub fn insert(&mut self, k: Vec<u8>, v: V) -> bool {
        let ix = tree_index_from_key(&k);
        if let Some(existing) = self.values.get_mut(&ix) {
            if existing.key.as_slice() != k {
                return false;
            };
            existing.value = v;
            self.tree.insert(ix, existing.hash_value());
        } else {
            let value = Value { key: k, value: v };
            self.tree.insert(ix, value.hash_value());
            self.indexes.insert(value.key.clone(), ix);
            self.values.insert(ix, value);
        }

        true
    }

    pub fn root(&self) -> &Hash {
        self.tree.root()
    }
}

#[derive(Clone, Encode, Decode)]
pub struct Token {}

impl HashValue for Token {
    fn hash_value(&self) -> Hash {
        unimplemented!();
    }
}

pub type Balance = u64;

impl HashValue for Balance {
    fn hash_value(&self) -> Hash {
        let mut result = [0; 32];
        // just encode balance, hash definition space is larger than content.
        result[0..8].copy_from_slice(u64::to_le_bytes(*self).as_slice());
        result
    }
}

#[derive(Clone, Encode, Decode)]
pub struct Value<V> {
    value: V,
    key: Vec<u8>,
}

impl<V: HashValue> HashValue for Value<V> {
    fn hash_value(&self) -> Hash {
        hash_multiple(&[self.value.hash_value().as_slice(), self.key.as_slice()])
    }
}

/// Hash to include in merkle structure for a given value.
trait HashValue {
    fn hash_value(&self) -> Hash;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state() {
        let empty: State<{ Mode::Client as u8 }> = Default::default();

        assert_eq!([0; 32], empty.root);
    }
    #[test]
    fn check_constants() {
        assert_eq!(TREE_DEPTH + 1, std::mem::size_of::<TreeIndex>() * 8);
    }
    #[test]
    fn create_token_and_distribute() {
        let mut state: State<{ Mode::Client as u8 }> = Default::default();

        // TODO rem, use primitives from ops
        state.insert_balance([1; 32], 1, 10);

        assert!([0; 32] != state.root);

        assert_eq!(Some(10), state.get_balance([1; 32], 1));
    }
}
