//! accumulation

use crate::rollup::RollupHash;
use codec::{Decode, Encode};
use jam_pvm_common::accumulate::{get, set};
use jam_pvm_common::{error, info, warn};
use jam_types::AccumulateItem;
use jam_types::WorkItemRecord;

#[cfg(feature = "single_payload")]
#[derive(Clone, Debug, Encode, Decode)]
pub struct Operation {
    pub previous_root: RollupHash,
    pub new_root: RollupHash,
}

#[cfg(feature = "single_payload")]
type ItemAccumulate = Result<Option<RollupHash>, ()>;

pub fn on_work_items(items: Vec<AccumulateItem>) {
    #[cfg(not(all(feature = "single_payload", feature = "drop_all_on_fail")))]
    unimplemented!();

    let mut items_result = Ok(None);

    for item in items {
        info!("Accumulate processing work item record");
        match item {
            AccumulateItem::WorkItem(r) => on_work_item(r, &mut items_result),
            AccumulateItem::Transfer(_) => panic!("not used in this example"),
        }
    }

    match items_result {
        Ok(Some(new_root)) => {
            // TODO manage error
            set("rollup_root", new_root).unwrap();
            info!("Rollup state transition success");
        }
        Ok(None) => {
            info!("Rollup unchanged");
        }
        Err(()) => {
            error!("Mismatch root, skipping all transition");
        }
    }
}

pub fn on_work_item(record: WorkItemRecord, acc: &mut ItemAccumulate) {
    if acc.is_err() {
        return;
    }
    info!(
        "Accumulate processing work item record: package {:?}",
        record.package
    );
    let output = match record.result {
        Ok(output) => output,
        Err(e) => {
            warn!("Work item failed: {:?}", e);
            return;
        }
    };

    let Ok(op) = Operation::decode(&mut &output[..]) else {
        warn!("Failed to decode validated operation");
        return;
    };

    info!("Processing rollup state transition operations");
    let current_root = if let Ok(Some(r)) = acc {
        *r
    } else {
        get("rollup_root").unwrap_or_default()
    };
    if op.previous_root == current_root {
        *acc = Ok(Some(op.new_root));
    } else {
        error!("Mismatch root, skipping all transition");
        *acc = Err(());
    }
}
