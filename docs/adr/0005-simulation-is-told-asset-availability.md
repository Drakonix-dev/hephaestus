# 5. The simulation is told when assets are available, never asked

Date: 2026-08-24

## Status

Accepted

## Context

ADR 0003 puts asset loading behind IO and background threads. ADR 0004 keeps
handle *values* out of that timing, but says nothing about *availability* —
whether the asset behind a handle can be used yet. Availability is genuinely
IO-dependent: the same request on two machines completes at different times, in a
different order relative to other requests, and sometimes fails on one machine and
succeeds on the other.

That makes availability a hazard for ADR 0001. If simulation logic can ask whether
an asset is ready, it branches on disk latency, and two machines replaying
identical inputs produce different state. The polling shape — hold a reference,
test whether it has arrived, act when it flips — is the most natural way to write
an asset system and is precisely what cannot be allowed near the simulation.

Whether an asset is simulation-facing is not a property of its type. The same mesh
can be rendered and used as a collision source; the same heightmap can be sampled
by a shader and drive terrain. It is a property of where the asset is used, which
means the distinction has to be expressed when the asset is requested or not at
all.

Pulling the other way: the alternatives to polling all involve the simulation
waiting on something. Determinism itself does not require loading to be fast — two
machines that compute identical state have not diverged because one took longer —
but a simulation that stops is a game that stops, and the player pays for that in
a way no invariant compensates for.

## Decision

> The simulation never observes asset availability. It is told, at a tick every
> machine agrees on.

This binds assets reached from **simulation** code. It does not bind presentation,
for which the timing of a delivery carries no determinism requirement.

Seven constraints enforce it:

1. **Availability is not queryable.** A handle carries no state and exposes no way
   to ask whether its asset has arrived. There is no such facility to reach for,
   from the simulation or anywhere else — availability exists only as something
   delivered.
2. **Availability is delivered.** The engine pushes an asset's state, ready or
   failed, identified by handle, and the simulation consumes it as ordinary input
   under ADR 0001 constraint 7. A handle is the key correlating a request with its
   delivery. An asset that is never pushed is simply never available, and there is
   nothing to inspect in the meantime.
3. **Two mechanisms exist for requesting an asset.** Both return a handle, and
   both are delivered under constraint 2. They differ in exactly one respect:
   whether an outstanding request suspends the simulation until the asset arrives.
   Simulation code may use either — suspending is a property of the request, not of
   the asset or of the handle.
4. **A suspending request halts tick progression, and the barrier is the
   engine's.** The simulation does not advance past the current tick until every
   outstanding suspending request has been delivered. Both machines stall at the
   *same* tick and resume at the *same* tick, so however long each waited, neither
   the tick counter nor anything derived from it can diverge. This is implemented
   once, in the engine: a game-side loading mechanism would put determinism-critical
   logic in every game built on the engine, where an error surfaces as a desync
   months later rather than a test failure now.
5. **Deliveries within a tick arrive in a reproducible order.** Where several
   assets become available in the same tick, they are delivered in an order derived
   from data both machines already agree on. The specific ordering rule is an
   implementation detail and is deliberately not fixed here; what consumers may
   rely on is that an order exists and is reproducible.
6. **What reaches simulation state must be deterministic in content and
   serializable.** ADR 0001 verifies itself by comparing serialized `World` state,
   so anything delivered into the World has to survive that comparison. Driver-owned
   objects do not — their bytes depend on the GPU, the driver, and allocation order
   inside it, and two machines would compare as divergent without having diverged.
   Such built forms are engine-internal and are never delivered; the simulation
   receives the handle and the asset's state, and the built form stays behind the
   engine boundary.
7. **Simulation and rendering are decoupled.** Rendering continues while the
   simulation is halted under constraint 4. Without this the barrier presents to
   the player as a hung process.

Failure is delivered under constraint 2 like any other state, and what the game
does about it is not constrained here. The determinism requirement is satisfied by
the delivery mechanism rather than by the response: each instance remains
replayable from its own input stream whatever the game chooses to do.

## Consequences

**Enabled.** The simulation uses assets without ever racing a load. Constraint 1
makes this structural rather than disciplinary: the polling mistake is not
discouraged, it is unavailable, because there is no surface through which to ask.
That is also why a single handle type suffices — there is nothing pollable for a
second type to protect. A recorded session replays without touching the disk,
which makes replay independent of the storage it was recorded on, and makes the
determinism harness in ADR 0001 cheaper to run since it need not reproduce IO
timing.

**Accepted costs.**

- A suspending request mid-game freezes the entire simulation until the disk
  answers. This is a policy and not merely a mechanism: in practice
  simulation-facing assets can only be requested where a full stop is acceptable,
  which means scene boundaries. Streaming simulation-facing data during play is
  not available.
- Delivered availability is part of "the same ordered sequence of inputs" in ADR
  0001's invariant. Two *fresh* runs on machines with different storage produce
  different input streams, and the invariant promises nothing about those matching.
  Each run is internally deterministic; they are not the same run.
- Presentation can issue a suspending request and halt the simulation. The footgun
  is bounded — both machines still reach the next tick with identical state, so the
  result is a stall rather than a desync — but nothing prevents it.
- Every consumer of an asset must be written to do nothing until delivery arrives,
  including consumers that would rather ask once and move on. Removing the question
  removes the shortcut along with the hazard.
- Constraint 7 is a hard requirement on the frame loop, not a preference. Any
  later change that couples simulation and render ticks breaks the barrier.

**Known deferrals.** Availability and failure are both **machine-local facts**.
With a single peer that is sufficient, since the only machine that has to agree is
the one running. With several it is not: a peer cannot announce readiness on
another peer's behalf, and a peer that fails a load while others succeed holds a
different world state — not a violation of ADR 0001, whose invariant is per
instance, but the precise thing lockstep exists to prevent. Resolving either
requires a networking model, which ADR 0001 deliberately leaves open and which
does not exist yet. Neither should be cited as settled once a peer layer lands.
