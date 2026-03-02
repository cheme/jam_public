//! refinement

use alloc::vec::Vec;
use codec::{Decode, Encode};
use jam_pvm_common::{error, info};

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

    info!("witness {:?}", &witness);
    let operations_len = operations.len();
    info!(
        "read payload of size {}, with {} operations",
        payload.len(),
        operations_len
    );
    let opt_partial_state = crate::external_client::state::State::from_witness(witness);
    if opt_partial_state.is_none() {
        error!("error loading state");
        unimplemented!("TODO error report in work output ?");
    }
    let mut partial_state = opt_partial_state.unwrap();
    info!("loaded state from witness");
    let previous_root = partial_state.get_root();
    info!("from root: {:?}", previous_root);
    crate::external_client::state_transition(&mut partial_state, &operations);
    let new_root = partial_state.get_root();
    info!("to root: {:?}", new_root);

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
