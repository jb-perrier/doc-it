<script lang="ts">
    import { browser } from "$app/environment";
    import { goto, replaceState } from "$app/navigation";
    import { page } from "$app/state";
    import { onMount, untrack } from "svelte";
    import { Search, Sun, Moon, Plus } from "lucide-svelte";

    import {
        deleteDocument,
        listDocuments,
        renameDocumentTitle,
    } from "$lib/api/documents";
    import { listFolders } from "$lib/api/folders";
    import DocItLogo from "$lib/components/DocItLogo.svelte";
    import DocumentSearchWorkspace from "$lib/components/DocumentSearchWorkspace.svelte";
    import {
        createSubfolderInCollections,
        deleteFolderInCollections,
        moveDocumentWithinFolders,
        renameFolderInCollections,
    } from "$lib/folders/operations";
    import { getFolderPathSegments } from "$lib/folders/path";
    import ProfileSettingsMenu from "$lib/components/ProfileSettingsMenu.svelte";
    import WorkspaceShell from "$lib/components/WorkspaceShell.svelte";
    import { getDocumentSearchResults } from "$lib/search/documents";
    import {
        ensureSessionProfile,
        updateSessionProfileName,
    } from "$lib/stores/session";
    import { theme, toggleTheme } from "$lib/stores/theme";
    import type {
        DocumentSummary,
        FolderSummary,
        SessionProfile,
    } from "$lib/types";

    const ROOT_FOLDER_ID = "workspace-root";
    const settingsMenuLabel = "Settings";

    let documents = $state<DocumentSummary[]>([]);
    let folders = $state<FolderSummary[]>([]);
    let session = $state<SessionProfile | null>(null);
    let loading = $state(true);
    let creating = $state(false);
    let errorMessage = $state("");
    let activeTopbarMenu = $state<string | null>(null);
    let currentExplorerFolderId = $state<string | null>(ROOT_FOLDER_ID);
    let usernameDraft = $state("");
    let usernameSaving = $state(false);
    let usernameErrorMessage = $state("");

    $effect(() => {
        const currentUrl = page.url;
        const nextExplorerFolderId =
            currentUrl.searchParams.get("folderId") ?? ROOT_FOLDER_ID;

        untrack(() => {
            if (currentExplorerFolderId !== nextExplorerFolderId) {
                currentExplorerFolderId = nextExplorerFolderId;
            }
        });
    });

    $effect(() => {
        if (!browser) {
            return;
        }

        const currentUrl = untrack(() => page.url);
        const nextUrl = new URL(currentUrl);

        if (
            currentExplorerFolderId &&
            currentExplorerFolderId !== ROOT_FOLDER_ID
        ) {
            nextUrl.searchParams.set("folderId", currentExplorerFolderId);
        } else {
            nextUrl.searchParams.delete("folderId");
        }

        if (nextUrl.search !== currentUrl.search) {
            replaceState(
                nextUrl,
                untrack(() => page.state),
            );
        }
    });

    onMount(async () => {
        await refreshDocuments();
    });

    function getExploreResults() {
        return getDocumentSearchResults(documents, "");
    }

    function getFolderPath(document: DocumentSummary) {
        return getFolderPathSegments(document.folderId, folders);
    }

    function getFolderName(folderId: string) {
        return (
            folders.find((folder) => folder.id === folderId)?.name ?? "folder"
        );
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

    async function handleCreate() {
        if (creating) {
            return;
        }

        creating = true;

        try {
            const search = new URLSearchParams();

            if (
                currentExplorerFolderId &&
                currentExplorerFolderId !== ROOT_FOLDER_ID
            ) {
                search.set("folderId", currentExplorerFolderId);
            }

            await goto(search.size > 0 ? `/new?${search.toString()}` : "/new");
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to open new document page";
            creating = false;
        }
    }

    function handleExplorerFolderChange(folderId: string | null) {
        currentExplorerFolderId = folderId ?? ROOT_FOLDER_ID;
    }

    async function handleMoveSearchResult(
        target: DocumentSummary,
        folderId: string,
    ) {
        if (
            typeof window !== "undefined" &&
            !window.confirm(
                `Move document "${target.title || "Untitled"}" to "${getFolderName(folderId)}"?`,
            )
        ) {
            return;
        }

        const updated = await moveDocumentWithinFolders(target, folderId);

        documents = documents.map((document) =>
            document.id === updated.id ? updated : document,
        );
    }

    async function handleRenameDocument(documentId: string, title: string) {
        const updated = await renameDocumentTitle(documentId, title);

        documents = documents.map((document) =>
            document.id === updated.id ? updated : document,
        );

        return updated;
    }

    async function handleDeleteSearchDocument(documentId: string) {
        await deleteDocument(documentId);

        documents = documents.filter((document) => document.id !== documentId);
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
</script>

<svelte:head>
    <title>Doc-it | Explore</title>
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
                onclick={handleCreate}
                disabled={creating}
            >
                <span>{creating ? "Opening..." : "New"}</span>
                <Plus size={14} strokeWidth={2.2} />
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
        <div class="explore-stage">
            <DocumentSearchWorkspace
                mode="explore"
                query=""
                results={getExploreResults()}
                {folders}
                viewMode="explorer"
                selectedIndex={0}
                {loading}
                {errorMessage}
                emptyMessage="No folders or documents yet."
                {getFolderPath}
                initialExplorerFolderId={currentExplorerFolderId}
                onExplorerFolderChange={handleExplorerFolderChange}
                onOpenResult={(target) => goto(`/d/${target.id}`)}
                onMoveResultToFolder={handleMoveSearchResult}
                onCreateSubfolder={handleCreateSubfolder}
                onRenameFolder={handleRenameFolder}
                onDeleteFolder={handleDeleteFolder}
                onRenameDocument={handleRenameDocument}
                onDeleteDocument={handleDeleteSearchDocument}
            />
        </div>
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

    .explore-stage {
        padding-top: clamp(18px, 3vw, 36px);
    }
</style>
