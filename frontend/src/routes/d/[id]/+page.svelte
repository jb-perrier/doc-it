<script lang="ts">
    import { goto } from "$app/navigation";
    import { onMount, tick } from "svelte";
    import { browser } from "$app/environment";
    import { ChevronDown, Search, Sun, Moon, Plus } from "lucide-svelte";
    import * as Y from "yjs";

    import {
        createDocument,
        deleteDocument,
        duplicateDocument,
        listDocuments,
        renameDocumentTitle,
    } from "$lib/api/documents";
    import { listFolders } from "$lib/api/folders";
    import DocItLogo from "$lib/components/DocItLogo.svelte";
    import DocumentSearchWorkspace from "$lib/components/DocumentSearchWorkspace.svelte";
    import ProfileSettingsMenu from "$lib/components/ProfileSettingsMenu.svelte";
    import PresenceBar from "$lib/components/PresenceBar.svelte";
    import WorkspaceShell from "$lib/components/WorkspaceShell.svelte";
    import EditorShell from "$lib/components/EditorShell.svelte";
    import { getFolderPathSegments } from "$lib/folders/path";
    import {
        createSubfolderInCollections,
        deleteFolderInCollections,
        moveDocumentWithinFolders,
        renameFolderInCollections,
    } from "$lib/folders/operations";
    import { RealtimeClient } from "$lib/realtime/client";
    import { getDocumentSearchResults } from "$lib/search/documents";
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
    let searchModeOpen = $state(false);
    let searchQuery = $state("");
    let searchResultsIndex = $state(0);
    let folders = $state<FolderSummary[]>([]);
    let searchDocuments = $state<DocumentSummary[] | null>(null);
    let searchFolders = $state<FolderSummary[] | null>(null);
    let searchLoading = $state(false);
    let searchErrorMessage = $state("");
    let savedDocumentScrollY = 0;
    let hasSavedDocumentScroll = false;
    let usernameDraft = $state("");
    let usernameSaving = $state(false);
    let usernameErrorMessage = $state("");
    let creatingDocument = $state(false);
    let duplicatingDocument = $state(false);
    let deletingDocument = $state(false);
    let moveFolderPending = $state(false);

    const shareMenu = {
        label: "Share",
        items: ["Invite collaborators", "Copy share link", "Publish snapshot"],
    } as const;

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

    $effect(() => {
        searchQuery;
        searchResultsIndex = 0;
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
        searchDocuments = null;
        searchFolders = null;
        searchLoading = false;
        searchErrorMessage = "";
        usernameErrorMessage = "";
        activeTopbarMenu = null;
        closeSearchMode({ restoreScroll: false });
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
            const nextDocument = await createDocument(
                "Untitled",
                document?.folderId,
            );
            await goto(`/d/${nextDocument.id}`);
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to create document";
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
                await goto("/", { replaceState: true, invalidateAll: true });
            } catch {
                window.location.assign("/");
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
            const nextDocument = await duplicateDocument(document.id);
            await goto(`/d/${nextDocument.id}`);
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to duplicate document";
        } finally {
            duplicatingDocument = false;
        }
    }

    async function handleMoveDocumentFolder(folderId: string) {
        if (!document || moveFolderPending || folderId === document.folderId) {
            return;
        }

        moveFolderPending = true;

        try {
            const updated = await moveDocumentWithinFolders(document, folderId);
            document = updated;
        } finally {
            moveFolderPending = false;
        }
    }

    async function handleRenameFolder(folderId: string, name: string) {
        const nextState = await renameFolderInCollections(
            { folders, searchFolders },
            folderId,
            name,
        );

        folders = nextState.folders;
        searchFolders = nextState.searchFolders;

        return nextState.updated;
    }

    async function handleCreateSubfolder(parentFolderId: string) {
        const nextState = await createSubfolderInCollections(
            { folders, searchFolders },
            parentFolderId,
        );

        folders = nextState.folders;
        searchFolders = nextState.searchFolders;

        return nextState.created;
    }

    async function handleDeleteFolder(folderId: string) {
        const nextState = await deleteFolderInCollections(
            { folders, searchFolders },
            folderId,
        );

        folders = nextState.folders;
        searchFolders = nextState.searchFolders;
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

    async function openSearchMode() {
        activeTopbarMenu = null;

        if (browser && !searchModeOpen) {
            savedDocumentScrollY = window.scrollY;
            hasSavedDocumentScroll = true;
        }

        searchModeOpen = true;
        searchQuery = "";
        searchResultsIndex = 0;

        if (browser) {
            await tick();
            window.scrollTo({ top: 0, behavior: "auto" });
        }

        await ensureSearchDocumentsLoaded();
    }

    function closeSearchMode(options: { restoreScroll?: boolean } = {}) {
        const { restoreScroll = true } = options;
        const scrollTarget =
            browser && restoreScroll && hasSavedDocumentScroll
                ? savedDocumentScrollY
                : null;

        searchModeOpen = false;
        searchQuery = "";
        searchResultsIndex = 0;
        searchErrorMessage = "";
        hasSavedDocumentScroll = false;
        savedDocumentScrollY = 0;

        if (scrollTarget !== null) {
            void tick().then(() => {
                window.scrollTo({ top: scrollTarget, behavior: "auto" });
            });
        }
    }

    async function ensureSearchDocumentsLoaded() {
        if (searchLoading || (searchDocuments && searchFolders)) {
            return;
        }

        searchLoading = true;
        searchErrorMessage = "";
        try {
            const [nextDocuments, nextFolders] = await Promise.all([
                searchDocuments
                    ? Promise.resolve(searchDocuments)
                    : listDocuments(),
                searchFolders
                    ? Promise.resolve(searchFolders)
                    : folders.length > 0
                      ? Promise.resolve(folders)
                      : listFolders().catch(() => []),
            ]);

            searchDocuments = nextDocuments;
            searchFolders = nextFolders;
        } catch (error) {
            searchErrorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to load documents";
        } finally {
            searchLoading = false;
        }
    }

    function getSearchResults() {
        return getDocumentSearchResults(searchDocuments ?? [], searchQuery);
    }

    function getDocumentFolderPath() {
        if (!document) {
            return [];
        }

        return getFolderPathSegments(document.folderId, folders);
    }

    function getFolderPath(document: DocumentSummary) {
        return getFolderPathSegments(document.folderId, searchFolders ?? []);
    }

    function handleSearchInputKeyDown(event: KeyboardEvent) {
        const results = getSearchResults();

        if (event.key === "Escape") {
            event.preventDefault();
            closeSearchMode();
            return;
        }

        if (results.length === 0) {
            return;
        }

        if (event.key === "ArrowDown") {
            event.preventDefault();
            searchResultsIndex = (searchResultsIndex + 1) % results.length;
            return;
        }

        if (event.key === "ArrowUp") {
            event.preventDefault();
            searchResultsIndex =
                (searchResultsIndex - 1 + results.length) % results.length;
            return;
        }

        if (event.key === "Enter") {
            event.preventDefault();
            void openSearchResult(
                results[searchResultsIndex]?.document ?? null,
            );
        }
    }

    function handleSearchQueryChange(value: string) {
        searchQuery = value;
    }

    function handleSearchResultHover(index: number) {
        searchResultsIndex = index;
    }

    async function handleMoveSearchResult(
        target: DocumentSummary,
        folderId: string,
    ) {
        const updated = await moveDocumentWithinFolders(target, folderId);

        searchDocuments = (searchDocuments ?? []).map((searchDocument) =>
            searchDocument.id === updated.id ? updated : searchDocument,
        );

        if (document?.id === updated.id) {
            document = updated;
        }
    }

    async function handleRenameSearchDocument(
        documentId: string,
        title: string,
    ) {
        const updated = await renameDocumentTitle(documentId, title);

        searchDocuments = (searchDocuments ?? []).map((searchDocument) =>
            searchDocument.id === updated.id ? updated : searchDocument,
        );

        if (document?.id === updated.id) {
            document = { ...document, ...updated };
            titleDraft = updated.title;
        }

        return updated;
    }

    async function handleDeleteSearchDocument(documentId: string) {
        await deleteDocument(documentId);

        searchDocuments = (searchDocuments ?? []).filter(
            (searchDocument) => searchDocument.id !== documentId,
        );

        if (document?.id !== documentId) {
            return;
        }

        destroyActiveDocumentSession();
        document = null;
        loading = false;

        try {
            await goto("/", { replaceState: true, invalidateAll: true });
        } catch {
            window.location.assign("/");
        }
    }

    async function openSearchResult(target: DocumentSummary | null) {
        if (!target) {
            return;
        }

        if (target.id === data.id) {
            closeSearchMode({ restoreScroll: false });
            return;
        }

        await goto(`/d/${target.id}`);
        closeSearchMode({ restoreScroll: false });
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
                href="/"
                class="document-menu-brand"
                aria-label="Go to main page"
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
            {#if searchModeOpen}
                <button
                    type="button"
                    class="menu-badge-button"
                    onclick={() => closeSearchMode()}
                >
                    <span>Back to document</span>
                </button>
            {:else}
                <button
                    type="button"
                    class="menu-badge-button"
                    onclick={() => void openSearchMode()}
                >
                    <span>Search</span>
                    <Search size={14} strokeWidth={2.2} />
                </button>
            {/if}
        </div>
    {/snippet}

    {#snippet stage()}
        <div class="editor-content">
            {#snippet searchInputLeading()}
                <Search size={18} strokeWidth={2.1} />
            {/snippet}

            {#if searchModeOpen}
                <DocumentSearchWorkspace
                    query={searchQuery}
                    results={getSearchResults()}
                    folders={searchFolders ?? []}
                    selectedIndex={searchResultsIndex}
                    loading={searchLoading}
                    errorMessage={searchErrorMessage}
                    inputLeading={searchInputLeading}
                    {getFolderPath}
                    onQueryChange={handleSearchQueryChange}
                    onKeyDown={handleSearchInputKeyDown}
                    onOpenResult={(target) => void openSearchResult(target)}
                    onHoverResult={handleSearchResultHover}
                    onMoveResultToFolder={handleMoveSearchResult}
                    onCreateSubfolder={handleCreateSubfolder}
                    onRenameFolder={handleRenameFolder}
                    onDeleteFolder={handleDeleteFolder}
                    onRenameDocument={handleRenameSearchDocument}
                    onDeleteDocument={handleDeleteSearchDocument}
                />
            {:else}
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
                                folderId={document.folderId}
                                {folders}
                                folderPath={getDocumentFolderPath()}
                                {moveFolderPending}
                                {duplicatingDocument}
                                {deletingDocument}
                                onTitleChange={handleTitleChange}
                                onCreateSubfolder={handleCreateSubfolder}
                                onDuplicateDocument={handleDuplicateDocument}
                                onDeleteDocument={handleDeleteDocument}
                                onMoveToFolder={handleMoveDocumentFolder}
                                onRenameFolder={handleRenameFolder}
                                onSelectionChange={handleSelectionChange}
                            />
                        {/key}
                    {/if}
                </div>
            {/if}
        </div>
    {/snippet}

    {#snippet rightRail()}
        <div class="topbar-meta">
            <PresenceBar {peers} />
            <div class="side-rail__actions side-rail__actions--right">
                {#if document}
                    {#if searchModeOpen}
                        <button
                            type="button"
                            class="menu-badge-button menu-badge-button--disabled"
                            disabled
                        >
                            <span>{shareMenu.label}</span>
                            <ChevronDown size={14} strokeWidth={2.2} />
                        </button>
                    {:else}
                        <details
                            class="dropdown-badge dropdown-badge--right"
                            open={activeTopbarMenu === shareMenu.label}
                            ontoggle={(event) =>
                                handleTopbarMenuToggle(
                                    shareMenu.label,
                                    (event.currentTarget as HTMLDetailsElement)
                                        .open,
                                )}
                        >
                            <summary>
                                <span>{shareMenu.label}</span>
                                <ChevronDown size={14} strokeWidth={2.2} />
                            </summary>
                            <div class="dropdown-panel">
                                <div class="dropdown-items">
                                    {#each shareMenu.items as item (item)}
                                        <button
                                            type="button"
                                            class="dropdown-item"
                                        >
                                            {item}
                                        </button>
                                    {/each}
                                </div>
                            </div>
                        </details>
                    {/if}
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
                {:else}
                    <button
                        type="button"
                        class="menu-badge-button menu-badge-button--disabled"
                        disabled
                    >
                        <span>{shareMenu.label}</span>
                        <ChevronDown size={14} strokeWidth={2.2} />
                    </button>
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
