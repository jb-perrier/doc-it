<script lang="ts">
    import { goto } from "$app/navigation";
    import { onMount, tick } from "svelte";
    import { browser } from "$app/environment";
    import { Search, Sun, Moon, Plus, FolderOpen } from "lucide-svelte";
    import * as Y from "yjs";

    import { deleteDocument, renameDocumentTitle } from "$lib/api/documents";
    import { listFolders } from "$lib/api/folders";
    import DocItLogo from "$lib/components/DocItLogo.svelte";
    import ProfileSettingsMenu from "$lib/components/ProfileSettingsMenu.svelte";
    import PresenceBar from "$lib/components/PresenceBar.svelte";
    import WorkspaceShell from "$lib/components/WorkspaceShell.svelte";
    import EditorShell from "$lib/components/EditorShell.svelte";
    import { getFolderPathSegments } from "$lib/folders/path";
    import { RealtimeClient } from "$lib/realtime/client";
    import {
        ensureSessionProfile,
        updateSessionProfileName,
    } from "$lib/stores/session";
    import { theme, toggleTheme } from "$lib/stores/theme";
    import type {
        DocumentRecord,
        DocumentSummary,
        FolderSummary,
        PeerPresence,
        SessionProfile,
    } from "$lib/types";

    let { data } = $props<{
        data: {
            id: string;
            document: DocumentRecord | null;
            snapshot: string | null;
            loadError: string;
        };
    }>();

    let document = $state<DocumentRecord | null>(null);
    let session = $state<SessionProfile | null>(null);
    let peers = $state<PeerPresence[]>([]);
    let errorMessage = $state("");
    let syncErrorMessage = $state("");
    let loading = $state(true);
    let titleDraft = $state("");
    let activeTopbarMenu = $state<string | null>(null);
    let folders = $state<FolderSummary[]>([]);
    let savedDocumentScrollY = 0;
    let hasSavedDocumentScroll = false;
    let usernameDraft = $state("");
    let usernameSaving = $state(false);
    let usernameErrorMessage = $state("");
    let creatingDocument = $state(false);
    let duplicatingDocument = $state(false);
    let deletingDocument = $state(false);

    const settingsMenuLabel = "Settings";

    let ydoc = $state<Y.Doc>(new Y.Doc());
    let client: RealtimeClient | null = null;
    let renameTimer: number | null = null;
    let sessionPromise: Promise<SessionProfile> | null = null;

    onMount(() => {
        if (!browser) {
            return;
        }

        window.addEventListener("pointerdown", handleDocumentPointerDown);

        return () => {
            window.removeEventListener(
                "pointerdown",
                handleDocumentPointerDown,
            );
        };
    });

    $effect(() => {
        if (!browser) {
            return;
        }

        const documentId = data.id;
        let cancelled = false;

        void loadDocumentPage(documentId, () => cancelled);

        return () => {
            cancelled = true;
            destroyActiveDocumentSession();
        };
    });

    async function loadDocumentPage(
        documentId: string,
        isCancelled: () => boolean,
    ) {
        resetDocumentState();

        const nextYdoc = new Y.Doc();
        ydoc = nextYdoc;
        const initialDocument = data.document;
        const initialSnapshot = data.snapshot;
        const initialError = data.loadError;

        if (!initialDocument) {
            errorMessage = initialError || "Failed to open document";
            loading = false;
            return;
        }

        if (initialSnapshot) {
            try {
                Y.applyUpdate(nextYdoc, decodeSnapshot(initialSnapshot));
            } catch {
                syncErrorMessage = "Failed to preload document content";
            }
        }

        document = initialDocument;
        titleDraft = initialDocument.title;
        loading = false;

        void loadFolders(isCancelled);
        void initializeRealtimeSession(documentId, nextYdoc, isCancelled);
    }

    function destroyActiveDocumentSession() {
        client?.disconnect();
        client = null;
        peers = [];

        if (renameTimer) {
            clearTimeout(renameTimer);
            renameTimer = null;
        }

        ydoc.destroy();
    }

    function resetDocumentState() {
        document = null;
        peers = [];
        errorMessage = "";
        syncErrorMessage = "";
        loading = true;
        titleDraft = "";
        usernameErrorMessage = "";
        activeTopbarMenu = null;
    }

    async function loadFolders(isCancelled: () => boolean) {
        const nextFolders = await listFolders().catch(() => []);

        if (isCancelled()) {
            return;
        }

        folders = nextFolders;
    }

    async function getSessionProfile() {
        if (session) {
            return session;
        }

        if (!sessionPromise) {
            sessionPromise = ensureSessionProfile().then((profile) => {
                session = profile;
                usernameDraft = profile.name;
                return profile;
            });
        }

        return sessionPromise;
    }

    async function initializeRealtimeSession(
        documentId: string,
        targetDoc: Y.Doc,
        isCancelled: () => boolean,
    ) {
        try {
            const nextSession = await getSessionProfile();

            if (isCancelled()) {
                return;
            }

            session = nextSession;
            usernameDraft = nextSession.name;
            syncErrorMessage = "";

            await connectRealtimeClient(
                documentId,
                nextSession,
                targetDoc,
                isCancelled,
            );
        } catch (error) {
            if (isCancelled()) {
                return;
            }

            syncErrorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to connect to live document";
        }
    }

    function decodeSnapshot(value: string): Uint8Array {
        const binary = atob(value);
        const output = new Uint8Array(binary.length);

        for (let index = 0; index < binary.length; index += 1) {
            output[index] = binary.charCodeAt(index);
        }

        return output;
    }

    async function connectRealtimeClient(
        documentId: string,
        nextSession: SessionProfile,
        targetDoc: Y.Doc,
        isCancelled: () => boolean = () => false,
    ) {
        const nextClient = new RealtimeClient(
            documentId,
            nextSession,
            targetDoc,
            {
                onConnectionState() {},
                onPresence(nextPeers) {
                    if (isCancelled()) {
                        return;
                    }

                    peers = nextPeers.filter(
                        (peer) => peer.clientId !== nextSession.clientId,
                    );
                },
                onInitialSync() {
                    if (isCancelled()) {
                        return;
                    }
                },
            },
        );

        client = nextClient;
        await nextClient.connect();
    }

    function handleSelectionChange(anchor: number, head: number) {
        client?.updatePresence(anchor, head);
    }

    function handleTitleChange(value: string) {
        titleDraft = value;

        if (renameTimer) {
            clearTimeout(renameTimer);
        }

        renameTimer = setTimeout(async () => {
            try {
                if (!document) {
                    return;
                }

                const updated = await renameDocumentTitle(document.id, value);
                document = { ...document, ...updated };
            } catch (error) {
                errorMessage =
                    error instanceof Error
                        ? error.message
                        : "Failed to rename document";
            }
        }, 450);
    }

    function handleTopbarMenuToggle(label: string, isOpen: boolean) {
        activeTopbarMenu = isOpen
            ? label
            : activeTopbarMenu === label
              ? null
              : activeTopbarMenu;

        if (label === settingsMenuLabel && isOpen && session) {
            usernameDraft = session.name;
            usernameErrorMessage = "";
        }
    }

    async function handleCreateDocument() {
        if (creatingDocument) {
            return;
        }

        activeTopbarMenu = null;
        errorMessage = "";
        creatingDocument = true;

        try {
            const search = new URLSearchParams();

            if (document?.folderId) {
                search.set("folderId", document.folderId);
            }

            await goto(search.size > 0 ? `/new?${search.toString()}` : "/new");
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to open new document page";
        } finally {
            creatingDocument = false;
        }
    }

    async function handleDeleteDocument() {
        if (!document || deletingDocument) {
            return;
        }

        if (
            browser &&
            !window.confirm(
                "Delete this document? This action cannot be undone.",
            )
        ) {
            return;
        }

        activeTopbarMenu = null;
        errorMessage = "";
        deletingDocument = true;

        try {
            await deleteDocument(document.id);

            destroyActiveDocumentSession();
            document = null;
            loading = false;

            try {
                await goto("/search", {
                    replaceState: true,
                    invalidateAll: true,
                });
            } catch {
                window.location.assign("/search");
            }
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to delete document";
        } finally {
            deletingDocument = false;
        }
    }

    async function handleDuplicateDocument() {
        if (!document || duplicatingDocument || deletingDocument) {
            return;
        }

        activeTopbarMenu = null;
        errorMessage = "";
        duplicatingDocument = true;

        try {
            const search = new URLSearchParams({
                source: document.id,
                folderId: document.folderId,
                title: `${(titleDraft.trim() || document.title || "Untitled").trim()} Copy`,
            });

            await goto(`/new?${search.toString()}`);
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to open duplicate document page";
        } finally {
            duplicatingDocument = false;
        }
    }

    async function handleUsernameSubmit() {
        if (!session || usernameSaving) {
            return;
        }

        const nextName = usernameDraft.trim();
        if (!nextName) {
            usernameErrorMessage = "Username cannot be empty";
            return;
        }

        if (nextName === session.name) {
            activeTopbarMenu = null;
            usernameErrorMessage = "";
            return;
        }

        usernameSaving = true;
        usernameErrorMessage = "";

        try {
            const nextSession = await updateSessionProfileName(nextName);
            session = nextSession;
            sessionPromise = Promise.resolve(nextSession);
            usernameDraft = nextSession.name;

            if (document) {
                client?.disconnect();
                client = null;
                await connectRealtimeClient(data.id, nextSession, ydoc);
            }

            activeTopbarMenu = null;
        } catch (error) {
            usernameErrorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to update username";
        } finally {
            usernameSaving = false;
        }
    }

    function getDocumentFolderPath() {
        if (!document) {
            return [];
        }

        return getFolderPathSegments(document.folderId, folders);
    }

    async function handleOpenDocumentFolderPath() {
        if (!document) {
            return;
        }

        const search = new URLSearchParams({
            folderId: document.folderId,
        });

        await goto(`/explore?${search.toString()}`);
    }

    function handleDocumentPointerDown(event: PointerEvent) {
        if (!activeTopbarMenu) {
            return;
        }

        const target = event.target;
        if (
            !(target instanceof Element) ||
            !target.closest(".dropdown-badge")
        ) {
            activeTopbarMenu = null;
        }
    }
</script>

<WorkspaceShell>
    {#snippet leftRail()}
        <div class="topbar-left">
            <a
                href="/search"
                class="document-menu-brand"
                aria-label="Go to search page"
            >
                <span class="document-menu-brand__logo">
                    <DocItLogo />
                </span>
            </a>
            <button
                type="button"
                class="menu-badge-button"
                onclick={() => void handleCreateDocument()}
                disabled={creatingDocument}
            >
                <span>{creatingDocument ? "Creating..." : "New"}</span>
                <Plus size={14} strokeWidth={2.2} />
            </button>
            <button
                type="button"
                class="menu-badge-button"
                onclick={() =>
                    void goto(
                        document?.folderId
                            ? `/explore?folderId=${document.folderId}`
                            : "/explore",
                    )}
            >
                <span>Explore</span>
                <FolderOpen size={14} strokeWidth={2.2} />
            </button>
            <button
                type="button"
                class="menu-badge-button"
                onclick={() => void goto("/search")}
            >
                <span>Search</span>
                <Search size={14} strokeWidth={2.2} />
            </button>
        </div>
    {/snippet}

    {#snippet stage()}
        <div class="editor-content">
            <div class="editor-stage">
                {#if !loading && errorMessage && !document}
                    <p class="status-card error">{errorMessage}</p>
                {:else if !loading && document}
                    {#if syncErrorMessage}
                        <p class="status-card">{syncErrorMessage}</p>
                    {/if}
                    {#key data.id}
                        <EditorShell
                            title={titleDraft}
                            doc={ydoc}
                            {peers}
                            folderPath={getDocumentFolderPath()}
                            {duplicatingDocument}
                            {deletingDocument}
                            onTitleChange={handleTitleChange}
                            onDuplicateDocument={handleDuplicateDocument}
                            onOpenFolderPath={() =>
                                void handleOpenDocumentFolderPath()}
                            onDeleteDocument={handleDeleteDocument}
                            onSelectionChange={handleSelectionChange}
                        />
                    {/key}
                {/if}
            </div>
        </div>
    {/snippet}

    {#snippet rightRail()}
        <div class="topbar-meta">
            <PresenceBar {peers} />
            <div class="side-rail__actions side-rail__actions--right">
                {#if document}
                    <button
                        type="button"
                        class="menu-badge-button menu-badge-button--icon"
                        onclick={toggleTheme}
                        aria-label={$theme === "dark"
                            ? "Switch to light theme"
                            : "Switch to dark theme"}
                        title={$theme === "dark"
                            ? "Switch to light theme"
                            : "Switch to dark theme"}
                    >
                        {#if $theme === "dark"}
                            <Moon size={14} strokeWidth={2.2} />
                        {:else}
                            <Sun size={14} strokeWidth={2.2} />
                        {/if}
                    </button>
                    {#if session}
                        <ProfileSettingsMenu
                            open={activeTopbarMenu === settingsMenuLabel}
                            onToggle={(isOpen) =>
                                handleTopbarMenuToggle(
                                    settingsMenuLabel,
                                    isOpen,
                                )}
                            username={usernameDraft}
                            saving={usernameSaving}
                            errorMessage={usernameErrorMessage}
                            onUsernameInput={(value) => {
                                usernameDraft = value;
                            }}
                            onSubmit={() => void handleUsernameSubmit()}
                        />
                    {/if}
                {/if}
            </div>
        </div>
    {/snippet}
</WorkspaceShell>

<style>
    .editor-content {
        position: relative;
        display: grid;
        gap: 0;
        min-height: 70vh;
    }

    .editor-stage {
        min-height: 70vh;
    }

    .status-card {
        padding: 22px 24px;
        background: var(--panel);
        border: 1px solid var(--line);
        border-radius: 16px;
        color: var(--muted);
    }

    .error {
        color: var(--accent-strong);
    }

    .document-menu-brand {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        block-size: var(--menu-badge-height);
        border-radius: 10px;
        color: inherit;
        text-decoration: none;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .document-menu-brand:hover,
    .document-menu-brand:focus-visible {
        background: var(--surface-overlay-medium);
        outline: none;
    }

    .document-menu-brand__logo {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        inline-size: clamp(72px, 11vw, 96px);
        block-size: 100%;
        line-height: 1;
        color: var(--text);
    }

    @media (max-width: 1320px) {
    }
</style>
