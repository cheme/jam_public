#![cfg_attr(not(feature = "std"), no_std)]

//! This is a simple implementation of an example, as part of a tutorial on how to build services
//! for JAM. Although it demonstrates the basic concepts and techniques, it should is not
//! production-ready and should not be used as is in production.
//!
//! No error management is provided, and no consistency guarantees for persistence as this
//! targets example/tutorial focused on others aspects.
//!
//! Building client or running tests requires to manually set "std" feature.

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode};
use jam_pvm_common::{Service, accumulate, declare_service, error, info};
use jam_types::{CoreIndex, Hash, ServiceId, Slot, WorkOutput, WorkPackageHash, WorkPayload};

mod accumulation;
pub mod external_client;
mod refinement;

/// The Token Ledger Service
pub struct TokenLedgerExternalClient;
declare_service!(TokenLedgerExternalClient);

impl Service for TokenLedgerExternalClient {
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

        let refinement::Payload {
            operations,
            witness,
        } = match refinement::Payload::decode(&mut payload.0.as_slice()) {
            Ok(ops) => ops,
            Err(e) => {
                error!("Failed to parse signed operations: {}", e);
                return Vec::new().into();
            }
        };

        let operations_len = operations.len();
        let opt_partial_state = crate::external_client::state::State::from_witness(witness);
        if opt_partial_state.is_none() {
            unimplemented!("TODO error report in work output ?");
        }
        let mut partial_state = opt_partial_state.unwrap();
        let previous_root = partial_state.get_root();
        crate::external_client::state_transition(&mut partial_state, operations);
        let new_root = partial_state.get_root();

        #[cfg(feature = "single_payload")]
        let encoded = crate::accumulation::Operation {
            previous_root,
            new_root,
        }
        .encode();

        info!("Refinement done over {} operations", operations_len);
        encoded.into()
    }

    fn accumulate(slot: Slot, service_id: ServiceId, item_count: usize) -> Option<Hash> {
        info!("TokenLedger accumulate on service {service_id:x}h @{slot} with {item_count} items");

        crate::accumulation::on_work_items(accumulate::accumulate_items());
        None
    }
}

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
