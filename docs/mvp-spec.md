# Realtime Document Editor MVP Spec

## Status

This document is the execution-ready specification for the MVP.

It supersedes the exploratory note in `docs/mvp-brainstorm.md` for implementation decisions.

## 1. Product Summary

Build a minimal realtime collaborative document editor with these constraints:

- frontend: SvelteKit + TypeScript
- backend: Rust + Axum
- database: SQLite
- no authentication
- anyone with the document link can edit
- documents are saved as Markdown
- users edit through a clean rich-text interface rather than raw Markdown

The product should feel closer to a lightweight Google Docs page editor than a full Notion workspace.

## 2. MVP Scope

### Included

- create a document
- list documents
- rename a document
- open a document by URL
- rich-text editing
- realtime collaboration for multiple anonymous users
- collaborator cursor and presence display
- autosave
- Markdown persistence
- document recovery on reopen

### Excluded

- authentication
- permissions
- folders or workspaces
- comments
- version history UI
- media uploads
- tables
- offline sync
- mobile-first editing polish
- imports and exports beyond persisted Markdown

## 3. Product Decisions

### Editing Model

- page-oriented editor
- restrained block model
- no advanced Notion-style block nesting

### Access Model

- anyone with the document link can edit
- root document list is available in-app
- no read-only mode in MVP

### User Identity

- prompt once for display name on first visit
- store display name in `localStorage`
- assign a deterministic local color from a small palette

### Supported Content Types

The editor schema must support only content that round-trips to Markdown reliably:

- paragraph
- heading levels 1 to 3
- bullet list
- ordered list
- task list
- blockquote
- code block
- bold
- italic
- strikethrough
- inline code
- link
- horizontal rule

Anything outside this set is out of scope for MVP.

## 4. Technical Architecture

### Frontend

- SvelteKit application
- Tiptap editor built on ProseMirror
- Yjs for collaborative state
- WebSocket client for sync and presence
- fetch-based API client for metadata operations

### Backend

- Axum HTTP server
- Axum WebSocket endpoint per document
- in-memory room manager for active documents
- SQLite persistence layer
- periodic snapshot and Markdown materialization workers

### Data Model Strategy

Two representations exist for each document:

- live collaborative state: Yjs update stream and latest snapshot
- durable saved content: Markdown

Markdown is the durable format for storage and export.
Yjs state is the active collaboration format.

### Source of Truth Rule

- while a document is being actively edited, Yjs state is the source of truth
- on debounce and checkpoint, server materializes the latest Markdown and persists it
- when no snapshot exists, the document boots from Markdown-derived editor content

## 5. UX Requirements

### Main Screens

#### Document List

- route: `/`
- shows document titles ordered by most recently updated
- primary action: create document
- clicking a document navigates to `/d/:id`

#### Document Editor

- route: `/d/:id`
- top bar contains:
  - editable title
  - connection state
  - save state
  - active collaborators
- left sidebar remains minimal and can be hidden on smaller screens
- editor canvas is centered with generous whitespace

### Presence UX

- each active collaborator shows display name and color
- remote cursors and selection highlights are shown when supported by the editor integration
- presence disappears when connection closes or heartbeat expires

### Save UX

- autosave only
- show one of these states:
  - `Connecting`
  - `Connected`
  - `Saving`
  - `Saved`
  - `Offline`

No manual save button is needed in MVP.

## 6. Frontend Implementation Constraints

### Editor Rules

- use Tiptap extensions only for the approved Markdown-safe schema
- disable unsupported block types and menus
- paste handling should normalize unsupported formatting into supported nodes or plain text

### State Rules

- document metadata comes from HTTP API
- collaborative content comes from WebSocket and Yjs
- local display name and local color are stored client-side only

### Initial Load Flow

1. load document metadata from HTTP API
2. render loading state
3. connect to WebSocket room
4. receive initial sync payload
5. mount editor with synced document state

## 7. Backend Responsibilities

The backend is responsible for:

- document CRUD for metadata and persisted content
- active room lifecycle per document
- relaying Yjs updates between connected clients
- presence tracking and broadcasting
- snapshot persistence
- Markdown materialization and storage

The backend is not responsible for implementing a custom CRDT algorithm.

## 8. SQLite Schema

### Table: `documents`

Purpose: durable document metadata and latest materialized Markdown.

```sql
CREATE TABLE documents (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  markdown TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_documents_updated_at
ON documents(updated_at DESC);
```

Notes:

- `id` should be a URL-safe unique identifier such as UUID v7 or ULID
- timestamps are stored as ISO-8601 UTC strings for simplicity in SQLite

### Table: `document_snapshots`

Purpose: latest persisted collaboration state checkpoints.

```sql
CREATE TABLE document_snapshots (
  document_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  yjs_snapshot BLOB NOT NULL,
  markdown TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (document_id, version),
  FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX idx_document_snapshots_created_at
ON document_snapshots(document_id, created_at DESC);
```

Notes:

- keep at least the latest snapshot for each document
- older snapshots may be pruned later; snapshot retention is not a user-visible feature in MVP

### Optional Table: `document_events`

Purpose: append-only debug or replay log of collaboration updates.

This table is optional for MVP. Do not build the first iteration around it.

```sql
CREATE TABLE document_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  document_id TEXT NOT NULL,
  client_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  payload BLOB NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX idx_document_events_document_id
ON document_events(document_id, created_at ASC);
```

Recommendation:

- skip this table for v1 unless debugging or replay becomes necessary

## 9. Persistence Rules

### Document Creation

- create row in `documents`
- initialize `markdown` to empty string
- no snapshot required until first checkpoint

### Autosave

- client changes sync continuously through WebSocket
- server debounces persistence per document
- debounce target: 2 seconds after the latest update burst

### Snapshotting

- persist a full snapshot no more than once every 15 seconds per active document
- also persist a snapshot when the last collaborator disconnects

### Markdown Materialization

- regenerate Markdown at each snapshot
- update `documents.markdown`
- update `documents.updated_at`

This keeps reads simple while avoiding full database writes on every keystroke.

## 10. Axum HTTP API

Base path: `/api`

### `GET /api/documents`

Returns the list of documents ordered by `updated_at DESC`.

Response:

```json
{
  "documents": [
    {
      "id": "01HR...",
      "title": "Project Notes",
      "updatedAt": "2026-03-18T12:00:00Z",
      "createdAt": "2026-03-18T10:00:00Z"
    }
  ]
}
```

### `POST /api/documents`

Creates a new blank document.

Request:

```json
{
  "title": "Untitled"
}
```

Response: `201 Created`

```json
{
  "document": {
    "id": "01HR...",
    "title": "Untitled",
    "markdown": "",
    "updatedAt": "2026-03-18T12:00:00Z",
    "createdAt": "2026-03-18T12:00:00Z"
  }
}
```

### `GET /api/documents/:id`

Returns metadata and latest persisted Markdown.

Response:

```json
{
  "document": {
    "id": "01HR...",
    "title": "Project Notes",
    "markdown": "# Project Notes\n",
    "updatedAt": "2026-03-18T12:00:00Z",
    "createdAt": "2026-03-18T10:00:00Z"
  }
}
```

### `PATCH /api/documents/:id/title`

Updates document title.

Request:

```json
{
  "title": "New Title"
}
```

Response:

```json
{
  "document": {
    "id": "01HR...",
    "title": "New Title",
    "updatedAt": "2026-03-18T12:02:00Z"
  }
}
```

### Error Shape

All API errors should use a consistent envelope.

```json
{
  "error": {
    "code": "document_not_found",
    "message": "Document not found"
  }
}
```

## 11. WebSocket Contract

Endpoint: `/api/documents/:id/live`

Transport: WebSocket

Message format: JSON envelope for control and presence messages, binary frames allowed later for raw Yjs update optimization.

For MVP simplicity, start with JSON using base64-encoded Yjs updates if needed. Move to binary frames only if profiling justifies it.

### Client to Server Messages

#### `join`

Sent immediately after socket open.

```json
{
  "type": "join",
  "payload": {
    "clientId": "c_8f3d...",
    "name": "Alice",
    "color": "#4F46E5"
  }
}
```

Rules:

- `clientId` is client-generated and stable for the browser session
- name and color come from local client state

#### `sync_update`

Carries a Yjs document update.

```json
{
  "type": "sync_update",
  "payload": {
    "clientId": "c_8f3d...",
    "update": "BASE64_ENCODED_YJS_UPDATE"
  }
}
```

#### `presence_update`

Carries cursor or selection metadata.

```json
{
  "type": "presence_update",
  "payload": {
    "clientId": "c_8f3d...",
    "anchor": 124,
    "head": 124
  }
}
```

#### `heartbeat`

Keeps ephemeral presence alive.

```json
{
  "type": "heartbeat",
  "payload": {
    "clientId": "c_8f3d..."
  }
}
```

### Server to Client Messages

#### `joined`

Confirms join and returns current room state.

```json
{
  "type": "joined",
  "payload": {
    "documentId": "01HR...",
    "serverTime": "2026-03-18T12:00:00Z",
    "snapshotVersion": 3,
    "peers": [
      {
        "clientId": "c_111",
        "name": "Alice",
        "color": "#4F46E5"
      }
    ]
  }
}
```

#### `sync_init`

Provides the latest full document state.

```json
{
  "type": "sync_init",
  "payload": {
    "snapshotVersion": 3,
    "update": "BASE64_ENCODED_FULL_YJS_STATE"
  }
}
```

#### `sync_update`

Broadcast of another client's update.

```json
{
  "type": "sync_update",
  "payload": {
    "clientId": "c_111",
    "update": "BASE64_ENCODED_YJS_UPDATE"
  }
}
```

#### `presence_snapshot`

Complete active collaborator list.

```json
{
  "type": "presence_snapshot",
  "payload": {
    "peers": [
      {
        "clientId": "c_111",
        "name": "Alice",
        "color": "#4F46E5",
        "anchor": 124,
        "head": 124
      }
    ]
  }
}
```

#### `peer_joined`

```json
{
  "type": "peer_joined",
  "payload": {
    "clientId": "c_222",
    "name": "Bob",
    "color": "#0EA5E9"
  }
}
```

#### `peer_left`

```json
{
  "type": "peer_left",
  "payload": {
    "clientId": "c_222"
  }
}
```

#### `error`

```json
{
  "type": "error",
  "payload": {
    "code": "invalid_message",
    "message": "Unsupported websocket message"
  }
}
```

### WebSocket Rules

- on connect, the client must send `join` first
- server responds with `joined`, then `sync_init`, then the latest `presence_snapshot`
- server broadcasts `sync_update` to all peers except the sender
- presence expires if heartbeat is not received within 30 seconds
- when the last peer disconnects, server triggers final snapshot persistence

## 12. Recommended Rust Service Structure

```text
backend/
  src/
    main.rs
    app_state.rs
    routes/
      documents.rs
      websocket.rs
    db/
      documents.rs
      snapshots.rs
      migrations.rs
    realtime/
      rooms.rs
      presence.rs
      sync.rs
    markdown/
      materialize.rs
    models/
      api.rs
      db.rs
      ws.rs
```

Notes:

- keep HTTP handlers thin
- keep room and persistence logic outside route files
- define explicit DTOs for API and WebSocket payloads

## 13. Recommended Frontend Structure

```text
frontend/
  src/
    routes/
      +page.svelte
      d/[id]/+page.svelte
    lib/
      api/
        documents.ts
      editor/
        schema.ts
        tiptap.ts
        markdown.ts
      realtime/
        client.ts
        presence.ts
        yjs.ts
      stores/
        session.ts
        document.ts
      components/
        DocumentList.svelte
        EditorShell.svelte
        PresenceBar.svelte
        SaveIndicator.svelte
```

Notes:

- keep the editor integration isolated from route files
- keep WebSocket transport separate from editor schema and serialization logic

## 14. Build Plan

### Phase 1: Project Setup

Goal: establish runnable frontend and backend apps.

Tasks:

- scaffold SvelteKit app with TypeScript
- scaffold Axum app
- configure SQLite access and migrations
- define shared document ID strategy
- add basic development scripts

Exit criteria:

- frontend runs locally
- backend runs locally
- database migrations apply cleanly

### Phase 2: Single-User Editing

Goal: get the editor model correct before realtime work.

Tasks:

- integrate Tiptap
- configure approved schema only
- implement Markdown import and export utilities
- build document list route
- build editor route
- implement create document flow
- implement title editing

Exit criteria:

- user can create, open, rename, and edit a document
- page reload restores document content from persisted Markdown

### Phase 3: Persistence

Goal: persist document data through Axum and SQLite.

Tasks:

- implement `GET /api/documents`
- implement `POST /api/documents`
- implement `GET /api/documents/:id`
- implement `PATCH /api/documents/:id/title`
- connect frontend to backend API
- implement autosave state indicator

Exit criteria:

- metadata and Markdown persist correctly
- list view reflects recent edits ordering

### Phase 4: Realtime Sync

Goal: enable multiple users to edit the same document safely.

Tasks:

- add WebSocket endpoint
- add room manager
- add Yjs sync flow
- send and receive initial document state
- relay updates across connected peers
- implement reconnect behavior

Exit criteria:

- two users can edit the same document simultaneously
- both clients converge on the same content state

### Phase 5: Presence and Snapshotting

Goal: make the collaboration experience usable and durable.

Tasks:

- implement join and leave events
- implement presence heartbeat
- render collaborator names and colors
- render remote cursor state if supported by the chosen integration
- persist snapshots on debounce and disconnect
- materialize Markdown from live state at checkpoints

Exit criteria:

- collaborators are visible
- reopened documents recover from latest persisted state

### Phase 6: MVP Hardening

Goal: remove obvious reliability issues before release.

Tasks:

- validate malformed WebSocket messages
- add basic API error handling
- add migration bootstrap for empty environment
- test disconnect and reconnect flows
- verify unsupported paste formats degrade safely
- verify no unsupported schema nodes leak into persisted Markdown

Exit criteria:

- basic multi-user editing is stable under normal local testing

## 15. Acceptance Criteria

The MVP is complete when all of the following are true:

- a user can create a document from the list page
- a user can open a document at `/d/:id`
- the title can be renamed in place
- supported rich-text content can be edited without seeing raw Markdown
- two browser sessions can edit the same document concurrently
- collaborator presence is visible
- document content is autosaved
- closing and reopening the document restores the latest persisted content
- persisted content in SQLite is stored as Markdown in `documents.markdown`

## 16. Non-Goals and Guardrails

- do not add features that do not round-trip to Markdown
- do not build custom CRDT logic
- do not optimize prematurely for horizontal scale
- do not add auth before the collaboration model is stable
- do not add version history UI in MVP

## 17. Immediate Next Step

Start implementation with Phase 1 and Phase 2 together:

- scaffold frontend and backend
- create SQLite schema and migration
- integrate single-user Tiptap editing
- persist Markdown through Axum API before adding WebSocket sync