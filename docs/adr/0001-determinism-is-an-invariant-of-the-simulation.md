# 1. Determinism is an invariant of the simulation

Date: 2026-07-10

## Status

Accepted

## Context

Hephaestus is a general-purpose engine, not a single game. Several capabilities we
intend to support are only possible if the simulation can be replayed:

- Lockstep and rollback netcode replicate *inputs* and re-simulate; if two peers
  simulate the same inputs differently, they desync.
- A save file is only valid if replaying from it reproduces the same state.
- A bug report is only actionable if the recorded session reproduces the bug.

Committing to determinism after the fact is not practical. Non-determinism enters
through ordinary, individually reasonable choices — a `HashMap` iterated in a system,
a `rand::random()` call, wall-clock `dt` passed into game logic — and each one is
cheap to avoid up front and expensive to find later. Deferring the decision would
also force a networking model to be chosen early, since only some models tolerate
divergence.

At the same time, presentation work — interpolation, particles, animation, UI — has
none of these requirements, and constraining it would cost real quality and
performance for no benefit.

## Decision

Determinism is an invariant of the engine, not a feature of a subsystem:

> Given the same starting world state and the same ordered sequence of inputs, the
> simulation produces byte-identical world state on every run, on every machine.

This binds the **simulation**: the ECS `World`, the fixed-timestep update, physics,
and game logic. It deliberately does **not** bind **presentation** — rendering,
interpolation, animation, particles, audio, UI — on the condition that nothing
presentation does feeds back into sim state.

Seven constraints enforce it:

1. **Fixed timestep only** — sim `update` gets a fixed dt; no wall-clock
   (`Instant::now()`, frame delta) reaches game logic.
2. **Deterministic iteration order** — no `HashMap`/`HashSet` iteration in any path
   that mutates sim state (Rust randomizes it); iterate by stable entity index.
3. **Seeded RNG in the World** — no `rand::random()` / thread-local RNG in logic;
   randomness comes from a seeded generator stored as a World resource.
4. **Explicit, ordered system execution** — single-threaded ordered schedule; no
   unordered parallelism over sim state.
5. **Deterministic external subsystems** — any dependency that touches sim state must
   itself be reproducible across machines, be driven by the fixed clock rather than
   frame time, and be version-pinned so its numerical behaviour cannot shift under
   us. A dependency that cannot meet this bar stays presentation-side.
6. **No hidden global mutable state** — all sim state is reachable from the `World`;
   nothing consequential in statics/thread-locals/ambient singletons.
7. **Inputs are a captured, ordered stream** — input is sampled at the sim boundary
   into a per-tick snapshot; that stream is the replayable record.

The invariant is verified, not assumed: log the input stream, feed it to a fresh
instance, and compare the resulting `World`. If two runs diverge from the same seed
and inputs, one of the seven constraints was broken (usually #2 or #1).

## Consequences

**Enabled.** All networking models stay open — the engine ships primitives and a game
picks lockstep, authoritative, or rollback. Snapshots become trustworthy, so save/load
and replay-based debugging follow from the same machinery.

**Accepted costs.**

- Sim-side parallelism is restricted to deterministic forms: parallelism over a
  deterministically-computed partition, with cross-partition combination in fixed
  order. Scheduling-inferred system parallelism is excluded, since its determinism
  degrades silently as systems are added. In practice the schedule is single-threaded
  until profiling identifies a stage that justifies the complexity. Presentation is
  unaffected.
- Constraint 5 narrows the field of usable dependencies for anything sim-facing, and
  forces pinning where we would otherwise track upstream freely. Which crates clear
  that bar, and how they are isolated, is a separate decision.
- The two-clock split has to be respected everywhere: any code that reaches for frame
  time inside the sim is a bug, not a shortcut.

**Follow-on work.** World-state serialization lands early, since it is the comparison
mechanism the test depends on. A determinism harness automates the replay test in CI
so violations fail the build rather than surfacing as a desync months later.
