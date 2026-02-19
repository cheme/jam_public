#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std)]

/// This is a simple implementation of an example, as part of a tutorial on how to build services
/// for JAM. Although it demonstrates the basic concepts and techniques, it should is not
/// production-ready and should not be used as is in production.
extern crate alloc;

use alloc::{collections::BTreeMap, vec::Vec};
use codec::Encode;
use jam_pvm_common::{Service, accumulate, declare_service, error, info, warn};
use jam_types::{
    AccumulateItem, CoreIndex, Hash, ServiceId, Slot, WorkOutput, WorkPackageHash, WorkPayload,
};
use token_ledger_common::VerificationKey;

mod accumulation;
mod refinement;

use token_ledger_common::json;

/// The Token Ledger Service
pub struct TokenLedger;
declare_service!(TokenLedger);

impl Service for TokenLedger {
    fn refine(
        _core_index: CoreIndex,
        item_index: usize,
        service_id: ServiceId,
        payload: WorkPayload,
        package_hash: WorkPackageHash,
    ) -> WorkOutput {
        use refinement::SignedOperation;
        use token_ledger_common::{Counterparts, TokenId};
        // TODO: by casting a transfer's u64 to i64, to preserve the direction,
        // we are reducing 2-fold the effective maximum amount. This can be dealt in a few different ways,
        // be it by keeping track of a direction flag, or even by keeping one cumulative balance
        // for each possible direction. For now, we just assume that all transfers are up to i64::MAX.
        let mut staged_transfers: BTreeMap<(TokenId, Counterparts), i64> = BTreeMap::new();
        info!(
            "TokenLedger refine on service {service_id:x}h for package/item {package_hash} / {item_index}"
        );

        // Parse the incoming payload as a JSON array of signed operations
        let operations: Vec<SignedOperation> = match json::parse_signed_operations(&payload) {
            Ok(ops) => ops,
            Err(e) => {
                error!("Failed to parse signed operations: {}", e);
                return Vec::new().into();
            }
        };

        let mut validated: Vec<accumulation::ValidatedOperation> = Vec::new();

        for signed_op in operations {
            let SignedOperation {
                operation,
                signature,
            } = signed_op;

            match operation {
                refinement::Operation::Mint { amount, .. } => {
                    let admin_key: VerificationKey =
                        VerificationKey::try_from(token_ledger_common::admin())
                            .expect("Hard-coded Admin key");

                    if refinement::verify_signature(&operation, &signature, admin_key).is_err() {
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
                    validated.push(accumulation::ValidatedOperation(operation));
                }
                refinement::Operation::Transfer {
                    from,
                    to,
                    token_id,
                    amount,
                } => {
                    let Ok(signer_key) = VerificationKey::try_from(from) else {
                        warn!("Invalid 'from' account in transfer operation: {:?}", from);
                        continue;
                    };
                    if refinement::verify_signature(&operation, &signature, signer_key).is_err() {
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
                    let transfer = refinement::canonical_transfer(from, to, token_id, amount);
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
                validated.push(accumulation::ValidatedOperation(
                    refinement::Operation::Transfer {
                        from,
                        to,
                        token_id,
                        amount: net_amount as u64,
                    },
                ));
            } else if net_amount < 0 {
                validated.push(accumulation::ValidatedOperation(
                    refinement::Operation::Transfer {
                        from: to,
                        to: from,
                        token_id,
                        amount: (-net_amount) as u64,
                    },
                ));
            } // if zero, skip
        }

        info!(
            "TokenLedger refine: Validated {} operations for accumulation",
            validated.len()
        );

        // Encode and return for accumulation
        let encoded = validated.encode();
        info!("Refinement total output size: {} bytes", encoded.len());
        encoded.into()
    }

    fn accumulate(slot: Slot, service_id: ServiceId, item_count: usize) -> Option<Hash> {
        info!("TokenLedger accumulate on service {service_id:x}h @{slot} with {item_count} items");

        for item in accumulate::accumulate_items() {
            info!("Accumulate processing work item record");
            match item {
                AccumulateItem::WorkItem(r) => accumulation::on_work_item(r),
                AccumulateItem::Transfer(t) => accumulation::on_transfer(t),
            }
        }

        None
    }
}

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
