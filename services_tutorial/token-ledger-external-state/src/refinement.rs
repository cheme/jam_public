//! refinement

use codec::{Decode, Encode};

#[derive(Encode, Decode)]
pub struct Payload {
    pub operations: crate::external_client::Operations,
    pub witness: crate::external_client::state::Witness,
}
