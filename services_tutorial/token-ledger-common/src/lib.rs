#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std)]

/// Common code over example simple token leger implementation(s).
/// Only for example, tutorials.
/// It is puposedly not target production-ready and must not be used as is in production.
extern crate alloc;

pub use ed25519_consensus::{Signature, VerificationKey, VerificationKeyBytes};

// An auxiliary module for handling JSON-encoded data.
pub mod json;

pub mod api {
    use super::{AccountId, Signature, TokenId, VerificationKey};
    use codec::{Decode, Encode};

    /// Operations that can be submitted to the token ledger
    #[derive(Clone, Debug, Encode, Decode)]
    pub enum Operation {
        Mint {
            to: AccountId,
            token_id: TokenId,
            amount: u64,
        },
        Transfer {
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            amount: u64,
        },
    }

    /// A Refinement Operation with its authorization signature
    /// For this tutorial, we will define this authorization as a signature by the hard-coded admin
    /// account, covering the encoded operation.
    /// In real-world cases, developers might specify a specific format to the signature message.
    #[derive(Clone, Debug)]
    pub struct SignedOperation {
        pub operation: Operation,
        pub signature: Signature,
    }

    pub fn verify_signature(
        op: &Operation,
        signature: &Signature,
        key: VerificationKey,
    ) -> Result<(), &'static str> {
        let message = op.encode();
        key.verify(&signature, &message)
            .map_err(|_| "Signature verification failed")
    }
}

/// For demonstration only: a hard-coded admin account that must authorise every operation submitted
/// to the service. In a real-world scenario, developers must decide on a proper authorization
/// layer, deciding who can authorize operations and how the service would manage their identities
/// and keys.
pub fn admin() -> VerificationKeyBytes {
    [0_u8; 32].into()
}

/// A unique identifier for a token type
pub type TokenId = u32;

/// An account identifier (32-byte public key)
pub type AccountId = [u8; 32];

/// A set of two counterparts of a single transfer.
/// There is no implication of who the sender is, because we
/// want to accumulate several operations, in either direction,
/// between the same accounts.
/// The final net balance will determine the resultant transfer direction.
pub type Counterparts = (AccountId, AccountId);
