//! accumulation

use codec::{Encode, Decode};
use crate::rollup::RollupHash;

#[cfg(feature = "single_payload")]
#[derive(Clone, Debug, Encode, Decode)]
pub struct Operation {
    pub previous_root: RollupHash,
    pub new_root: RollupHash,
}

// TODO rem: is feature gate
#[derive(Clone, Debug, Encode, Decode)]
pub enum OperationToRem {
    SingleWorkpayload(Operation),
    // TODO
    SingleWorkPackage,
    // TODO
    Multiple,
}
