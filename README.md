### Determinism invariant

**Given the same starting world state and the same ordered sequence of inputs, the simulation produces byte-identical world state on every run, on every machine.**

This binds the **simulation** (the ECS `World`, the fixed-timestep update, physics, game logic). It deliberately does **not** bind **presentation** (rendering, interpolation, animation, particles, audio, UI) — nothing presentation does feeds back into sim state.

It is load-bearing: networking (lockstep/rollback replicate inputs and re-simulate), save/load (a snapshot is only valid if replaying it reproduces the same state), and reproducible debugging all depend on it.

**Constraints that enforce it:**
1. **Fixed timestep only** — sim `update` gets a fixed dt; no wall-clock (`Instant::now()`, frame delta) reaches game logic.
2. **Deterministic iteration order** — no `HashMap`/`HashSet` iteration in any path that mutates sim state (Rust randomizes it); iterate by stable entity index.
3. **Seeded RNG in the World** — no `rand::random()` / thread-local RNG in logic; randomness comes from a seeded generator stored as a World resource.
4. **Explicit, ordered system execution** — single-threaded ordered schedule; no unordered parallelism over sim state.
5. **Deterministic external subsystems** — rapier stepped with `enhanced-determinism`, version pinned, on the fixed clock; any crate touching sim state meets this bar or stays presentation-side.
6. **No hidden global mutable state** — all sim state is reachable from the `World`; nothing consequential in statics/thread-locals/ambient singletons.
7. **Inputs are a captured, ordered stream** — input is sampled at the sim boundary into a per-tick snapshot; that stream is the replayable record.

**The test:** log the input stream, feed it to a fresh instance, get an identical `World`. If two runs diverge from the same seed + inputs, one of the seven constraints was broken (usually #2 or #1). This is automated by the determinism harness (W–Z).
