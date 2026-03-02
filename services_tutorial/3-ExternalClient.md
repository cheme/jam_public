

## Overview

This tutorial extends part 2 with a focus on:
- designing rollup/sidechain like state: accounts are not stored anymore on jam state.
- discuss cost of such design.
- use a minimal external client state implementation only for educational purpose.


This tutorial will not attempt to:
- scalable, we use a very simple bounded, unoptimal, merkle state and proofs. For real use a proper third party implementation of state and state storage should be use.
- be secure, we keep skipping signature checks.
- implement state distribution: each client should sink upon the last state root read in jam state, this can be using different strategy. Here a disconnected client will lose ability to synch state if work items got pruned.
- define proper role for distribution: every client are just validator that accessed directly the jam datalake, on a real implementation, distribution strategy must fit the usecase.
- external client must handle fail or success accumulate processing, here we assume it will always succeed, a failure will put client in an invalid state. TODO can we draft a simple cow state?

We thus remain at service level.

## distribution

simply use datalake of workpackage.

## Concurrency handling

Work package can define differents strategy, we define them in an enum to get all example in the same codebase choice is feature gated to ensure proper binary size.

- SingleWorkpayload: single payload containing all transaction send to refine, accumulate will just update state root of external client from a single workitem
- SingleWorkPackage: single work package with multiple payload, accumulate will resolve new root from validated updates and partial state from multiple workitem.
- Multiple: multiple work package, potentially at different time, add a delay for accumulation: batching multiple states.

TODO different accumulate reconciliation :
- fail all on conflict
- fail partial: relay ops status (pass or dropped)
- state change payload build from all passing ops (in accumulate or in client).


## State progress

Rollup state progress is only effective when accumulate change stored root.

In this we simply commit client side optimitically and have no rollback, so on first error in accumulate, our client procducing
workitems will be out of sync.

In real application, client would audit the accumulate (eg by runing a jam node).

A client seeing a changed root, must sync its internal state db to match it, depending on concurency handling, it will use:
- single workpayload: the workpayload in datalake that produced the workitem for this accumulate step.
- others: the accumulate workitems contains needed data.

## Testing

This tutorial can run the same examples as the token ledger one. One will observe that the logs are slightly different:
- transfer are noted in refine
- transfer in refine are asociated with a workpackage hash and workitem (we could have a single extrinsic root)_
- accumulate advance state root
- accumulate display workitem processed or failure (can fail if two workpackage try to advance same external client state: only one get processed, failure need to be handled properly though).

## TODO parallel resolution on accumulate : launch two transfer json leading to invalid. 
    - root building from prefixes
    - merge state with both package ids -> then refine merge step (do not accept futher refine in merge state after x time). TODO plus client side a merge state should suspend until not merge and rebase tx on merged state root.

## TODO add authorizer to system
