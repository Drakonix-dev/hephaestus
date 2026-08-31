# 3. Assets need to be processed asynchronously

Date: 2026-08-24

## Status

Accepted

## Context

Various aspects of the engine require assets, including rendering (meshes,
shaders, materials) and audio. Some of these must be built against renderer state
that is main-thread-bound. The game is not guaranteed to request or manage assets
from the main thread, so assets need to be capable of loading asynchronously.

This decision is live now because the asset system has not landed yet, and every
downstream consumer of assets is written against whatever shape it takes.

Pulling the other way: moving asset work off the main thread introduces
synchronization, and synchronization bugs are timing-dependent. They do not
reproduce on demand, they do not fail the same way twice, and they surface under
load rather than under test. A synchronous loader has none of that exposure, at
the cost of stalling the main thread for the duration of every read.

## Decision

Assets are loaded in three distinct stages:

1. **Request** — a load is asked for, and returns immediately.
2. **Parse and prepare** — bytes are read and turned into an intermediate form.
3. **Build** — the intermediate form becomes the usable asset.

Requests are safe to emit from anywhere. Stage 2 occurs across two pools of
background threads:

1. IO-blocking threads
2. Asset worker threads

Assets are read from disk on the first pool, parking those threads until the IO
completes. Results are picked up by the second pool, which finishes parsing and
preparing. Stage 3 occurs on the main thread, before the frame is rendered.

Build is kept on the main thread because some built forms are GPU-owned and are
created against renderer state that cannot leave it. Splitting stage 3 by whether
a given asset touches the GPU would mean two build paths and two sets of ordering
rules for one stage; a single path is worth the constraint.

**Priority orders background work and nothing else.** It selects which parse queue
a job enters, so a higher-priority asset reaches stage 3 sooner in wall-clock
terms. It carries no meaning for the simulation: whether an asset blocks the
simulation is a property of how it was requested, not of its priority.
The enum is `#[non_exhaustive]` and ships with two levels, so a level can be added
when a consumer actually needs the distinction rather than in anticipation of one.

**Hot reload sits outside the determinism invariant.** Re-entering stage 2 for an
already-loaded asset replaces its content mid-run, at a moment decided by a file
changing on disk. That is not reproducible and is not meant to be. Reload is a
development-loop facility and is unavailable wherever a run has to be reproducible —
while recording, while replaying, and in any networked session. ADR 0001 is not
weakened by it, because reload cannot occur under the conditions the invariant
covers.

Third-party decoders are subsystem providers under ADR 0002 and stay confined to
the asset layer's backend module. This ADR decides how asset work is scheduled and
nothing else — what a handle means, and what the pipeline owes the simulation, are
separate decisions.

## Consequences

**Enabled.** The main thread never stalls on a read. Streaming and background
loading are available rather than being a later redesign, and hot reload becomes
re-entry into stage 2 rather than a separate mechanism. Intermediate forms are
`Send` and cross freely between pools; built forms never leave the main thread and
need be neither `Send` nor `Sync`.

**Accepted costs.**

- An asset is usable no earlier than the frame after its background work
  completes. Nothing in this pipeline makes a load fast; it makes it not block.
- Stage 3 competes with frame time on the main thread, and there is no per-frame
  build budget. A large batch completing together is a frame spike.
- Identical requests are not deduplicated. Two requests for the same source
  produce two jobs, two slots, and two handles.
- Threading bugs in stages 1 and 2 are timing-dependent and will not reproduce
  reliably. This is the standing cost of the decision, not a defect to be fixed.
- The development loop and the reproducible modes are not the same environment. A
  bug that only appears after a reload has to be reproduced from a fresh load before
  it can be recorded, and iteration on a networked or replaying session goes back to
  a restart.

**Follow-on work.** A per-frame build budget, once stage 3 is doing enough work to
be measurable. Request deduplication, if profiling shows duplicate loads are
common in practice rather than in theory.
