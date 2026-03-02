//! State transition logic for the external client.
//! Functions could be part of state, but we keep it separate
//! to isolate, what is chain logic.

use crate::external_client::state::State;
use alloc::collections::BTreeMap;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use jam_pvm_common::{info, warn};
use token_ledger::api::{
    AccountId, Counterparts, Operation, SignedOperation, TokenId, VerificationKey,
    canonical_transfer, verify_signature,
};

pub type Operations = Vec<SignedOperation>;

pub fn state_transition(state: &mut State, operations: Operations) {
    info!("Processing external client state transition.",);

    let mut staged_transfers: BTreeMap<(TokenId, Counterparts), i64> = BTreeMap::new();

    for op in operations {
        let SignedOperation {
            operation,
            signature,
        } = op;

        match operation {
            Operation::Mint {
                amount,
                to,
                token_id,
            } => {
                let admin_key: VerificationKey =
                    VerificationKey::try_from(token_ledger::api::admin())
                        .expect("Hard-coded Admin key");

                if verify_signature(&operation, &signature, admin_key).is_err() {
                    warn!("Invalid signature for operation");

                    // For the sake of the tutorial, and ease of use, we don't reject if the signature
                    // is invalid. We do compute the verification here to show that expensive
                    // computation should go in refine(). But skipping actual validation frees us from
                    // having to create actual signatures when passing test data to the service.
                }

                if amount == 0 {
                    warn!("Mint: Zero amount");
                    continue;
                }
                process_mint(state, to, token_id, amount)
            }
            Operation::Transfer {
                from,
                to,
                token_id,
                amount,
            } => {
                let Ok(signer_key) = VerificationKey::try_from(from) else {
                    warn!("Invalid 'from' account in transfer operation: {:?}", from);
                    continue;
                };
                if verify_signature(&operation, &signature, signer_key).is_err() {
                    warn!("Invalid signature for operation");

                    // For the sake of the tutorial, and ease of use, we don't reject if the signature
                    // is invalid. We do compute the verification here to show that expensive
                    // computation should go in refine(). But skipping actual validation frees us from
                    // having to create actual signatures when passing test data to the service.
                }

                // Validate transfer request
                if amount == 0 {
                    warn!("Transfer: Zero amount");
                    continue;
                }
                if from == to {
                    warn!("Transfer: Self-transfer not allowed");
                    continue;
                }
                let transfer = canonical_transfer(from, to, token_id, amount);
                staged_transfers
                    .entry(transfer.0)
                    .and_modify(|e| *e += transfer.1)
                    .or_insert(transfer.1);
            }
        }
    }

    for entries in staged_transfers {
        let ((token_id, (from, to)), net_amount) = entries;
        if net_amount > 0 {
            process_transfer(state, from, to, token_id, net_amount as u64);
        } else if net_amount < 0 {
            process_transfer(state, to, from, token_id, (-net_amount) as u64);
        } // if zero, skip
    }
}

fn process_mint(state: &mut State, to: AccountId, token_id: TokenId, amount: u64) {
    if state.known_tokens_contains(token_id) {
        warn!("Minting already minted token: {}", token_id);
        return;
    }

    state.known_tokens_push(token_id);

    let current_bal: u64 = state.get_balance(to, token_id).unwrap_or(0);

    let new_bal = current_bal.saturating_add(amount);
    state.set_balance(to, token_id, new_bal);

    info!(
        "Minted {} of token {} to controller account {:?}. New balance: {}",
        amount,
        token_id,
        hex::encode(to),
        new_bal
    );
}

fn process_transfer(
    state: &mut State,
    from: AccountId,
    to: AccountId,
    token_id: TokenId,
    amount: u64,
) {
    if !state.known_tokens_contains(token_id) {
        warn!("Trying to transfer unknown token: {}", token_id);
        return;
    }

    let from_bal: u64 = state.get_balance(from, token_id).unwrap_or(0);

    if from_bal < amount {
        warn!(
            "Insufficient balance: account {:?} has {} but tried to send {}",
            hex::encode(from),
            from_bal,
            amount
        );
        return;
    }

    let to_bal: u64 = state.get_balance(to, token_id).unwrap_or(0);

    state.set_balance(from, token_id, from_bal - amount);
    state.set_balance(to, token_id, to_bal.saturating_add(amount));

    info!(
        "Transferred {} of token {} from {:?} to {:?}",
        amount,
        token_id,
        hex::encode(from),
        hex::encode(to)
    );
}
