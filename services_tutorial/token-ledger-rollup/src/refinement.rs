//! refinement

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
use alloc::vec::Vec;
use codec::{Decode, Encode};

#[derive(Encode, Decode)]
pub struct Payload {
    pub operations: crate::rollup::Operations,
    pub witness: crate::rollup::state::Witness,
}
