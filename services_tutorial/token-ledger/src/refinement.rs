// Code support for the refinement phase, including data types and expensive computations.
use token_ledger_common::{AccountId, Counterparts, TokenId};

// ledger api directly used by refine.
pub use token_ledger_common::api::{Operation, SignedOperation, verify_signature};

// Orders a transaction between two parties, so that for both possible directions of transfer,
// we always have the same party first, independently from being the sender or the recipient.
// This allows to keep track of the net balance between two parties over several transfers
// without having to keep track of the direction of each individual transfer.
pub fn canonical_transfer(
    from: AccountId,
    to: AccountId,
    token_id: TokenId,
    amount: u64,
) -> ((TokenId, Counterparts), i64) {
    let a = &from;
    let b = &to;
    let (counterparts, amount) = if a < b {
        ((*a, *b), amount as i64)
    } else {
        ((*b, *a), -(amount as i64))
    };
    ((token_id, (counterparts.0, counterparts.1)), amount)
}
