## Moving data out of JAM database

JAM allow storing data during its accumulation phase, yet this is costy and non scalable, this accumulation centric design is very similar to the way ethereum did work at launch: a single state shared by all peers.

Here we will move as much as possible of this accumulation process into refinement. To allow this refinement will run over a partial state that we pass into its workitem, we call this partial state the witness in this tutorial as it is the witness of a given state transition, is also often refered as a state proof (eg in polkadot).

The users still need to access the whole state (all tokens and balance), here jam only see some partial state to validate some state transition, so the users or client will need to store this data. This is similar to a parachain or a sidechain.

To sumup, we got clients that store and share a common state, and send proofs of state transition to jam. Jam refinement will validate those proofs, and jam accumulation will only store and update this common state merkle root.

## Overview

This tutorial extends part 2 with a focus on:
- designing rollup/sidechain like state: accounts are not stored anymore on jam state.
- discuss cost of such design.
- use a minimal external client state implementation only for educational purpose.

This tutorial will not attempt to:
- be secure, we keep skipping signature checks.
- be optimal, we use a very simple bounded, unoptimal, merkle state and proofs. For real use a proper third party implementation of state and state storage should be use (eg polkadot sdk).
- implement state distribution: each client should synch upon the last state root finalized in jam state. A disconnected client will lose ability to synch state if work items got pruned (TODO refer to gp data lake retention duration). Here we will not implement such client but just launch client commands from a single state persistence.
- define proper role for distribution: every client are just validators with direct access the jam datalake and work items, on a real implementation, distribution strategy must fit the usecase.
- external client must handle fail or success accumulate processing, here we assume it will always succeed, a failure will put client in an invalid state. TODO should we backup old persistence files to rollback (sounds simple enough).

We thus remain at service level.

## Single workitem state transition

This design simply put a batch of operations in a single work item, processed in a single refinement call, such that refinement can directly pass the new and olt state root to accumulation which only update this root if old root matches.

JAM persistence is therefore only:
- a key value for the current state root
- work item in the datalake.



### Testing

This tutorial can run the same examples as the token ledger one. One will observe that the logs are slightly different:
- transfer are noted in refine
- transfer in refine are asociated with a workpackage hash and workitem (we could have a single extrinsic root)_
- accumulate advance state root
- accumulate display workitem processed or failure (can fail if two workpackage try to advance same external client state: only one get processed, failure need to be handled properly though).

### Prepare a payload for refinement

```cargo run --features=std -- ./example_payloads/op_mint.json refinement_payload```

This run locally the external client operations, and write a payload for refinement containing both input operations and the state witness to be able to run.
The json file shall contain all operation to run for a single slot. `op_mint.json` for instance will involves:  three balance value included of each minted token, and the tokens (as documented in code sample the state is simply includding all tokens everytime).

### Example code

can be found in token-ledger-external-state :
- external_client module is the dummy client external state implementation. Description of this state is out of the scope of this tutorial, but code has been written with the intention of being simple and easy to read (serializing deserializing all at once from file, simple binary tree for balances, single out of tree value to store all tokens ids).
- main.rs: produce payload for refinement:  just open external client state from last serializing, process state transition from operations in input json and a jam encoding binary payload in a file 
- lib.rs: the actual service, split into accumalution and refinement modules.

### Run on jam

Simply use jst as in previous tutorial (use the submit-file command for work item with the produced payload refinement_payload).

TODO document what happens with jamt item command (workpackage produce, data in lake...).

TODO next content is just a draft and unimplemented.

## Concurrent workitems

Multiple workitems with a sequence of operation can be processed in parallel.

Accumulation will store:
- current state root

Multiple refinement will validate signature, witness and run state transition.
All updated balances and additional tokens will be sent to accumulation.


Accumulation will look for updates conflict between the multiple parallel refinements:
- balance update on a same key should not go bellow 0
- token should not be minted twice.

On conflict we either:
- drop all
- invalidate minted token and transaction, while keeping others. TODO here would be good to put a log of those in datalake (for clients): on paper clients should just run all process, but it is awkward TODO could also write all update so clients synch on them

If multiple workitem are sent, accumulation will build the new root by consolidating all witness.

This design is pretty bad, as accumulation ends up doing a lot of work, this is only didactic.


## Concurrent workitems with delay

We do a similar design as the previous one, but accumulation will also maintain a pending conflict preimage hash and send back task to refinement.

So we will have two refinement process:
- multiple operation import from external clients with transaction at slot N
- conflict resolution of transactions at slot N-1

And accumulation will:
- put data for conflict resolution of slot N in datalake
- update state root for slot N-1

Client when creating operations, will process witness (state proof) over a resolved state root at slot N (client will run ahed the conflict resolution refinement process).

## TODO parallel resolution on accumulate : launch two transfer json leading to invalid. 
    - root building from prefixes
    - merge state with both package ids -> then refine merge step (do not accept futher refine in merge state after x time). TODO plus client side a merge state should suspend until not merge and rebase tx on merged state root.

## TODO add authorizer to system
