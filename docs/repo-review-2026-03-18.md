# Repository Review - 2026-03-18

This document captures a repository-level review of Doc-it focused on architecture, code quality, cleanup opportunities, and simplification targets.

## Scope

- backend architecture and persistence model
- frontend architecture and editor/realtime flow
- maintainability and cleanup priorities
- documentation and test coverage gaps

## Summary

The repository is in a solid MVP state structurally:

- the product spec is clear and implementation-oriented
- the backend modules are separated sensibly
- the frontend has a reasonable component split for the list, presence, and editor shell
- static diagnostics are clean across backend and frontend

The main issues are not build failures. They are architectural trust boundaries, lifecycle ownership, and a few places where complexity is already collecting too early.

## Findings

### 1. Backend source-of-truth drift risk

Severity: High

The backend is not fully enforcing the source-of-truth rule described in the MVP spec.

The spec says:

- while a document is being actively edited, Yjs state is the source of truth
- on debounce and checkpoint, the server materializes the latest Markdown and persists it

Current implementation behavior differs:

- the room accepts Markdown directly from the client during sync updates
- that client-provided Markdown is stored as the latest durable Markdown

Why this matters:

- a stale or buggy client can persist Markdown that does not match the actual CRDT state
- durable storage can drift from the collaborative state the system is supposed to trust
- this is the most important architectural risk in the repo

Recommended cleanup:

- make the server derive persisted Markdown from Yjs state only
- treat client Markdown as advisory at most, or remove it from the websocket update contract entirely
- add tests that validate persisted Markdown always matches room state after updates and reconnects

Relevant files:

- [docs/mvp-spec.md](docs/mvp-spec.md#L121)
- [docs/mvp-spec.md](docs/mvp-spec.md#L122)
- [backend/src/realtime/rooms.rs](backend/src/realtime/rooms.rs#L231)
- [backend/src/realtime/rooms.rs](backend/src/realtime/rooms.rs#L337)

### 2. Websocket initial-load failure can leave the editor hanging

Severity: High

The document page awaits websocket connection completion before it clears loading for the synced editor path.

The current connection promise resolves only after `sync_init` is received. If the socket closes before that message arrives, the client marks the connection as offline and schedules a reconnect, but the original await remains unresolved.

Why this matters:

- the first open of a document can get stuck in a loading state
- the user does not get a deterministic error state on failed initial handshake
- retry behavior exists, but the UI contract for initial load is fragile

Recommended cleanup:

- change the realtime client API so initial connection can resolve or reject explicitly
- distinguish between "connected socket" and "document sync ready"
- surface an initial connection error state to the route instead of waiting indefinitely

Relevant files:

- [frontend/src/routes/d/[id]/+page.svelte](frontend/src/routes/d/%5Bid%5D/+page.svelte#L112)
- [frontend/src/lib/realtime/client.ts](frontend/src/lib/realtime/client.ts#L77)
- [frontend/src/lib/realtime/client.ts](frontend/src/lib/realtime/client.ts#L103)
- [frontend/src/lib/realtime/client.ts](frontend/src/lib/realtime/client.ts#L158)

### 3. Title autosave has an out-of-order response race

Severity: Medium

Title rename requests are debounced on the client, but there is no protection against older requests resolving after newer ones.

Why this matters:

- the UI can revert to an earlier title after a slower network response
- this is a classic async consistency bug that becomes more visible under latency

Recommended cleanup:

- track a monotonic request token or sequence number for title updates
- discard stale rename responses on the client
- optionally move title persistence behind a small document-session controller instead of handling it directly in the route

Relevant files:

- [frontend/src/routes/d/[id]/+page.svelte](frontend/src/routes/d/%5Bid%5D/+page.svelte#L151)

### 4. Room lifecycle is incomplete in the manager

Severity: Medium

Rooms are created and cached by document ID, but empty rooms are not removed from the manager after the last peer disconnects.

Why this matters:

- a long-running server can accumulate inactive room instances for every document ever opened
- memory growth and stale room retention become more likely over time
- the spec explicitly treats active room lifecycle as a backend responsibility

Recommended cleanup:

- remove rooms from the manager map once they are flushed and empty
- make room ownership and eviction an explicit lifecycle responsibility
- add a test around room creation, last-peer leave, flush, and eviction

Relevant files:

- [docs/mvp-spec.md](docs/mvp-spec.md#L192)
- [backend/src/realtime/rooms.rs](backend/src/realtime/rooms.rs#L69)
- [backend/src/realtime/rooms.rs](backend/src/realtime/rooms.rs#L73)
- [backend/src/realtime/rooms.rs](backend/src/realtime/rooms.rs#L307)

### 5. Full-document Markdown serialization happens on every editor update

Severity: Medium

The frontend converts the entire editor document to Markdown on every update, then the realtime client includes the current Markdown alongside each Yjs update.

Why this matters:

- work scales with document size rather than just the incremental update
- CRDT updates are already present, so the extra serialization is redundant for collaboration transport
- this will become a performance cost before the rest of the stack does

Recommended cleanup:

- stop sending full Markdown with each websocket sync update
- move Markdown materialization to a lower-frequency save/checkpoint path
- benchmark conversion cost with medium and large documents before adding new editor features

Relevant files:

- [frontend/src/lib/components/EditorShell.svelte](frontend/src/lib/components/EditorShell.svelte#L73)
- [frontend/src/lib/realtime/client.ts](frontend/src/lib/realtime/client.ts#L61)
- [frontend/src/lib/realtime/client.ts](frontend/src/lib/realtime/client.ts#L71)

### 6. The document route is becoming a controller blob

Severity: Medium

The document page route currently owns too many concerns:

- session bootstrap
- document fetch
- realtime connection orchestration
- title autosave
- formatting badge state
- topbar dropdown state
- error and loading flow

Why this matters:

- the route is becoming the default place for unrelated editor-page logic
- this makes future changes harder to test and reason about
- placeholder UI is already living in the same file as core collaboration behavior

Recommended cleanup:

- extract a document-session controller or store for fetch, sync, presence, and save state
- keep the route focused on page composition and top-level load/error handling
- move placeholder topbar actions out until they are implemented, or isolate them behind a dedicated component

Relevant files:

- [frontend/src/routes/d/[id]/+page.svelte](frontend/src/routes/d/%5Bid%5D/+page.svelte#L71)
- [frontend/src/routes/d/[id]/+page.svelte](frontend/src/routes/d/%5Bid%5D/+page.svelte#L99)
- [frontend/src/routes/d/[id]/+page.svelte](frontend/src/routes/d/%5Bid%5D/+page.svelte#L228)

### 7. Documentation and testing need cleanup

Severity: Low

There are no meaningful test files in the repository at the moment, and the frontend README is still generated scaffold content.

Why this matters:

- the repo has a clear product spec, but not enough executable verification around the risky parts
- onboarding documentation does not yet describe how this application actually works

Recommended cleanup:

- replace scaffold README content with project-specific frontend notes
- add targeted tests for markdown round-trip, websocket handshake behavior, snapshot recovery, and title update ordering
- document the backend persistence model and room lifecycle in a short maintainer note

Relevant files:

- [frontend/README.md](frontend/README.md#L1)
- [frontend/README.md](frontend/README.md#L3)
- [frontend/README.md](frontend/README.md#L11)

## Prioritized Cleanup Plan

Recommended order of work:

1. Make the server the sole authority for durable Markdown.
2. Fix websocket initial connection and sync lifecycle handling.
3. Extract document-session logic from the route component.
4. Add tests around markdown conversion and realtime persistence.
5. Remove or isolate placeholder UI and replace scaffold docs.

## Verification Notes

Reviewed state at time of writing:

- backend `cargo check` previously passed in workspace context
- frontend diagnostics were clean
- no repository-wide compiler or editor errors were reported during this review pass

## Conclusion

Doc-it already has a reasonable MVP structure. The next step is not broad refactoring for its own sake. The next step is to tighten trust boundaries, complete lifecycle handling, and extract the first layer of orchestration logic before more features get added on top.