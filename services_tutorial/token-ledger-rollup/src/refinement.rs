//! refinement

use codec::{Encode, Decode};
use token_ledger_common::{AccountId, Counterparts, TokenId};
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
use alloc::vec::Vec;

// ledger api directly used by refine.
pub use token_ledger_common::{SignedOperation, verify_signature};


#[derive(Clone, Debug, Encode, Decode)]
pub struct Operation(pub Vec<SignedOperation>);

