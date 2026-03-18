# Yjs-Authoritative Storage Plan

This document defines the target storage model for Doc-it going forward.

## Goal

Adopt this model without backward compatibility constraints:

- do not store Markdown in the database
- keep Yjs snapshot in the database
- treat Yjs snapshot as the authoritative editor state
- generate Markdown on demand from the authoritative state when needed
- no backward compatibility support for older storage behavior

## Desired End State

Each document has one persisted content representation and one on-demand export representation:

1. authoritative collaborative state
2. derived human-readable content generated when needed

Authoritative state:

- stored as Yjs snapshot bytes
- used for editor boot and collaborative recovery
- used as the only trusted source for live document state

Derived state:

- generated as Markdown on demand from authoritative state
- not stored in the database
- used for export, rendering, and non-editor consumers when needed

## Core Invariants

The system should enforce these invariants:

1. A document is valid only if it has an authoritative Yjs snapshot.
2. The editor always boots from Yjs state, never from Markdown.
3. Markdown is never used as the source of truth for collaboration.
4. The backend is responsible for generating Markdown on demand when required.
5. Clients send Yjs updates and presence data only.
6. The only persisted content state is the latest authoritative Yjs snapshot.

## Why This Model

This model matches how the product actually works:

- the editor is collaborative and CRDT-based
- Yjs already represents the real editing state
- Markdown is useful as an export and readable derived artifact
- Markdown is not the native collaboration format

Benefits:

- cleaner source-of-truth rules
- exact restore of collaborative state
- no duplicated content state in persistence
- simpler room bootstrap behavior
- clearer API boundaries between client and server

## Schema Direction

Two persistence areas remain:

### documents

Keep:

- `id`
- `title`
- `created_at`
- `updated_at`

Meaning:

- the document row stores metadata only

### document_snapshots

Preferred MVP simplification:

- store only the latest snapshot per document

Recommended shape:

```sql
CREATE TABLE document_snapshots (
  document_id TEXT PRIMARY KEY,
  yjs_snapshot BLOB NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);
```

Rationale:

- version history is not yet a user-facing feature
- one-row-per-document is simpler operationally
- retention and history can be added later if needed

If future history becomes important, versioned snapshots can be reintroduced deliberately.

## Creation Flow

New documents must be born with authoritative state.

Required flow:

1. create an empty Yjs document on the backend
2. encode the full Yjs state as snapshot bytes
3. insert the document row and snapshot row in one transaction

Result:

- every document is immediately valid
- no bootstrap-from-Markdown fallback is needed
- no first-client ownership logic is needed

## Loading Flow

Editor load must follow this rule:

1. HTTP may load metadata such as title
2. websocket sync loads authoritative Yjs state
3. editor initializes from Yjs state only

Markdown is not used to initialize the editor and does not need to exist at document load time.

## Persistence Flow

The backend owns the durable save boundary.

On checkpoint or flush:

1. collect full authoritative Yjs state from the room
2. persist the latest snapshot
3. update timestamps

Markdown generation happens outside the save path, only when an API or export flow needs it.

## Websocket Contract Changes

The realtime protocol should be simplified to reflect the new trust boundary.

### Client to server

Allowed message types:

- `join`
- `sync_update`
- `presence_update`
- `heartbeat`

`sync_update` should contain:

- `clientId`
- `update`

Remove:

- client-supplied `markdown`

### Server to client

Allowed message types:

- `joined`
- `sync_init`
- `sync_update`
- `presence_snapshot`

`sync_init` should contain authoritative Yjs state only.

Remove:

- `bootstrapMarkdown`

## Backend Changes

### Room lifecycle

Remove special bootstrap behavior entirely.

Delete these concepts:

- `needs_bootstrap`
- `bootstrap_owner`
- any empty-update bootstrap semantics tied to Markdown seeding

Rooms should always load from a snapshot.

### Persistence boundary

The room flush path becomes the single durable write boundary.

Responsibilities:

- obtain current Yjs state
- persist the authoritative snapshot
- update metadata timestamps atomically with snapshot persistence

The room must never trust Markdown supplied by clients.

### Document creation

The document creation path must always create both:

- a `documents` row
- an initial `document_snapshots` row

No document should exist without a corresponding authoritative snapshot.

## Frontend Changes

### Editor initialization

Remove any logic that seeds the editor from HTTP Markdown.

The editor should:

- wait for authoritative sync state
- initialize from Yjs only
- treat Markdown as an on-demand export or rendering format only

### Route simplification

Remove frontend concepts related to Markdown bootstrap:

- `bootstrapMarkdown`
- conditional initialization from `initialMarkdown`
- first-client bootstrap logic

This should simplify the document route and editor shell considerably.

### Client protocol

The realtime client should stop sending Markdown alongside Yjs updates.

It should only send:

- Yjs updates
- presence updates
- heartbeats

## Migration Approach

Because there is no backward compatibility requirement, the migration should be a hard cutover.

Recommended approach:

1. replace the schema with the new model
2. reset local development databases
3. change backend creation/load/flush behavior
4. change websocket payloads
5. simplify frontend initialization
6. remove dead code paths

Do not preserve mixed old/new behavior.

## Testing Requirements

Minimum backend coverage:

1. creating a document also creates an initial snapshot
2. opening a room always loads from snapshot
3. Yjs updates mutate authoritative room state
4. flush persists snapshot reliably and updates document metadata
5. reopening restores exact editor state from snapshot
6. Markdown generation from a stored snapshot works correctly when requested

Minimum frontend or integration coverage:

1. opening a document no longer depends on Markdown bootstrap
2. reconnect restores state from authoritative sync
3. client sync payloads do not include Markdown
4. multiple clients converge using Yjs-only collaboration traffic
5. export or render flows can request Markdown generation on demand

## Recommended Implementation Order

1. update the spec and internal invariants
2. simplify the snapshot schema to latest-only storage
3. make document creation always create an initial snapshot
4. remove backend bootstrap logic
5. remove Markdown from websocket sync payloads
6. persist snapshots as the only durable content state
7. simplify frontend editor initialization to Yjs-only
8. add on-demand backend Markdown generation for export or rendering use cases
9. delete dead compatibility code
10. add tests around the new invariants

## Final Rule

The intended architectural rule is simple:

- the editor never boots from Markdown
- Markdown never drives collaboration state
- Yjs is the system-facing truth
- Markdown is an on-demand derived format, not persisted state

That rule should remain stable as the application grows.