//! access proof to load before refining
//!
//! TODO just check all items against root and load all decoded in a btreemap at start
//! TODO a struct wrapping key hashing over such btreemap (only get and insert).
//! TODO on insert: run a pending change and log its ix
//! TODO allow calculating new root from this : btreemap of witness hash by ix on all changed
//! -> ordered so get list of ix to update, next value: update all value of the list up to common
//! find common between n and n + 1,
//! calc stack of first proof new hash up to common, replace with n + 1 hashes up to common and
//! repeat.

use crate::rollup::{RollupHash, TreeIndex, TREE_DEPTH};
use alloc::{collections::BTreeMap, vec::Vec};
use codec::{Decode, Encode};

// items must be ordered: btreemap is used when building
pub struct Witness(pub Vec<WitnessItem>);

pub struct WitnessItem {
    index: TreeIndex,
    encoded_value: Vec<u8>,
    siblings: [RollupHash; TREE_DEPTH],
}

impl Witness {
    pub fn new_root<V: Encode>(&self, change_set: &BTreeMap<TreeIndex, V>) -> RollupHash {
        // we have a witness for each insert (insert involves a get)
        let mut stack: [RollupHash; TREE_DEPTH] = [[0; 32]; TREE_DEPTH];
        let mut stack_ix: u16 = 0;
        let mut prev_value: Option<&V> = None;

        let mut witness_ix = 0;

        for change in change_set.iter() {
            while witness_ix < self.0.len() {
                if *change.0 == self.0[witness_ix].index {
                    // TODO common from stack_ix
										// TODO calc parent hash at common from stack and prev value
                    // TODO fill stack with witness hash up to new calculated
                    witness_ix += 1;

                    stack_ix = *change.0;
										prev_value = Some(change.1);
                    break;
                }
                witness_ix += 1;
            }
        }

        stack[0]
    }
}
