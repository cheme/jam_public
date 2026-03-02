//! refinement

use alloc::vec::Vec;
use codec::{Decode, Encode};
use jam_pvm_common::error;

#[derive(Encode, Decode)]
pub struct Payload {
    pub operations: crate::external_client::Operations,
    pub witness: crate::external_client::state::Witness,
}

pub fn refine_payload(mut payload: &[u8]) -> (Vec<u8>, usize) {
    let Payload {
        operations,
        witness,
    } = match Payload::decode(&mut payload) {
        Ok(ops) => ops,
        Err(e) => {
            error!("Failed to parse signed operations: {}", e);
            return (Vec::new(), 0);
        }
    };

    let operations_len = operations.len();
    let opt_partial_state = crate::external_client::state::State::from_witness(witness);
    if opt_partial_state.is_none() {
        unimplemented!("TODO error report in work output ?");
    }
    let mut partial_state = opt_partial_state.unwrap();
    let previous_root = partial_state.get_root();
    crate::external_client::state_transition(&mut partial_state, operations);
    let new_root = partial_state.get_root();

    #[cfg(feature = "single_payload")]
    (
        crate::accumulation::Operation {
            previous_root,
            new_root,
        }
        .encode(),
        operations_len,
    )
}
