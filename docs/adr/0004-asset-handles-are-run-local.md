# 4. Asset handles are run-local and allocated at request time

Date: 2026-08-24

## Status

Accepted

## Context

A handle is the only thing a consumer holds between requesting an asset and using
it, so its semantics reach every call site that touches an asset. Two properties
have to be settled before that surface exists: when a handle's value is minted,
and how long that value means anything.

The minting point is not a free choice. ADR 0003 requires a request to return
immediately, before any background work is dispatched, so a handle has to exist at
that moment or the request path becomes blocking. Assigning at completion would mean
a request returns nothing usable and a second mechanism — a callback, a two-phase
handle — has to hand the value back later, which puts IO timing back on the path ADR
0003 exists to keep it off.

Determinism does not additionally force this. Because no simulation behaviour may
depend on a handle's value (constraint 6), a handle minted at completion would not by
itself desync anything. The argument for request time is the shape of the request
path, not the invariant.

Durability is a separate question with a real alternative. A content-addressed or
path-derived identity would survive process restarts, save files, and the network,
and would give deduplication for free. It also requires a stable identity scheme,
a manifest or hashing step, and a decision about what happens when content changes
under a stored id. There is currently no save system and no peer layer to design
that against, so any scheme chosen now would be designed against nothing.

## Decision

> A handle is allocated synchronously at request time, on the calling thread,
> before any work is dispatched.

Six constraints follow from that:

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
6. **No behaviour depends on a handle's numeric value.** Equality is permitted — a
   handle is what correlates a request with the asset it asked for, and two handles
   equal on one machine are equal on every machine whatever values were allocated.
   Excluded is anything that reads the value itself: ordering, arithmetic,
   serialization, or deriving a position in simulation state from it. Lookup by
   handle is permitted; iterating a handle-keyed map in a simulation path stays
   barred by ADR 0001 constraint 2.

Constraint 5 is a promise about what a handle is *not*, and it binds consumers: a
handle may not be written to a save file, sent over a network, or compared across
processes. Where an asset must be referred to durably, the reference is the
source, and the handle is re-derived by requesting it again.

Constraints 5 and 6 bind the handle type rather than the consumer's discipline: a
handle offers equality and lookup, and offers no surface through which its value can
be read, ordered, or written out. Both mistakes are unavailable rather than
discouraged.

Because a handle means nothing across runs, handle values are not part of what the
determinism harness in ADR 0001 compares. Two runs may allocate different values for
the same asset without having diverged.

## Consequences

**Enabled.** A request returns a usable handle immediately, on any thread, without
waiting on IO or on the pools ADR 0003 dispatches to. Constraint 6 is what keeps that
cheap: because no behaviour reads a handle's value, ids may be recycled, may differ
between runs, and may be released from a background thread without any of it reaching
simulation state. Constraint 3 survives for storage rather than for determinism —
per-type id spaces keep each type's storage densely indexed and keep one type's churn
out of another's allocator.

**Accepted costs.**

- Handle equality is not asset identity. Without deduplication (ADR 0003), two
  requests for the same source yield two unequal handles to equivalent data.
- Handle values are not stable across runs. Releases feed the same allocator that
  requests draw from, and a release can happen on frame timing or on a background
  thread, so two runs from identical inputs may allocate different values for the
  same asset. Constraint 6 makes that invisible to the simulation, but it also makes
  a handle value useless in a log, a diff, or a bug report — asset identity in
  diagnostics is the source, never the handle.
- Constraint 6 removes handles from any position needing an order. A consumer that
  wants a sorted set of assets, or a stable iteration over them, cannot get it from
  the handle and orders by source or by the owning entity instead.
- Nothing about an asset can be persisted by reference. Any system that outlives
  the process — saves, editor state, tooling — stores sources and pays the cost of
  re-requesting on load.

**Known deferral.** A stable, cross-run asset identity is not decided here. It is
required by both a save system and a peer layer, and neither exists yet. When
either lands, this ADR is the one that changes: the runtime handle can stay
exactly as specified, with a durable identity added alongside it rather than
replacing it. Until then, "handles are not durable" is a guarantee consumers may
rely on, not an omission to be worked around.
