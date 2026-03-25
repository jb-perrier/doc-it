<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { Copy, Moon, Plus, Search, Sun, FolderOpen } from "lucide-svelte";

    import {
        createDocument,
        duplicateDocument,
        listDocuments,
        moveDocumentToFolder,
        renameDocumentTitle,
    } from "$lib/api/documents";
    import { listFolders } from "$lib/api/folders";
    import DocItLogo from "$lib/components/DocItLogo.svelte";
    import DocumentSearchWorkspace from "$lib/components/DocumentSearchWorkspace.svelte";
    import ProfileSettingsMenu from "$lib/components/ProfileSettingsMenu.svelte";
    import WorkspaceShell from "$lib/components/WorkspaceShell.svelte";
    import {
        createSubfolderInCollections,
        deleteFolderInCollections,
        renameFolderInCollections,
    } from "$lib/folders/operations";
    import { getFolderPathSegments } from "$lib/folders/path";
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
        SessionProfile,
    } from "$lib/types";

    const ROOT_FOLDER_ID = "workspace-root";
    const settingsMenuLabel = "Settings";

    const initialSourceDocumentId = page.url.searchParams.get("source");
    const initialExplorerFolderId =
        page.url.searchParams.get("folderId") ?? ROOT_FOLDER_ID;
    const initialTitle = getInitialTitle();

    let documents = $state<DocumentSummary[]>([]);
    let folders = $state<FolderSummary[]>([]);
    let session = $state<SessionProfile | null>(null);
    let loading = $state(true);
    let submitting = $state(false);
    let errorMessage = $state("");
    let titleDraft = $state(initialTitle);
    let currentExplorerFolderId = $state<string | null>(
        initialExplorerFolderId,
    );
    let activeTopbarMenu = $state<string | null>(null);
    let usernameDraft = $state("");
    let usernameSaving = $state(false);
    let usernameErrorMessage = $state("");

    onMount(async () => {
        await refreshDocuments();
    });

    function getInitialTitle() {
        const rawTitle = page.url.searchParams.get("title")?.trim();
        return rawTitle && rawTitle.length > 0 ? rawTitle : "Untitled";
    }

    function getSearchResults() {
        return getDocumentSearchResults(documents, "");
    }

    function getFolderPath(document: DocumentSummary) {
        return getFolderPathSegments(document.folderId, folders);
    }

    function getTargetFolderId() {
        return currentExplorerFolderId ?? ROOT_FOLDER_ID;
    }

    function getPrimaryActionLabel() {
        return initialSourceDocumentId ? "Duplicate" : "Create";
    }

    function handlePagePointerDown(event: PointerEvent) {
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

    async function refreshDocuments() {
        loading = true;
        errorMessage = "";

        try {
            const [nextDocuments, nextFolders] = await Promise.all([
                listDocuments(),
                listFolders().catch(() => []),
            ]);
            documents = nextDocuments;
            folders = nextFolders;
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to load documents";
        } finally {
            loading = false;
        }
    }

    function handleTitleChange(value: string) {
        titleDraft = value;
        if (errorMessage === "Document name cannot be empty") {
            errorMessage = "";
        }
    }

    function handleTitleInputKeyDown(event: KeyboardEvent) {
        if (event.key !== "Enter") {
            return;
        }

        event.preventDefault();
        void handleSubmit();
    }

    async function handleSubmit() {
        if (submitting) {
            return;
        }

        const nextTitle = titleDraft.trim();
        if (!nextTitle) {
            errorMessage = "Document name cannot be empty";
            return;
        }

        submitting = true;
        errorMessage = "";

        try {
            let nextDocument: DocumentRecord;
            const targetFolderId = getTargetFolderId();

            if (initialSourceDocumentId) {
                nextDocument = await duplicateDocument(initialSourceDocumentId);

                if (nextDocument.title !== nextTitle) {
                    const renamed = await renameDocumentTitle(
                        nextDocument.id,
                        nextTitle,
                    );
                    nextDocument = { ...nextDocument, ...renamed };
                }

                if (nextDocument.folderId !== targetFolderId) {
                    nextDocument = await moveDocumentToFolder(
                        nextDocument.id,
                        targetFolderId,
                    );
                }
            } else {
                nextDocument = await createDocument(nextTitle, targetFolderId);
            }

            await goto(`/d/${nextDocument.id}`);
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : `Failed to ${getPrimaryActionLabel().toLowerCase()} document`;
            submitting = false;
        }
    }

    async function handleTopbarMenuToggle(label: string, isOpen: boolean) {
        activeTopbarMenu = isOpen
            ? label
            : activeTopbarMenu === label
              ? null
              : activeTopbarMenu;

        if (label === settingsMenuLabel && isOpen) {
            const nextSession = session ?? (await ensureSessionProfile());
            session = nextSession;
            usernameDraft = nextSession.name;
            usernameErrorMessage = "";
        }
    }

    function handleUsernameDraftChange(value: string) {
        usernameDraft = value;
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
            usernameDraft = nextSession.name;
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

    async function handleRenameFolder(folderId: string, name: string) {
        const nextState = await renameFolderInCollections(
            { folders, searchFolders: folders },
            folderId,
            name,
        );

        folders = nextState.folders;

        return nextState.updated;
    }

    async function handleCreateSubfolder(parentFolderId: string) {
        const nextState = await createSubfolderInCollections(
            { folders, searchFolders: folders },
            parentFolderId,
        );

        folders = nextState.folders;

        return nextState.created;
    }

    async function handleDeleteFolder(folderId: string) {
        const nextState = await deleteFolderInCollections(
            { folders, searchFolders: folders },
            folderId,
        );

        folders = nextState.folders;
    }
</script>

<svelte:head>
    <title>Doc-it | New</title>
</svelte:head>

<svelte:document onpointerdown={handlePagePointerDown} />

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
                onclick={() => void goto("/search")}
            >
                <span>Search</span>
                <Search size={14} strokeWidth={2.2} />
            </button>
            <button
                type="button"
                class="menu-badge-button"
                onclick={() =>
                    void goto(
                        currentExplorerFolderId &&
                            currentExplorerFolderId !== ROOT_FOLDER_ID
                            ? `/explore?folderId=${currentExplorerFolderId}`
                            : "/explore",
                    )}
            >
                <span>Explore</span>
                <FolderOpen size={14} strokeWidth={2.2} />
            </button>
        </div>
    {/snippet}

    {#snippet stage()}
        {#snippet explorerToolbarAction()}
            <button
                type="button"
                class="menu-badge-button"
                onclick={() => void handleSubmit()}
                disabled={loading ||
                    submitting ||
                    titleDraft.trim().length === 0}
            >
                <span
                    >{submitting
                        ? `${getPrimaryActionLabel()}ing...`
                        : getPrimaryActionLabel()}</span
                >
                {#if initialSourceDocumentId}
                    <Copy size={14} strokeWidth={2.2} />
                {:else}
                    <Plus size={14} strokeWidth={2.2} />
                {/if}
            </button>
        {/snippet}

        <DocumentSearchWorkspace
            mode="create"
            query=""
            results={getSearchResults()}
            {folders}
            viewMode="explorer"
            selectedIndex={0}
            {loading}
            {errorMessage}
            emptyMessage="No folders or documents yet."
            titleValue={titleDraft}
            titlePlaceholder={initialSourceDocumentId
                ? "Document copy title"
                : "Document title"}
            {explorerToolbarAction}
            {getFolderPath}
            {initialExplorerFolderId}
            onTitleChange={handleTitleChange}
            onKeyDown={handleTitleInputKeyDown}
            onExplorerFolderChange={(folderId) => {
                currentExplorerFolderId = folderId;
            }}
            onCreateSubfolder={handleCreateSubfolder}
            onRenameFolder={handleRenameFolder}
            onDeleteFolder={handleDeleteFolder}
        />
    {/snippet}

    {#snippet rightRail()}
        <div class="topbar-meta">
            <div class="side-rail__actions side-rail__actions--right">
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
                <ProfileSettingsMenu
                    open={activeTopbarMenu === settingsMenuLabel}
                    onToggle={(isOpen) =>
                        void handleTopbarMenuToggle(settingsMenuLabel, isOpen)}
                    username={usernameDraft}
                    saving={usernameSaving}
                    errorMessage={usernameErrorMessage}
                    onUsernameInput={handleUsernameDraftChange}
                    onSubmit={() => void handleUsernameSubmit()}
                />
            </div>
        </div>
    {/snippet}
</WorkspaceShell>

<style>
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
</style>
