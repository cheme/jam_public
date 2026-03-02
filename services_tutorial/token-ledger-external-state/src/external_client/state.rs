//! External client state, build over a simple fix size binary tree and linked data.
//! For the sake of keeping this example concise, client implementation only manage
//! two use cases (in realy world for efficiency there is many processing that can be
//! skip for others use cases):
//! - client (with unsafe persistence (breaks on any crash), and always registering witnesses).
//! - pvm, no std, riscv targetting and running on partial state build from witness.
//!
//! Client implementation is obtain by including "std" feature, while pvm is obtain by
//! building no std.

use super::{
    EMPTY_HASH, Hash, MerkleValue, TREE_DEPTH, TreeIndex, hash_multiple, hash_pair, hash_sequence,
    tree_index_from_key,
};

use alloc::collections::BTreeMap;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use codec::{Decode, Encode};
use token_ledger::api::{AccountId, TokenId};

#[cfg(feature = "std")]
use core::cell::RefCell;

#[cfg(feature = "std")]
use std::io::{Seek, Write};

// very small state size, expect hash collisions (jut fail on hash collision: we store key so we
// can see if hash collision)_
const TREE_SIZE: usize = 1 << TREE_DEPTH;
const TREE_HASHES: usize = (TREE_SIZE * 2) - 1;

// We use some small bounded simple binary tree (2^15 itemrs) for the sake of having the simpliest implementation.
// We also avoid optimization to have same kind of footprint between empty state and full state,
// and a cost model relatively easy to reason with.
// Merkle hash 0 is used for empty state so we can use hash default implementation for it.
// A key access involve a witness of 15 hash and the actual key and value.
// Missing value in witness will false positive return an empty hash leading to root mismatch, not
// the best for debugging purpose.
#[derive(Default)]
pub struct MerkleTree {
    hashes: BTreeMap<TreeIndex, Hash>,
    #[cfg(feature = "std")]
    hashes_for_witness: BTreeMap<TreeIndex, Hash>,
    // we use a refcell as client implementation is only for testing
    // and run on a single thread, this way we keep sane prototyping
    // for non client code.
    #[cfg(feature = "std")]
    witness: RefCell<BTreeMap<TreeIndex, Hash>>,
}

impl MerkleTree {
    // Note this must always be call to access any `hashes field content`,
    // so we register witness properly.
    fn get_hash(&self, ix: TreeIndex) -> &Hash {
        if let Some(hash) = self.hashes.get(&ix) {
            #[cfg(feature = "std")]
            {
                let hash = self.hashes_for_witness.get(&ix).unwrap_or(&EMPTY_HASH);
                if hash != &EMPTY_HASH {
                    self.witness.borrow_mut().insert(ix, *hash);
                    self.witness.borrow_mut().insert(ix, *hash);
                }
            }
            return hash;
        } else {
            return &EMPTY_HASH;
        }
    }

    pub fn root(&self) -> &Hash {
        return self.get_hash((TREE_HASHES - 1) as TreeIndex);
    }

    // expect value
    pub fn witness_access(&self, ix: TreeIndex) {
        let mut at = ix;
        let mut offset: TreeIndex = 0;
        for depth in 0..TREE_DEPTH {
            if at % 2 == 0 {
                self.get_hash(offset + at + 1);
            } else {
                self.get_hash(offset + at - 1);
            }
            offset += 1 << (TREE_DEPTH - depth);
            at = at / 2;
        }
    }

    pub fn insert(&mut self, ix: TreeIndex, value_hash: Hash) {
        let mut hash = value_hash;
        let mut at = ix;
        let mut offset: TreeIndex = 0;
        for depth in 0..TREE_DEPTH {
            self.hashes.insert(offset + at, hash);
            if at % 2 == 0 {
                hash = hash_pair(&hash, self.get_hash(offset + at + 1));
            } else {
                hash = hash_pair(self.get_hash(offset + at - 1), &hash);
            }
            offset += 1 << (TREE_DEPTH - depth);
            at = at / 2;
        }

        self.hashes.insert(offset + at, hash);
    }
}

#[derive(Default)]
pub struct State {
    balances: StateTree<Balance>,
    known_tokens: KnownTokens,
}

#[derive(Default, Encode, Decode, Debug)]
pub struct Witness {
    // root is part of the hashes
    hashes: Vec<(TreeIndex, Hash)>,
    key_value_balances: Vec<(Vec<u8>, Balance)>,
    // Currently no operation make sense without accessing it so always store.
    token_ids: Vec<TokenId>,
}

impl State {
    #[cfg(feature = "std")]
    pub fn from_files(balances: std::fs::File, tokens: std::fs::File) -> Self {
        let mut result = State {
            balances: StateTree::<Balance>::from_file(balances),
            known_tokens: KnownTokens::from_file(tokens),
        };
        let _ = result.take_witness();
        result
    }

    #[cfg(feature = "std")]
    pub fn set_new_persist_files(&mut self, balances: std::fs::File, tokens: std::fs::File) {
        self.balances.persist = Some(balances);
        self.known_tokens.persist = Some(tokens);
    }

    #[cfg(feature = "std")]
    pub fn take_witness(&mut self) -> Witness {
        // take
        let hashes = std::mem::replace(
            self.balances.tree.witness.get_mut(),
            BTreeMap::<TreeIndex, Hash>::default(),
        );
        let values = std::mem::replace(
            self.balances.witness_values.get_mut(),
            BTreeMap::<Vec<u8>, Balance>::default(),
        );
        let token_ids = std::mem::take(&mut self.known_tokens.witness);

        // update for next run
        self.balances.tree.hashes_for_witness = self.balances.tree.hashes.clone();
        self.known_tokens.witness = self.known_tokens.token_ids.clone();

        return Witness {
            hashes: hashes.into_iter().collect(),
            key_value_balances: values.into_iter().collect(),
            token_ids,
        };
    }

    pub fn from_witness(witness: Witness) -> Option<Self> {
        let mut result = Self::default();
        if let Some(balances) =
            StateTree::init_from_witness(&witness.hashes, witness.key_value_balances)
        {
            result.balances = balances;
        } else {
            return None;
        }

        result.known_tokens.token_ids = witness.token_ids;

        #[cfg(feature = "std")]
        let _ = result.take_witness();

        return Some(result);
    }

    pub fn get_root(&self) -> Hash {
        return hash_pair(self.balances.root(), &self.known_tokens.merkle_value());
    }

    pub fn get_balance(&self, account: AccountId, token_id: TokenId) -> Option<u64> {
        let to_key = token_ledger::api::balance_key(token_id, &account);
        self.balances.get(to_key.as_slice()).cloned()
    }

    pub fn set_balance(&mut self, account: AccountId, token_id: TokenId, balance: u64) {
        let to_key = token_ledger::api::balance_key(token_id, &account);
        if !self.balances.set(to_key.to_vec(), balance) {
            unimplemented!("error on key collision");
        }
    }

    pub fn known_tokens_contains(&self, token_id: TokenId) -> bool {
        // not registering in witness as known token is always part of it.
        return self.known_tokens.token_ids.contains(&token_id);
    }

    pub fn known_tokens_push(&mut self, token_id: TokenId) {
        self.known_tokens.push_token(token_id);
    }
}

trait ValueTraits: Clone + Decode + Encode + MerkleValue {}

impl<V: Clone + Decode + Encode + MerkleValue> ValueTraits for V {}

struct StateTree<V: ValueTraits> {
    // TODO rem (TreeIndex is always hashextract of key...), yet avoid checking for existing key
    indexes: BTreeMap<Vec<u8>, TreeIndex>,
    values: BTreeMap<TreeIndex, Value<V>>,
    tree: MerkleTree,
    // No consistency, just rewrite all on drop.
    #[cfg(feature = "std")]
    persist: Option<std::fs::File>,
    #[cfg(feature = "std")]
    witness_values: RefCell<BTreeMap<Vec<u8>, V>>,
}

impl<V: ValueTraits> Drop for StateTree<V> {
    fn drop(&mut self) {
        #[cfg(feature = "std")]
        self.serialize();
    }
}

impl<V: ValueTraits> Default for StateTree<V> {
    fn default() -> Self {
        Self {
            indexes: Default::default(),
            values: Default::default(),
            tree: Default::default(),
            #[cfg(feature = "std")]
            persist: None,
            #[cfg(feature = "std")]
            witness_values: Default::default(),
        }
    }
}

impl<V: ValueTraits> StateTree<V> {
    #[cfg(feature = "std")]
    fn from_file(mut file: std::fs::File) -> Self {
        let mut result = Self::default();
        let mut buf_reader = codec::IoReader(std::io::BufReader::new(&mut file));
        let nb_item = u64::decode(&mut buf_reader).unwrap();
        dbg!("loading {} items", nb_item);

        for _ in 0..nb_item {
            let v = Value::<V>::decode(&mut buf_reader).unwrap();
            result.set(v.key, v.value);
        }
        result.persist = Some(file);

        result
    }
    #[cfg(feature = "std")]
    fn serialize(&mut self) {
        let Some(file) = self.persist.as_mut() else {
            return;
        };
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        dbg!("serializing {} items", self.values.len());
        (self.values.len() as u64).encode_to(file);
        for (_, v) in self.values.iter() {
            v.encode_to(file);
        }
        file.flush().unwrap();
    }

    fn get_value(&self, k: &[u8]) -> Option<&Value<V>> {
        let Some(i) = self.indexes.get(k) else {
            #[cfg(feature = "std")]
            {
                let ix = tree_index_from_key(&k);
                self.tree.witness_access(ix);
            }
            return None;
        };

        let v = self.values.get(i);
        #[cfg(feature = "std")]
        {
            if let Some(value_v) = v.as_ref() {
                if !self.witness_values.borrow().contains_key(&value_v.key) {
                    self.witness_values
                        .borrow_mut()
                        .insert(value_v.key.clone(), value_v.value.clone());
                }
            }
            self.tree.witness_access(*i);
        }
        return v;
    }

    pub fn get(&self, k: &[u8]) -> Option<&V> {
        let v = self.get_value(k)?;
        if v.key.as_slice() != k {
            return None;
        };
        Some(&v.value)
    }

    // fail on key collision by returning false
    pub fn set(&mut self, k: Vec<u8>, v: V) -> bool {
        let ix = tree_index_from_key(&k);
        if let Some(existing) = self.values.get_mut(&ix) {
            if existing.key.as_slice() == k {
                existing.value = v;
                self.tree.insert(ix, existing.merkle_value());
            } else {
                #[cfg(feature = "std")]
                let _ = self.get(k.as_slice()); // register witness (we access value to check key).
                return false;
            }
        } else {
            let value = Value { key: k, value: v };
            self.tree.insert(ix, value.merkle_value());
            self.indexes.insert(value.key.clone(), ix);
            self.values.insert(ix, value);
        }
        true
    }

    pub fn root(&self) -> &Hash {
        self.tree.root()
    }

    fn init_from_witness(
        witness_hashes: &[(TreeIndex, Hash)],
        witness_key_values: Vec<(Vec<u8>, V)>,
    ) -> Option<Self> {
        let mut result = Self::default();
        // insert all witness hashes
        for (index, hash) in witness_hashes.iter() {
            result.tree.hashes.insert(*index, *hash);
        }
        let witness_root = *result.root();
        for (key, value) in witness_key_values.into_iter() {
            result.set(key, value);
            // set should not change root injected from hashes.
            if result.root() != &witness_root {
                return None;
            }
        }

        Some(result)
    }
}

#[derive(Default)]
pub struct KnownTokens {
    token_ids: Vec<TokenId>,
    #[cfg(feature = "std")]
    persist: Option<std::fs::File>,
    #[cfg(feature = "std")]
    witness: Vec<TokenId>,
}

impl Drop for KnownTokens {
    fn drop(&mut self) {
        #[cfg(feature = "std")]
        self.serialize();
    }
}

impl MerkleValue for KnownTokens {
    fn merkle_value(&self) -> Hash {
        if self.token_ids.len() > 0 {
            hash_sequence(self.token_ids.as_slice())
        } else {
            EMPTY_HASH
        }
    }
}

impl KnownTokens {
    #[cfg(feature = "std")]
    fn from_file(mut file: std::fs::File) -> Self {
        let mut result = Self::default();
        let mut buf_reader = codec::IoReader(std::io::BufReader::new(&mut file));
        result.token_ids = Decode::decode(&mut buf_reader).unwrap();
        result.persist = Some(file);
        result
    }

    #[cfg(feature = "std")]
    fn serialize(&mut self) {
        let Some(file) = self.persist.as_mut() else {
            return;
        };
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let encoded = self.token_ids.encode();
        dbg!("serializing {} token bytes", encoded.len());
        file.write_all(encoded.as_slice()).unwrap();
        file.flush().unwrap();
    }

    fn push_token(&mut self, token_id: TokenId) {
        if !self.token_ids.iter().any(|t| t == &token_id) {
            self.token_ids.push(token_id);
        }
    }
}

pub type Balance = u64;

impl MerkleValue for Balance {
    fn merkle_value(&self) -> Hash {
        let mut result = [0; 32];
        result[0..8].copy_from_slice(u64::to_le_bytes(*self).as_slice());
        result
    }
}

impl MerkleValue for TokenId {
    fn merkle_value(&self) -> Hash {
        let mut result = [0; 32];
        // could actually put 8 tokenid per hash_value, no need here
        result[0..4].copy_from_slice(u32::to_le_bytes(*self).as_slice());
        result
    }
}

#[derive(Clone, Encode, Decode)]
pub struct Value<V> {
    value: V,
    key: Vec<u8>,
}

impl<V: MerkleValue> MerkleValue for Value<V> {
    fn merkle_value(&self) -> Hash {
        hash_multiple(&[self.value.merkle_value().as_slice(), self.key.as_slice()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state() {
        let empty: State = Default::default();

        assert_eq!([0; 32], empty.get_root());
    }
    #[test]
    fn check_constants() {
        assert_eq!(TREE_DEPTH + 1, std::mem::size_of::<TreeIndex>() * 8);
    }
    #[test]
    fn create_token_and_distribute() {
        let mut state: State = Default::default();

        state.set_balance([1; 32], 1, 10);

        assert!([0; 32] != state.get_root());

        let witness_1 = state.take_witness();
        let mut state2 = State::from_witness(witness_1).unwrap();
        state2.set_balance([1; 32], 1, 10);
        assert_eq!(state.get_root(), state2.get_root());

        assert_eq!(Some(10), state.get_balance([1; 32], 1));
        // get balance witness
        let witness_2 = state.take_witness();
        let state3 = State::from_witness(witness_2).unwrap();
        assert_eq!(state.get_root(), state3.get_root());
        assert_eq!(Some(10), state3.get_balance([1; 32], 1));

        // insert token and insert another value
        state.set_balance([8; 32], 2, 32);
        state.known_tokens_push(1);
        state.known_tokens_push(2);
        let witness_3 = state.take_witness();

        let mut state4 = State::from_witness(witness_3).unwrap();
        assert_eq!(state3.get_root(), state4.get_root());
        state4.set_balance([8; 32], 2, 32);
        state4.known_tokens_push(1);
        state4.known_tokens_push(2);
        assert_eq!(state.get_root(), state4.get_root());

        // test serialize
        let dir = tempfile::tempdir().unwrap();
        let mut balances_file_path = dir.path().to_path_buf();
        balances_file_path.push("balances");
        let mut tokens_file_path = dir.path().to_path_buf();
        tokens_file_path.push("tokens");
        let balances_file = std::fs::File::create_new(&balances_file_path).unwrap();
        let tokens_file = std::fs::File::create_new(&tokens_file_path).unwrap();
        state.set_new_persist_files(balances_file, tokens_file);
        let root_bef_ser = state.get_root();
        core::mem::drop(state);

        let balances_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&balances_file_path)
            .unwrap();
        let tokens_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&tokens_file_path)
            .unwrap();
        let state_from_ser = State::from_files(balances_file, tokens_file);

        assert_eq!(root_bef_ser, state_from_ser.get_root());
    }
}
