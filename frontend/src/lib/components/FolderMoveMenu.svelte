<script lang="ts">
    import { onMount, tick } from "svelte";
    import {
        ChevronDown,
        ChevronRight,
        FolderPlus,
        Pencil,
        Search,
    } from "lucide-svelte";

    import FolderPathBadge from "$lib/components/FolderPathBadge.svelte";
    import type { FolderPathSegment } from "$lib/folders/path";
    import type { FolderSummary } from "$lib/types";

    type FolderTreeItem = {
        id: string;
        name: string;
        depth: number;
        parentFolderId: string | null;
        hasChildren: boolean;
    };

    let {
        folders,
        currentFolderId,
        currentPath = [],
        moving = false,
        onCreateSubfolder,
        onMove,
        onRenameFolder,
    } = $props<{
        folders: FolderSummary[];
        currentFolderId: string;
        currentPath?: FolderPathSegment[];
        moving?: boolean;
        onCreateSubfolder: (parentFolderId: string) => Promise<FolderSummary>;
        onMove: (folderId: string) => Promise<void>;
        onRenameFolder: (
            folderId: string,
            name: string,
        ) => Promise<FolderSummary>;
    }>();

    let rootElement = $state<HTMLDetailsElement | null>(null);
    let searchInputElement = $state<HTMLInputElement | null>(null);
    let renameInputElement = $state<HTMLInputElement | null>(null);
    let open = $state(false);
    let pendingFolderId = $state<string | null>(null);
    let errorMessage = $state("");
    let searchQuery = $state("");
    let expandedFolderIds = $state<Set<string>>(new Set());
    let hoveredFolderId = $state<string | null>(null);
    let editingFolderId = $state<string | null>(null);
    let editingValue = $state("");
    let savingFolderId = $state<string | null>(null);

    const treeItems = $derived.by(() => buildFolderTreeItems(folders));
    const parentFolderIds = $derived.by(
        () =>
            new Map(
                treeItems.map(
                    (item) =>
                        [item.id, item.parentFolderId] satisfies [
                            string,
                            string | null,
                        ],
                ),
            ),
    );

    const filteredTreeItems = $derived.by(() => {
        const query = searchQuery.trim().toLocaleLowerCase();
        if (!query) {
            return treeItems.filter((folder) => isFolderVisible(folder));
        }

        const matchingFolderIds = new Set(
            treeItems
                .filter((folder) =>
                    folder.name.toLocaleLowerCase().includes(query),
                )
                .map((folder) => folder.id),
        );

        const visibleFolderIds = new Set<string>();
        for (const folder of treeItems) {
            if (!matchingFolderIds.has(folder.id)) {
                continue;
            }

            let currentFolderId: string | null = folder.id;
            while (currentFolderId) {
                visibleFolderIds.add(currentFolderId);
                currentFolderId = parentFolderIds.get(currentFolderId) ?? null;
            }
        }

        return treeItems.filter((folder) => visibleFolderIds.has(folder.id));
    });

    $effect(() => {
        treeItems;
        currentPath;

        if (!open) {
            expandedFolderIds = getExpandedFolderIdsForCurrentPath();
        }
    });

    $effect(() => {
        editingFolderId;
        void tick().then(() => {
            if (editingFolderId) {
                renameInputElement?.focus();
                renameInputElement?.select();
            }
        });
    });

    onMount(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                if (editingFolderId) {
                    editingFolderId = null;
                    editingValue = "";
                    errorMessage = "";
                    return;
                }

                open = false;
            }
        };

        document.addEventListener("keydown", handleKeyDown);

        return () => {
            document.removeEventListener("keydown", handleKeyDown);
        };
    });

    async function handleMove(folderId: string) {
        if (
            folderId === currentFolderId ||
            moving ||
            pendingFolderId ||
            editingFolderId ||
            savingFolderId
        ) {
            return;
        }

        pendingFolderId = folderId;
        errorMessage = "";

        try {
            await onMove(folderId);
            open = false;
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to move document";
        } finally {
            pendingFolderId = null;
        }
    }

    function startRename(folderId: string, name: string) {
        editingFolderId = folderId;
        editingValue = name;
        errorMessage = "";
    }

    async function submitRename() {
        if (!editingFolderId || savingFolderId) {
            return;
        }

        const nextName = editingValue.trim();
        if (!nextName) {
            errorMessage = "Folder name cannot be empty";
            return;
        }

        savingFolderId = editingFolderId;
        errorMessage = "";

        try {
            await onRenameFolder(editingFolderId, nextName);
            editingFolderId = null;
            editingValue = "";
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to rename folder";
        } finally {
            savingFolderId = null;
        }
    }

    async function handleCreateNestedFolder(parentFolderId: string) {
        if (savingFolderId || pendingFolderId) {
            return;
        }

        savingFolderId = parentFolderId;
        errorMessage = "";

        try {
            const created = await onCreateSubfolder(parentFolderId);
            const nextExpanded = new Set(expandedFolderIds);
            nextExpanded.add(parentFolderId);
            expandedFolderIds = nextExpanded;
            editingFolderId = created.id;
            editingValue = created.name;
        } catch (error) {
            errorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to create folder";
        } finally {
            savingFolderId = null;
        }
    }

    function isFolderVisible(folder: FolderTreeItem) {
        let parentFolderId = folder.parentFolderId;

        while (parentFolderId) {
            if (!expandedFolderIds.has(parentFolderId)) {
                return false;
            }

            parentFolderId = parentFolderIds.get(parentFolderId) ?? null;
        }

        return true;
    }

    function toggleFolderExpansion(folderId: string) {
        const nextExpanded = new Set(expandedFolderIds);
        if (nextExpanded.has(folderId)) {
            nextExpanded.delete(folderId);
        } else {
            nextExpanded.add(folderId);
        }

        expandedFolderIds = nextExpanded;
    }

    function getExpandedFolderIdsForCurrentPath(): Set<string> {
        return new Set(
            currentPath.map((segment: FolderPathSegment) => segment.id),
        );
    }

    function buildFolderTreeItems(folderList: FolderSummary[]) {
        const items: FolderTreeItem[] = [];
        const childrenByParent = new Map<string | null, FolderSummary[]>();

        for (const folder of folderList) {
            const siblings = childrenByParent.get(folder.parentFolderId) ?? [];
            siblings.push(folder);
            childrenByParent.set(folder.parentFolderId, siblings);
        }

        for (const siblings of childrenByParent.values()) {
            siblings.sort((left, right) => left.name.localeCompare(right.name));
        }

        const visited = new Set<string>();

        const visit = (parentId: string | null, depth: number) => {
            for (const folder of childrenByParent.get(parentId) ?? []) {
                if (visited.has(folder.id)) {
                    continue;
                }

                visited.add(folder.id);
                items.push({
                    id: folder.id,
                    name: folder.name,
                    depth,
                    parentFolderId: folder.parentFolderId,
                    hasChildren:
                        (childrenByParent.get(folder.id)?.length ?? 0) > 0,
                });
                visit(folder.id, depth + 1);
            }
        };

        visit(null, 0);

        for (const folder of folderList) {
            if (visited.has(folder.id)) {
                continue;
            }

            items.push({
                id: folder.id,
                name: folder.name,
                depth: 0,
                parentFolderId: folder.parentFolderId,
                hasChildren: (childrenByParent.get(folder.id)?.length ?? 0) > 0,
            });
            visit(folder.id, 1);
        }

        return items;
    }
</script>

<details
    bind:this={rootElement}
    class="folder-move-menu"
    {open}
    ontoggle={(event) => {
        open = (event.currentTarget as HTMLDetailsElement).open;
        if (open) {
            expandedFolderIds = getExpandedFolderIdsForCurrentPath();
            void tick().then(() => {
                searchInputElement?.focus();
                searchInputElement?.select();
            });
        } else {
            errorMessage = "";
            searchQuery = "";
        }
    }}
>
    {#if open}
        <button
            type="button"
            class="folder-move-menu__backdrop"
            aria-label="Close folder menu"
            onclick={() => {
                open = false;
            }}
        ></button>
    {/if}

    <summary
        class="folder-move-menu__summary"
        aria-label="Move document to another folder"
        title="Move document to another folder"
    >
        <span class="folder-move-menu__trigger">
            <FolderPathBadge segments={currentPath} size="sm" />
            <ChevronDown size={12} strokeWidth={2.25} />
        </span>
    </summary>

    <div class="dropdown-panel folder-move-menu__panel">
        <label class="folder-move-menu__search" aria-label="Search folders">
            <Search size={14} strokeWidth={2.15} />
            <input
                bind:this={searchInputElement}
                class="folder-move-menu__search-input"
                type="text"
                value={searchQuery}
                placeholder="Search folders"
                spellcheck="false"
                autocomplete="off"
                oninput={(event) => {
                    searchQuery = (event.currentTarget as HTMLInputElement)
                        .value;
                }}
            />
        </label>

        {#if treeItems.length === 0}
            <p class="folder-move-menu__empty">No folders available</p>
        {:else if filteredTreeItems.length === 0}
            <p class="folder-move-menu__empty">No matching folders</p>
        {:else}
            <div class="folder-tree" role="tree" aria-label="Available folders">
                {#each filteredTreeItems as folder (folder.id)}
                    <div
                        role="presentation"
                        class="folder-tree__row"
                        style={`--folder-depth:${folder.depth};`}
                        onmouseenter={() => {
                            hoveredFolderId = folder.id;
                        }}
                        onmouseleave={() => {
                            if (hoveredFolderId === folder.id) {
                                hoveredFolderId = null;
                            }
                        }}
                    >
                        {#if folder.hasChildren}
                            <button
                                type="button"
                                class="folder-tree__toggle"
                                aria-label={expandedFolderIds.has(folder.id)
                                    ? `Collapse ${folder.name}`
                                    : `Expand ${folder.name}`}
                                aria-expanded={expandedFolderIds.has(folder.id)}
                                onclick={() => toggleFolderExpansion(folder.id)}
                            >
                                {#if expandedFolderIds.has(folder.id)}
                                    <ChevronDown size={12} strokeWidth={2.2} />
                                {:else}
                                    <ChevronRight size={12} strokeWidth={2.2} />
                                {/if}
                            </button>
                        {:else}
                            <span
                                class="folder-tree__toggle-spacer"
                                aria-hidden="true"
                            ></span>
                        {/if}

                        {#if editingFolderId === folder.id}
                            <div
                                role="presentation"
                                class:folder-tree__item--current={folder.id ===
                                    currentFolderId}
                                class="folder-tree__item folder-tree__item--editing"
                            >
                                <input
                                    bind:this={renameInputElement}
                                    class="folder-tree__rename-input"
                                    type="text"
                                    value={editingValue}
                                    spellcheck="false"
                                    maxlength="80"
                                    oninput={(event) => {
                                        editingValue = (
                                            event.currentTarget as HTMLInputElement
                                        ).value;
                                    }}
                                    onkeydown={(event) => {
                                        event.stopPropagation();
                                        if (event.key === "Enter") {
                                            event.preventDefault();
                                            void submitRename();
                                        }

                                        if (event.key === "Escape") {
                                            event.preventDefault();
                                            editingFolderId = null;
                                            editingValue = "";
                                            errorMessage = "";
                                        }
                                    }}
                                    onblur={() => {
                                        void submitRename();
                                    }}
                                />
                                {#if pendingFolderId === folder.id}
                                    <span class="folder-tree__meta"
                                        >Moving...</span
                                    >
                                {:else if folder.id === currentFolderId}
                                    <span class="folder-tree__meta"
                                        >Current</span
                                    >
                                {/if}
                            </div>
                        {:else}
                            <button
                                type="button"
                                role="treeitem"
                                aria-selected={folder.id === currentFolderId}
                                aria-expanded={folder.hasChildren
                                    ? expandedFolderIds.has(folder.id)
                                    : undefined}
                                class:folder-tree__item--current={folder.id ===
                                    currentFolderId}
                                class="folder-tree__item"
                                disabled={folder.id === currentFolderId ||
                                    moving ||
                                    pendingFolderId !== null ||
                                    editingFolderId !== null ||
                                    savingFolderId !== null}
                                onclick={() => void handleMove(folder.id)}
                            >
                                <span class="folder-tree__name"
                                    >{folder.name}</span
                                >
                                {#if pendingFolderId === folder.id}
                                    <span class="folder-tree__meta"
                                        >Moving...</span
                                    >
                                {:else if folder.id === currentFolderId}
                                    <span class="folder-tree__meta"
                                        >Current</span
                                    >
                                {/if}
                            </button>
                        {/if}

                        <div
                            class:folder-tree__actions--visible={hoveredFolderId ===
                                folder.id || editingFolderId === folder.id}
                            class="folder-tree__actions"
                        >
                            <button
                                type="button"
                                class:folder-tree__action--visible={hoveredFolderId ===
                                    folder.id || editingFolderId === folder.id}
                                class="folder-tree__action"
                                aria-label={`Rename ${folder.name}`}
                                title="Rename folder"
                                onclick={() =>
                                    startRename(folder.id, folder.name)}
                            >
                                <Pencil size={12} strokeWidth={2.1} />
                            </button>
                            <button
                                type="button"
                                class:folder-tree__action--visible={hoveredFolderId ===
                                    folder.id || editingFolderId === folder.id}
                                class="folder-tree__action"
                                aria-label={`Create subfolder in ${folder.name}`}
                                title="Create subfolder"
                                onclick={() =>
                                    void handleCreateNestedFolder(folder.id)}
                            >
                                <FolderPlus size={12} strokeWidth={2.1} />
                            </button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}

        {#if errorMessage}
            <p class="folder-move-menu__error">{errorMessage}</p>
        {/if}
    </div>
</details>

<style>
    .folder-move-menu {
        position: relative;
        display: inline-block;
        min-width: 0;
        max-width: 100%;
    }

    .folder-move-menu[open] {
        z-index: 40;
    }

    .folder-move-menu__backdrop {
        position: fixed;
        inset: 0;
        z-index: 39;
        padding: 0;
        border: 0;
        background: transparent;
        cursor: default;
    }

    .folder-move-menu__summary {
        position: relative;
        z-index: 41;
        list-style: none;
        cursor: pointer;
    }

    .folder-move-menu__summary::-webkit-details-marker {
        display: none;
    }

    .folder-move-menu__trigger {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        min-width: 0;
        max-width: 100%;
        color: var(--muted);
    }

    .folder-move-menu__trigger :global(svg) {
        transform: translateY(1px);
    }

    .folder-move-menu__trigger :global(.folder-path-badge) {
        background: transparent;
        transition:
            color 120ms ease,
            background 120ms ease;
    }

    .folder-move-menu[open]
        .folder-move-menu__trigger
        :global(.folder-path-badge),
    .folder-move-menu__summary:hover
        .folder-move-menu__trigger
        :global(.folder-path-badge) {
        background: transparent;
        color: var(--text);
    }

    .folder-move-menu__panel {
        z-index: 41;
        min-width: min(320px, calc(100vw - 32px));
        max-width: min(360px, calc(100vw - 32px));
        padding: 8px 0 6px;
        background: var(--dropdown-panel-bg, var(--panel));
        box-shadow: 0 18px 44px rgba(0, 0, 0, 0.14);
        backdrop-filter: blur(18px);
        isolation: isolate;
    }

    .folder-move-menu__search {
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 0 8px 6px;
        padding: 0 4px 6px;
        color: var(--text-soft);
        box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--line) 82%, transparent);
    }

    .folder-move-menu__search-input {
        width: 100%;
        min-width: 0;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--text);
        font: inherit;
        font-size: var(--presence-chip-font-size, 0.84rem);
        outline: none;
    }

    .folder-move-menu__search-input::placeholder {
        color: var(--text-soft);
    }

    .folder-tree {
        display: grid;
        gap: 1px;
        max-height: min(320px, 52vh);
        overflow: auto;
        padding: 0 6px;
    }

    .folder-tree__row {
        display: grid;
        grid-template-columns: 12px minmax(0, 1fr) auto;
        align-items: center;
        gap: 0;
        padding-left: calc(var(--folder-depth) * 13px);
    }

    .folder-tree__toggle,
    .folder-tree__toggle-spacer {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 12px;
        height: 12px;
        color: var(--text-soft);
    }

    .folder-tree__toggle {
        padding: 0;
        border: 0;
        border-radius: 6px;
        background: transparent;
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .folder-tree__toggle:hover,
    .folder-tree__toggle:focus-visible {
        background: var(--surface-overlay-medium);
        color: var(--text);
        outline: none;
    }

    .folder-tree__item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        width: 100%;
        min-width: 0;
        padding: 6px 10px;
        border: 0;
        border-radius: 8px;
        background: transparent;
        color: var(--text);
        font: inherit;
        font-size: var(--presence-chip-font-size, 0.84rem);
        line-height: 1.2;
        text-align: left;
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .folder-tree__rename-input {
        width: 100%;
        min-width: 0;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--text);
        font: inherit;
        font-size: inherit;
        line-height: inherit;
        outline: none;
    }

    .folder-tree__item:hover:not(:disabled),
    .folder-tree__item:focus-visible {
        background: var(--surface-overlay-medium);
        outline: none;
    }

    .folder-tree__item:disabled {
        cursor: default;
    }

    .folder-tree__item--current {
        color: var(--muted);
    }

    .folder-tree__name {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .folder-tree__meta {
        flex-shrink: 0;
        color: var(--text-soft);
        font-size: 0.68rem;
    }

    .folder-tree__actions {
        display: inline-flex;
        align-items: center;
        gap: 2px;
        width: 0;
        overflow: hidden;
        padding-right: 0;
        transition:
            width 120ms ease,
            padding-right 120ms ease;
    }

    .folder-tree__actions--visible {
        width: 48px;
        padding-right: 4px;
    }

    .folder-tree__action {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 22px;
        height: 22px;
        padding: 0;
        border: 0;
        border-radius: 6px;
        background: transparent;
        color: var(--text-soft);
        cursor: pointer;
        opacity: 0;
        pointer-events: none;
        transition:
            opacity 120ms ease,
            background 120ms ease,
            color 120ms ease;
    }

    .folder-tree__action--visible {
        opacity: 1;
        pointer-events: auto;
    }

    .folder-tree__action:hover,
    .folder-tree__action:focus-visible {
        background: var(--surface-overlay-medium);
        color: var(--text);
        outline: none;
    }

    .folder-move-menu__empty,
    .folder-move-menu__error {
        margin: 0;
        padding: 4px 12px 2px;
        font-size: 0.78rem;
    }

    .folder-move-menu__empty {
        color: var(--muted);
    }

    .folder-move-menu__error {
        color: var(--accent-strong);
    }
</style>
