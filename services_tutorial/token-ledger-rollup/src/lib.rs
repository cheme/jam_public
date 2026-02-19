#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std)]

/// This is a simple implementation of an example, as part of a tutorial on how to build services
/// for JAM. Although it demonstrates the basic concepts and techniques, it should is not
/// production-ready and should not be used as is in production.
extern crate alloc;

use alloc::{collections::BTreeMap, vec::Vec};
use codec::Encode;
use jam_pvm_common::{accumulate, declare_service, error, info, warn, Service};
use jam_types::{
    AccumulateItem, CoreIndex, Hash, ServiceId, Slot, WorkOutput, WorkPackageHash, WorkPayload,
};
use token_ledger_common::VerificationKey;

mod accumulation;
mod refinement;
mod rollup;

use token_ledger_common::json;

/// The Token Ledger Service
pub struct TokenLedgerRollup;
declare_service!(TokenLedgerRollup);

enum ConcurrencyMode {
    // one workitem produced to accumulate, no concurency, accumulate simply change root, all is
    // checked in refine
    SingleWorkpayload,
    // multiple workitem to pass in a single accumulate for a simple state transition.
    // accumulate produce changed root from input
    SingleWorkPackage,
    // multiple workitem over multiple accumulate calls.
    Multiple,
}

#[cfg(feature = "single_payload")]
const CONCURRENCY: ConcurrencyMode = ConcurrencyMode::SingleWorkpayload;
#[cfg(feature = "single_package")]
const CONCURRENCY: ConcurrencyMode = ConcurrencyMode::SingleWorkPackage;
#[cfg(feature = "multiple")]
const CONCURRENCY: ConcurrencyMode = ConcurrencyMode::Multiple;

enum FailureManagement {
    // all ops are drops if a single fail.
    DropAll,
    // ops are tracked and failed ops in accumulate are just ignored
    TrackOps,
}

#[cfg(feature = "drop_all_on_fail")]
const FAILURE: FailureManagement = FailureManagement::DropAll;
#[cfg(feature = "track_fail")]
const FAILURE: FailureManagement = FailureManagement::TrackOps;

impl Service for TokenLedgerRollup {
    fn refine(
        _core_index: CoreIndex,
        item_index: usize,
        service_id: ServiceId,
        payload: WorkPayload,
        package_hash: WorkPackageHash,
    ) -> WorkOutput {
        //use token_ledger_common::{Counterparts, TokenId};
        info!(
            "TokenLedger refine on service {service_id:x}h for package/item {package_hash} / {item_index}"
        );

        // Parse the incoming payload as a JSON array of signed operations
        let operations = refinement::Operation(match json::parse_signed_operations(&payload) {
            Ok(ops) => ops,
            Err(e) => {
                error!("Failed to parse signed operations: {}", e);
                return Vec::new().into();
            }
        });

        for signed_op in operations.0 {
            // TODO add proof to signed operation Need new client ops that run on top of a db to
            // extract proof against read json and write new json with attached proof.
            // TODO also just encode json to encoded so no longer json in refine/accumulate
            // TODO an util to pipe json to encoded payload file -> this file attach also previous
            // root and call on it.
            let token_ledger_common::api::SignedOperation {
                operation,
                signature,
            } = signed_op;

            match operation {
                token_ledger_common::api::Operation::Mint { amount, .. } => {
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
                        warn!("Mint: Zero amount skipping");
                        continue;
                    }
                }
                token_ledger_common::api::Operation::Transfer { .. } => {
                    warn!("unimpl");
                }
            }
        }

        let encoded = crate::accumulation::Operation {
            previous_root: Default::default(),
            new_root: Default::default(),
        }
        .encode();

        info!("Refinement total output size: {} bytes", encoded.len());
        encoded.into()
    }

    fn accumulate(_slot: Slot, _service_id: ServiceId, _item_count: usize) -> Option<Hash> {
        return None;
    }
}

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
