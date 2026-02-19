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
pub struct TokenLedgerRollup;
declare_service!(TokenLedgerRollup);

impl Service for TokenLedgerRollup {
    fn refine(
        _core_index: CoreIndex,
        item_index: usize,
        service_id: ServiceId,
        payload: WorkPayload,
        package_hash: WorkPackageHash,
    ) -> WorkOutput {
        todo!();
    }

    fn accumulate(slot: Slot, service_id: ServiceId, item_count: usize) -> Option<Hash> {
        todo!();
    }
}

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
