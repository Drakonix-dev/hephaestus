# 4. Asset handles are run-local and allocated at request time

Date: 2026-08-24

## Status

Accepted

## Context

A handle is the only thing a consumer holds between requesting an asset and using
it, so its semantics reach every call site that touches an asset. Two properties
have to be settled before that surface exists: when a handle's value is minted,
and how long that value means anything.

The minting point is not a free choice. ADR 0003 puts stages 2 and 3 behind IO and
thread scheduling, both of which vary by machine. If a handle's value were
assigned when a load *completed*, then disk latency and pool contention would
determine handle values, and any simulation state derived from a handle would
differ between machines running identical inputs — the failure ADR 0001 exists to
prevent. Assigning at request time avoids this by construction, because a request
is an ordinary call made in ordinary program order.

Durability is a separate question with a real alternative. A content-addressed or
path-derived identity would survive process restarts, save files, and the network,
and would give deduplication for free. It also requires a stable identity scheme,
a manifest or hashing step, and a decision about what happens when content changes
under a stored id. There is currently no save system and no peer layer to design
that against, so any scheme chosen now would be designed against nothing.

## Decision

> A handle is allocated synchronously at request time, on the calling thread,
> before any work is dispatched.

Five constraints follow from that:

1. **Allocation is a request-path operation.** No later stage may allocate a
   handle value. Stages 2 and 3 receive a handle that already exists.
2. **No stage mutates a handle value.** A handle's value is fixed from the moment
   it is returned until the asset is released.
3. **Identity is per asset type.** Each asset type owns its own id space and free
   list, so activity against one type cannot perturb the handle values of another.
4. **Reuse is generation-guarded.** Ids are recycled; a recycled id carries an
   incremented generation, so a stale handle resolves to "not found" rather than
   silently addressing a different asset.
5. **Handles are run-local.** A handle's value is a function of the ordered
   sequence of requests and releases in one run of one binary. It means nothing
   outside that run.

Constraint 5 is a promise about what a handle is *not*, and it binds consumers: a
handle may not be written to a save file, sent over a network, or compared across
processes. Where an asset must be referred to durably, the reference is the
source, and the handle is re-derived by requesting it again.

## Consequences

**Enabled.** Handle values depend only on program order, so requesting an asset is
deterministic regardless of how long the load takes or which thread finishes
first. Constraint 3 keeps that blast radius small: presentation churning through
texture handles cannot shift the ids of a simulation-facing asset type, which is
what lets ADR 0005 bind only the types that need it rather than the whole
subsystem.

**Accepted costs.**

- Handle equality is not asset identity. Without deduplication (ADR 0003), two
  requests for the same source yield two unequal handles to equivalent data.
- Releases feed the same allocator that requests draw from, so a handle's value
  depends on release ordering as much as on request ordering. Where the simulation
  holds a handle, that ordering is subject to the same requirement as any other
  simulation state; where it does not, a release from a background thread is legal.
- Nothing about an asset can be persisted by reference. Any system that outlives
  the process — saves, editor state, tooling — stores sources and pays the cost of
  re-requesting on load.

**Known deferral.** A stable, cross-run asset identity is not decided here. It is
required by both a save system and a peer layer, and neither exists yet. When
either lands, this ADR is the one that changes: the runtime handle can stay
exactly as specified, with a durable identity added alongside it rather than
replacing it. Until then, "handles are not durable" is a guarantee consumers may
rely on, not an omission to be worked around.
