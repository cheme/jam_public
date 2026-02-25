//! rollup state on client size.
//! For the sake of keeping example concise, client implementation always register state
//! and manage failible persistence, non client (pvm) do not.
//! Client implementation is obtain by including "std" feature, while pvm is obtain by
//! building no std.

use super::{
    hash_key, hash_multiple, hash_pair, hash_sequence, tree_index_from_key, HashValue,
    RollupHash as Hash, TreeIndex, EMPTY_HASH, TREE_DEPTH,
};

use alloc::collections::BTreeMap;
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
use alloc::vec::Vec;
use codec::{Decode, Encode};
use token_ledger_common::{AccountId, TokenId};

#[cfg(feature = "std")]
use core::cell::RefCell;
#[cfg(feature = "std")]
use std::io::{Read, Write};

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
    // we use a refcell as client implementation is only for testing
    // and run on a single thread, this way we keep sane prototyping
    // for non client code.
    #[cfg(feature = "std")]
    witness: RefCell<BTreeMap<TreeIndex, Hash>>,
}

impl MerkleTree {
    fn start_processing(&self) {
        #[cfg(feature = "std")]
        {
            self.witness.borrow_mut().clear();
        }
    }
    pub fn from_state(state: BTreeMap<TreeIndex, Hash>) -> Self {
        let mut result = Self::default();
        result.hashes = state;
        result
    }

    // Note this must always be call to access any `hashes field content`,
    // so we register witness properly.
    fn get_hash(&self, ix: TreeIndex) -> &Hash {
        if let Some(hash) = self.hashes.get(&ix) {
            #[cfg(feature = "std")]
						if !self.witness.borrow().contains_key(&ix) {
							self.witness.borrow_mut().insert(ix, *hash);
						}
            return hash;

        } else {
            return &EMPTY_HASH;
        }
    }
    pub fn root(&self) -> &Hash {
        return self.get_hash((TREE_HASHES - 1) as TreeIndex);
    }

    pub fn insert(&mut self, ix: TreeIndex, value_hash: Hash) {
        let mut hash = value_hash;
        let mut at = ix;
        let mut offset: TreeIndex = 0;
        for depth in 0..TREE_DEPTH {
            self.hashes.insert(offset + at, value_hash);
            if at % 2 == 0 {
                hash = hash_pair(
                    &hash,
                    self.hashes.get(&(offset + at + 1)).unwrap_or(&EMPTY_HASH),
                );
            } else {
                hash = hash_pair(
                    self.hashes.get(&(offset + at - 1)).unwrap_or(&EMPTY_HASH),
                    &hash,
                );
            }
            offset += 1 << (TREE_DEPTH - depth);
            at = at / 2;
        }
        self.hashes.insert(offset + at, value_hash);
    }
}

#[derive(Default)]
pub struct State {
    root: Hash,
    balances: StateTree<Balance>,
    tokens: KnownTokens,
}

#[derive(Default, Encode, Decode)]
pub struct Witness {
	// root is part of the hashes
	hashes: Vec<(TreeIndex, Hash)>,
	key_value_balances: Vec<(Vec<u8>, Balance)>,
	// Currently no operation make sense without accessing it so always store.
	token_ids: Vec<TokenId>,
}

impl State {
    #[cfg(feature = "std")]
    pub fn take_witness(&mut self, account: AccountId, token_id: TokenId, balance: u64) -> Witness {
			let mut key_value_balances: Vec<(Vec<u8>, Balance)> = Default::default();
			let mut hashes = std::mem::replace(self.balances.tree.witness.get_mut(), BTreeMap::<TreeIndex, Hash>::default());
			let mut values = std::mem::replace(self.balances.witness_values.get_mut(), BTreeMap::<Vec<u8>, Balance>::default());

			return Witness {
				hashes: hashes.into_iter().collect(),
				key_value_balances: values.into_iter().collect(),
				token_ids: std::mem::take(&mut self.tokens.witness_tokens),
			};

		}

		// TODO proper error
    pub fn from_witness(witness: Witness) -> Option<Self> {
			let mut result = Self::default();
   if let Some(balances) = StateTree::init_from_witness(&witness.hashes, witness.key_value_balances) {
result.balances= balances;
 } else {
	 return None;
	 }
result.tokens.tokens = witness.token_ids;
result.tokens.update_hash();
result.update_hash();
return Some(result);

		}

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
        self.root = hash_pair(self.balances.root(), &self.tokens.root);
    }
}

trait ValueTraits: Clone + Decode + Encode + HashValue {}

impl<V: Clone + Decode + Encode + HashValue> ValueTraits for V {}

// just write all added key value and replay when opening
pub struct SerializedState {}

pub struct StateTree<V: ValueTraits> {
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
        while let Ok(item) = <(Vec<u8>, Vec<u8>)>::decode(&mut buf_reader) {
            let v = V::decode(&mut item.1.as_slice()).unwrap();
            result.insert(item.0, v);
        }
        result.persist = Some(file);
        result
    }
    #[cfg(feature = "std")]
    fn serialize(&mut self) {
        let Some(file) = self.persist.as_mut() else {return };
        file.set_len(0).unwrap();
        for (_, v) in self.values.iter() {
            file.write_all((&v.key, v.value.encode()).encode().as_slice())
                .unwrap();
        }
        file.flush().unwrap();
    }

    fn get_value(&self, k: &[u8]) -> Option<&Value<V>> {
        let i = self.indexes.get(k)?;
        let v = self.values.get(i);
				#[cfg(feature = "std")]
				if let Some(value_v) = v.as_ref() {
						if !self.witness_values.borrow().contains_key(&value_v.key) {
							self.witness_values.borrow_mut().insert(value_v.key.clone(), value_v.value.clone());
						}
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
    pub fn insert(&mut self, k: Vec<u8>, v: V) -> bool {
        let ix = tree_index_from_key(&k);
        if let Some(existing) = self.values.get_mut(&ix) {
						#[cfg(feature = "std")]
						if !self.witness_values.borrow().contains_key(&k) {
							self.witness_values.get_mut().insert(k.clone(), existing.value.clone());
						}
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

    fn init_from_witness(
        witness_hashes: &[(TreeIndex, Hash)],
        witness_key_values: Vec<(Vec<u8>, V)>,
//        expected_root: Option<Hash>,
    ) -> Option<Self> {
        let mut result = Self::default();
        // insert all witness hashes
        for (index, hash) in witness_hashes.iter() {
            result.tree.hashes.insert(*index, *hash);
        }
        let witness_root = *result.root();
//        if expected_root
//            .map(|expected_root| expected_root != witness_root)
//            .unwrap_or(false)
//        {
//            return None;
//        }
        for (key, value) in witness_key_values.into_iter() {
            let ix = tree_index_from_key(key.as_slice());
            let value_tuple = Value {
                key: key.clone(),
                value,
            };
            result.tree.insert(ix, value_tuple.hash_value());
            if result.root() != &witness_root {
                return None;
            }
            result.indexes.insert(key, ix);
            result.values.insert(ix, value_tuple);
        }

        #[cfg(feature = "std")]
        result.tree.witness.get_mut().clear();
        #[cfg(feature = "std")]
        result.witness_values.get_mut().clear();
        Some(result)
    }
}

#[derive(Default)]
pub struct KnownTokens {
    tokens: Vec<TokenId>,
    // default hash on 0 len vec
    root: Hash,
    #[cfg(feature = "std")]
    persist: Option<std::fs::File>,
    #[cfg(feature = "std")]
    witness_tokens: Vec<TokenId>,
}

impl Drop for KnownTokens {
    fn drop(&mut self) {
        #[cfg(feature = "std")]
        self.serialize();
    }
}

impl KnownTokens {
    fn update_hash(&mut self) {
        if self.tokens.len() > 0 {
            self.root = hash_sequence(self.tokens.as_slice());
        } else {
            self.root = EMPTY_HASH;
        }
    }

    #[cfg(feature = "std")]
    fn from_file(mut file: std::fs::File) -> Self {
        let mut result = Self::default();
        let mut buf_reader = codec::IoReader(std::io::BufReader::new(&mut file));
        result.tokens = Decode::decode(&mut buf_reader).unwrap();
        result.update_hash();
        result.persist = Some(file);
        result
    }
    #[cfg(feature = "std")]
    fn serialize(&mut self) {
        let Some(file) = self.persist.as_mut() else {return };
        file.set_len(0).unwrap();
        file.write_all(self.tokens.encode().as_slice()).unwrap();
        file.flush().unwrap();
    }
}

pub type Balance = u64;

impl HashValue for Balance {
    fn hash_value(&self) -> Hash {
        let mut result = [0; 32];
        result[0..8].copy_from_slice(u64::to_le_bytes(*self).as_slice());
        result
    }
}

impl HashValue for TokenId {
    fn hash_value(&self) -> Hash {
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

impl<V: HashValue> HashValue for Value<V> {
    fn hash_value(&self) -> Hash {
        hash_multiple(&[self.value.hash_value().as_slice(), self.key.as_slice()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state() {
        let empty: State = Default::default();

        assert_eq!([0; 32], empty.root);
    }
    #[test]
    fn check_constants() {
        assert_eq!(TREE_DEPTH + 1, std::mem::size_of::<TreeIndex>() * 8);
    }
    #[test]
    fn create_token_and_distribute() {
        let mut state: State = Default::default();

        // TODO rem, use primitives from ops
        state.insert_balance([1; 32], 1, 10);

        assert!([0; 32] != state.root);

        assert_eq!(Some(10), state.get_balance([1; 32], 1));
    }
}
