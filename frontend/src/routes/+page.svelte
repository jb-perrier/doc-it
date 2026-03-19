<script lang="ts">
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { Search, Sun, Moon, Plus } from "lucide-svelte";

    import { createDocument, listDocuments } from "$lib/api/documents";
    import DocumentSearchWorkspace from "$lib/components/DocumentSearchWorkspace.svelte";
    import ProfileSettingsMenu from "$lib/components/ProfileSettingsMenu.svelte";
    import WorkspaceShell from "$lib/components/WorkspaceShell.svelte";
    import { getDocumentSearchResults } from "$lib/search/documents";
    import {
        ensureSessionProfile,
        updateSessionProfileName,
    } from "$lib/stores/session";
    import { theme, toggleTheme } from "$lib/stores/theme";
    import type { DocumentSummary, SessionProfile } from "$lib/types";

    let documents = $state<DocumentSummary[]>([]);
    let session = $state<SessionProfile | null>(null);
    let loading = $state(true);
    let creating = $state(false);
    let errorMessage = $state("");
    let searchQuery = $state("");
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
            documents = await listDocuments();
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
        creating = true;
        try {
            const document = await createDocument("Untitled");
            await goto(`/d/${document.id}`);
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to create document";
            creating = false;
        }
    }

    function getSearchResults() {
        return getDocumentSearchResults(documents, searchQuery);
    }

    function handleSearchQueryChange(value: string) {
        searchQuery = value;
    }

    function handleSearchResultHover(index: number) {
        searchResultsIndex = index;
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
                <span>{creating ? "Creating..." : "New document"}</span>
                <Plus size={14} strokeWidth={2.2} />
            </button>
        </div>
    {/snippet}

    {#snippet stage()}
        {#snippet searchInputLeading()}
            <Search size={18} strokeWidth={2.1} />
        {/snippet}

        <DocumentSearchWorkspace
            query={searchQuery}
            results={getSearchResults()}
            selectedIndex={searchResultsIndex}
            {loading}
            {errorMessage}
            inputLeading={searchInputLeading}
            onQueryChange={handleSearchQueryChange}
            onKeyDown={handleSearchInputKeyDown}
            onOpenResult={(target) => goto(`/d/${target.id}`)}
            onHoverResult={handleSearchResultHover}
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
