<script lang="ts">
    import { browser } from "$app/environment";
    import { goto, replaceState } from "$app/navigation";
    import { page } from "$app/state";
    import { onMount, untrack } from "svelte";
    import { Search, Sun, Moon, Plus, FolderOpen } from "lucide-svelte";

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

    type SearchViewMode = "list" | "tree" | "explorer";

    let documents = $state<DocumentSummary[]>([]);
    let folders = $state<FolderSummary[]>([]);
    let session = $state<SessionProfile | null>(null);
    let loading = $state(true);
    let creating = $state(false);
    let errorMessage = $state("");
    let searchQuery = $state("");
    let viewMode = $state<SearchViewMode>("list");
    let searchResultsIndex = $state(0);
    let activeTopbarMenu = $state<string | null>(null);
    let usernameDraft = $state("");
    let usernameSaving = $state(false);
    let usernameErrorMessage = $state("");

    const settingsMenuLabel = "Settings";

    $effect(() => {
        searchQuery;
        searchResultsIndex = 0;
    });

    $effect(() => {
        const currentUrl = page.url;
        const nextQuery = currentUrl.searchParams.get("q") ?? "";
        const nextViewMode = parseSearchViewMode(
            currentUrl.searchParams.get("v"),
        );
        untrack(() => {
            if (searchQuery !== nextQuery) {
                searchQuery = nextQuery;
            }

            if (viewMode !== nextViewMode) {
                viewMode = nextViewMode;
            }
        });
    });

    $effect(() => {
        if (!browser) {
            return;
        }

        const currentUrl = untrack(() => page.url);
        const nextUrl = new URL(currentUrl);

        if (searchQuery.trim().length > 0) {
            nextUrl.searchParams.set("q", searchQuery);
        } else {
            nextUrl.searchParams.delete("q");
        }

        if (viewMode !== "list") {
            nextUrl.searchParams.set("v", viewMode);
        } else {
            nextUrl.searchParams.delete("v");
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
            await goto("/new");
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to open new document page";
            creating = false;
        }
    }

    function getSearchResults() {
        return getDocumentSearchResults(documents, searchQuery);
    }

    function parseSearchViewMode(value: string | null): SearchViewMode {
        if (value === "tree") {
            return value;
        }

        return "list";
    }

    function getFolderPath(document: DocumentSummary) {
        return getFolderPathSegments(document.folderId, folders);
    }

    function getFolderName(folderId: string) {
        return (
            folders.find((folder) => folder.id === folderId)?.name ?? "folder"
        );
    }

    function handleSearchQueryChange(value: string) {
        searchQuery = value;
    }

    function handleViewModeChange(nextViewMode: SearchViewMode) {
        viewMode = nextViewMode;
    }

    function handleSearchResultHover(index: number) {
        searchResultsIndex = index;
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

    function handleSearchInputKeyDown(event: KeyboardEvent) {
        const results = getSearchResults();

        if (event.key === "Escape") {
            event.preventDefault();
            searchQuery = "";
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
            const target = results[searchResultsIndex]?.document;
            if (target) {
                void goto(`/d/${target.id}`);
            }
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
</script>

<svelte:head>
    <title>Doc-it | Search</title>
</svelte:head>

<svelte:document onpointerdown={handlePagePointerDown} />

<WorkspaceShell>
    {#snippet leftRail()}
        <div class="topbar-left">
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
                onclick={() => void goto("/explore")}
            >
                <span>Explore</span>
                <FolderOpen size={14} strokeWidth={2.2} />
            </button>
        </div>
    {/snippet}

    {#snippet stage()}
        {#snippet searchInputLeading()}
            <Search size={18} strokeWidth={2.1} />
        {/snippet}

        <div class="search-stage">
            <a
                href="/search"
                class="search-stage-brand"
                aria-label="Go to search page"
            >
                <span class="search-stage-brand__logo">
                    <DocItLogo fontSize="clamp(2.35rem, 5.2vw, 4.6rem)" />
                </span>
            </a>
            <DocumentSearchWorkspace
                query={searchQuery}
                results={getSearchResults()}
                {folders}
                {viewMode}
                selectedIndex={searchResultsIndex}
                {loading}
                {errorMessage}
                inputLeading={searchInputLeading}
                {getFolderPath}
                onQueryChange={handleSearchQueryChange}
                onKeyDown={handleSearchInputKeyDown}
                onViewModeChange={handleViewModeChange}
                onOpenResult={(target) => goto(`/d/${target.id}`)}
                onHoverResult={handleSearchResultHover}
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
    .search-stage-brand {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        justify-self: center;
        margin: 0 auto clamp(20px, 3vw, 28px);
        color: inherit;
        text-decoration: none;
    }

    .search-stage-brand:hover,
    .search-stage-brand:focus-visible {
        color: var(--text);
        outline: none;
    }

    .search-stage-brand__logo {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        color: var(--text);
    }

    .search-stage {
        padding-top: clamp(28px, 4vw, 42px);
        display: grid;
        justify-items: stretch;
    }
</style>
