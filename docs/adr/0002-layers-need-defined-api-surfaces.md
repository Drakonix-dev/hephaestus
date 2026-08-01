# 2. Each layer needs a well defined API surface

Date: 2026-07-31

## Status

Accepted

## Context

Hephaestus is designed to be capable of creating games of varying calibers. This
involves spanning several layers over a shifting technological landscape, including,
but not limited to:

- platform management
- rendering
- asset handling
- networking

Each of those layers is currently served by a third-party crate chosen for today's
constraints. Heavily modifying or swapping the implementation of a single layer
touches everything that depends on it, and the cost of that change scales with the
number of places the crate's types reached — worse than proportionally, if those
types appear in the public signatures of downstream consumers.

Three things make this decision live now rather than later. ADR 0001 leaves it open
by design: constraint 5 requires that any dependency touching sim state be
reproducible, clock-driven, and version-pinned, but deliberately defers *which*
dependencies qualify and how they are isolated. Physics is the next dependency to
land and the first that will touch simulation state, so whatever rule exists when it
arrives is the rule it gets built under. And the engine is pre-1.0 with no second
implementation of any layer; every seam is one module and a handful of call sites,
which is the cheapest this decision will ever be.

Pulling the other way: confining a crate to a boundary means translating at that
boundary. Any object that passes between layers is converted on each crossing, and
the conversion is pure overhead in the common case where the backend never changes.
A boundary also has to be designed before a second implementation exists to validate
it, which means designing it against a sample size of one.

## Decision

Third-party crates are not imported outside the implementing layer's backend module.

This binds **substitutable subsystem providers** — platform, rendering, physics,
audio, UI. The test for membership is structural rather than a matter of taste:

> Can this dependency be confined to a single module without changing the shape of
> our own types?

If it can, the rule binds it. A `rapier` rigid body or a `wgpu` device is the crate's
own type; it can live inside a backend module and never appear on our side of the
boundary. Where a crate is ambiguous, the tiebreaker is that a subsystem provider
owns state and does work over time.

If it cannot, it is a **vocabulary crate** and the rule does not reach it. Bytemuck is
the standing example: `Pod` and `Zeroable` are derived onto our own vertex and uniform
types, so the crate attaches to our data model by construction and there is no module
it could have been confined to. Serde and thiserror sit in the same category. The
exemption is not a concession — confinement was never structurally available.

The boundary is a **backend-agnostic API surface, not an abstraction over backends**.
Layers expose their own vocabulary types and implement them directly against one
backend. No trait or dynamic dispatch layer is introduced until a second
implementation actually exists, because replaceability comes from the type boundary,
not from polymorphism: swapping a backend means rewriting one module against an
unchanged API either way, and a trait with a single implementor only decorates that
work.

## Consequences

**Enabled.** Replacing a dependency is bounded work — one backend module plus its
call sites — rather than a codebase-wide edit. Layers can be reasoned about and
tested against their own vocabulary. ADR 0001's constraint 5 becomes enforceable,
since sim-facing dependencies are confined to modules that can be audited for
determinism.

**Accepted costs.**

- Types are converted at every layer crossing, per type, for as long as the backend
  never changes. The glam/nalgebra split will make this concrete the moment physics
  lands.
- Wrapping a tool insulates us from it, so fluency in the underlying crate develops
  slowly. When a problem has to be debugged through the boundary, or a
  backend-specific capability exploited, that unfamiliarity is the cost.
- An API surface designed against one implementation is a guess. Parts of it will be
  wrong, and capability that does not fit the shared vocabulary gets flattened out.
  The commented-out `RendererBackend` trait at `src/renderer/mod.rs:35` is a prior
  instance of exactly this being tried and withdrawn.

**Known deviation.** `diagnostics` wraps tracing and OpenTelemetry, whose value comes
from integrating with an external ecosystem rather than from being substitutable.
It is recorded here as unresolved: either it is brought under the rule, or the rule's
scope narrows to layers that own simulation-facing or platform-facing state. Until
that is settled, it should not be cited as precedent in either direction.
