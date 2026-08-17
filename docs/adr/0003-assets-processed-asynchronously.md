# 3. Assets need to be processed asynchronously

Date: 2026-08-01

## Status

Accepted

## Context

Various aspects of the engine require assets, including rendering (meshes,
shaders, materials, etc.) and audio. Some of these assets must be built on the
main thread, like the gpu owned objects. The game is not guaranteed to request
or manage all assets on the main thread, or in other words, assets need to be
capable of loading asynchronously.

To comply with ADR 0001, assets must be able to be referenced and loaded
deterministically. This decision is live now because the asset
system has not landed yet, and changing this later affects every downstream
consumer of assets.

However, processing assets asynchronously brings in the added complexity of
threading and data synchronization.

## Decision

Assets will be loaded in 3 distinct stages:

1. Request to load asset
2. Parsing and preparing asset for build
3. Building asset

Requests are safe to emit from anywhere and return stable deterministic handles
to the requested assets. Step 2 occurs asynchronously via thread pool.
Requesting to load an asset will return a handle, or reference, to the
requested asset that can be used to check status and/or retrieve the loaded
asset. The handle will be returned while the asset is still being loaded,
meaning the handle can reference a pending, or unbuilt, asset. Building of the
actual asset must occur on the main thread.

Parsing and preparing assets will occur via 2 pools of background threads:

1. IO blocking threads
2. Asset worker threads

Assets will be read from disk on the first thread pool, parking those threads
until the IO operations are complete. At which point they'll then be picked up
by the second thread pool to finish any remaining parsing or preparing. Final
build of the asset will occur on the main thread, where the built assets
themselves will reside (avoiding any send/sync issues of asset data).

## Consequences

