

## Overview

This tutorial extends part 2 with a focus on:
- designing rollup like state: accounts are not stored anymore on jam state.
- discuss cost of such design.
- use a minimal rollup state implementation only for educational purpose.


This tutorial will not attempt to:
- scalable, we use a very simple bounded, unoptimal, merkle state and proofs. For real use a proper third party implementation of state and state storage should be use.
- be secure, we keep skipping signature checks.
- implement state distribution: each client should sink upon the last state root read in jam state, this can be using different strategy. Here a disconnected client will lose ability to synch state if work items got pruned.
- define proper role for distribution: every client are just validator that accessed directly the jam datalake, on a real implementation, distribution strategy must fit the usecase.
- rollup must handle fail or success accumulate processing, here we assume it will always succeed, a failure will put client in an invalid state. TODO can we draft a simple cow state?

We thus remain at service level.


## Testing

This tutorial can run the same examples as the token ledger one. One will observe that the logs are slightly different:
- transfer are noted in refine
- transfer in refine are asociated with a workpackage hash and workitem (we could have a single extrinsic root)_
- accumulate advance state root
- accumulate display workitem processed or failure (can fail if two workpackage try to advance same rollup state: only one get processed, failure need to be handled properly though).

## TODO parallel resolution on accumulate : launch two transfer json leading to invalid. 
    - root building from prefixes
    - merge state with both package ids -> then refine merge step (do not accept futher refine in merge state after x time). TODO plus client side a merge state should suspend until not merge and rebase tx on merged state root.

## TODO add authorizer to system
