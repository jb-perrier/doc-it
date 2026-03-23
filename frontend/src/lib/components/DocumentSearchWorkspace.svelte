<script lang="ts">
    import type { Snippet } from "svelte";
    import { onMount, tick } from "svelte";
    import {
        ChevronDown,
        ChevronLeft,
        ChevronRight,
        FolderOpen,
        FolderTree,
        List,
    } from "lucide-svelte";

    import FolderPathBadge from "$lib/components/FolderPathBadge.svelte";
    import {
        createFolderLookup,
        getFolderPathSegments,
        type FolderPathSegment,
    } from "$lib/folders/path";
    import type { DocumentSearchResult } from "$lib/search/documents";
    import type { FolderSummary } from "$lib/types";

    type SearchViewMode = "list" | "tree" | "explorer";

    type SearchTreeDocumentEntry = {
        kind: "document";
        key: string;
        result: DocumentSearchResult;
        depth: number;
        folderPath: FolderPathSegment[];
        folderKey: string | null;
        index: number;
    };

    type SearchTreeFolderNode = {
        kind: "folder";
        key: string;
        name: string;
        depth: number;
        parentKey: string | null;
        childFolderKeys: string[];
        documentEntries: SearchTreeDocumentEntry[];
        firstIndex: number;
    };

    type SearchTreeRow = SearchTreeFolderNode | SearchTreeDocumentEntry;

    type SearchTreeData = {
        rootFolderKeys: string[];
        rootDocuments: SearchTreeDocumentEntry[];
        folders: Map<string, SearchTreeFolderNode>;
        defaultExpandedKeys: Set<string>;
    };

    let {
        query,
        results,
        folders = [],
        selectedIndex,
        loading,
        errorMessage,
        emptyMessage = "No documents match that search.",
        placeholder = "Find another document",
        autoFocus = true,
        inputLeading,
        getFolderPath,
        onQueryChange,
        onKeyDown,
        onOpenResult,
        onHoverResult,
        onMoveResultToFolder,
    } = $props<{
        query: string;
        results: DocumentSearchResult[];
        folders?: FolderSummary[];
        selectedIndex: number;
        loading: boolean;
        errorMessage: string;
        emptyMessage?: string;
        placeholder?: string;
        autoFocus?: boolean;
        inputLeading?: Snippet;
        getFolderPath?: (
            document: DocumentSearchResult["document"],
        ) => FolderPathSegment[];
        onQueryChange: (value: string) => void;
        onKeyDown: (event: KeyboardEvent) => void;
        onOpenResult: (result: DocumentSearchResult["document"]) => void;
        onHoverResult?: (index: number) => void;
        onMoveResultToFolder?: (
            document: DocumentSearchResult["document"],
            folderId: string,
        ) => Promise<void>;
    }>();

    let inputElement = $state<HTMLInputElement | null>(null);
    let viewMode = $state<SearchViewMode>("list");
    let expandedFolderKeys = $state<Set<string>>(new Set());
    let currentExplorerFolderKey = $state<string | null>(null);
    let draggingDocumentId = $state<string | null>(null);
    let pendingMoveDocumentId = $state<string | null>(null);
    let dragTargetFolderKey = $state<string | null>(null);
    let moveErrorMessage = $state("");

    const hasSearchQuery = $derived(query.trim().length > 0);

    const selectedDocumentId = $derived(
        results[selectedIndex]?.document.id ?? null,
    );

    const searchTree = $derived.by(() =>
        buildSearchTree(results, folders, query),
    );

    const searchTreeRows = $derived.by(() =>
        getVisibleTreeRows(searchTree, expandedFolderKeys, query),
    );

    const explorerPath = $derived.by(() =>
        getExplorerPath(searchTree, currentExplorerFolderKey),
    );

    const explorerItems = $derived.by(() =>
        getExplorerItems(searchTree, currentExplorerFolderKey),
    );

    const hasVisibleItems = $derived(
        query.trim() ? results.length > 0 : searchTreeRows.length > 0,
    );

    const resultsLabel = $derived(
        query.trim()
            ? "Matching documents"
            : viewMode === "explorer"
              ? "Browse workspace"
              : "Workspace contents",
    );

    $effect(() => {
        searchTree;
        expandedFolderKeys = new Set(searchTree.defaultExpandedKeys);

        if (
            currentExplorerFolderKey &&
            !searchTree.folders.has(currentExplorerFolderKey)
        ) {
            currentExplorerFolderKey = null;
        }
    });

    $effect(() => {
        if (!hasSearchQuery) {
            return;
        }

        currentExplorerFolderKey = null;

        if (viewMode === "explorer") {
            viewMode = "list";
        }
    });

    onMount(() => {
        if (!autoFocus) {
            return;
        }

        void tick().then(() => {
            inputElement?.focus();
            inputElement?.select();
        });
    });

    function formatUpdatedAt(value: string) {
        return new Date(value).toLocaleString();
    }

    function toggleFolderExpansion(folderKey: string) {
        const nextExpanded = new Set(expandedFolderKeys);

        if (nextExpanded.has(folderKey)) {
            nextExpanded.delete(folderKey);
        } else {
            nextExpanded.add(folderKey);
        }

        expandedFolderKeys = nextExpanded;
    }

    function buildSearchTree(
        resultList: DocumentSearchResult[],
        folderList: FolderSummary[],
        searchQuery: string,
    ): SearchTreeData {
        const folders = new Map<string, SearchTreeFolderNode>();
        const rootFolderKeys: string[] = [];
        const rootDocuments: SearchTreeDocumentEntry[] = [];
        const hasSearchQuery = searchQuery.trim().length > 0;

        const ensureFolderPath = (
            folderPath: FolderPathSegment[],
            firstIndex = Number.MAX_SAFE_INTEGER,
        ) => {
            let parentKey: string | null = null;

            for (const [depth, segment] of folderPath.entries()) {
                let folder = folders.get(segment.id);

                if (!folder) {
                    folder = {
                        kind: "folder",
                        key: segment.id,
                        name: segment.name,
                        depth,
                        parentKey,
                        childFolderKeys: [],
                        documentEntries: [],
                        firstIndex,
                    };
                    folders.set(segment.id, folder);

                    if (parentKey) {
                        const parentFolder = folders.get(parentKey);
                        if (
                            parentFolder &&
                            !parentFolder.childFolderKeys.includes(segment.id)
                        ) {
                            parentFolder.childFolderKeys.push(segment.id);
                        }
                    } else if (!rootFolderKeys.includes(segment.id)) {
                        rootFolderKeys.push(segment.id);
                    }
                } else {
                    folder.firstIndex = Math.min(folder.firstIndex, firstIndex);
                }

                parentKey = segment.id;
            }

            return parentKey;
        };

        if (!hasSearchQuery && folderList.length > 0) {
            const folderLookup = createFolderLookup(folderList);

            for (const folder of folderList) {
                ensureFolderPath(
                    getFolderPathSegments(folder.id, folderLookup),
                );
            }
        }

        for (const [index, result] of resultList.entries()) {
            const folderPath = getFolderPath?.(result.document) ?? [];
            const parentKey = ensureFolderPath(folderPath, index);

            const entry: SearchTreeDocumentEntry = {
                kind: "document",
                key: result.document.id,
                result,
                depth: folderPath.length,
                folderPath,
                folderKey: parentKey,
                index,
            };

            if (parentKey) {
                folders.get(parentKey)?.documentEntries.push(entry);
            } else {
                rootDocuments.push(entry);
            }
        }

        return {
            rootFolderKeys,
            rootDocuments,
            folders,
            defaultExpandedKeys: new Set(folders.keys()),
        };
    }

    function getVisibleTreeRows(
        tree: SearchTreeData,
        expandedKeys: Set<string>,
        searchQuery: string,
    ): SearchTreeRow[] {
        const rows: SearchTreeRow[] = [];
        const hasSearchQuery = searchQuery.trim().length > 0;

        const appendFolder = (folderKey: string) => {
            const folder = tree.folders.get(folderKey);
            if (!folder) {
                return;
            }

            rows.push(folder);

            if (!expandedKeys.has(folderKey)) {
                return;
            }

            const children: Array<
                SearchTreeFolderNode | SearchTreeDocumentEntry
            > = [
                ...folder.childFolderKeys
                    .map((childKey) => tree.folders.get(childKey) ?? null)
                    .filter(
                        (child): child is SearchTreeFolderNode =>
                            child !== null,
                    ),
                ...folder.documentEntries,
            ];

            children
                .slice()
                .sort((left, right) =>
                    compareTreeItems(left, right, hasSearchQuery),
                )
                .forEach((child) => {
                    if (child.kind === "folder") {
                        appendFolder(child.key);
                        return;
                    }

                    rows.push(child);
                });
        };

        const rootItems: Array<SearchTreeFolderNode | SearchTreeDocumentEntry> =
            [
                ...tree.rootFolderKeys
                    .map((folderKey) => tree.folders.get(folderKey) ?? null)
                    .filter(
                        (item): item is SearchTreeFolderNode => item !== null,
                    ),
                ...tree.rootDocuments,
            ];

        rootItems
            .slice()
            .sort((left, right) =>
                compareTreeItems(left, right, hasSearchQuery),
            )
            .forEach((item) => {
                if (item.kind === "folder") {
                    appendFolder(item.key);
                    return;
                }

                rows.push(item);
            });

        return rows;
    }

    function getExplorerPath(
        tree: SearchTreeData,
        folderKey: string | null,
    ): SearchTreeFolderNode[] {
        if (!folderKey) {
            return [];
        }

        const path: SearchTreeFolderNode[] = [];
        let currentKey: string | null = folderKey;

        while (currentKey) {
            const folder = tree.folders.get(currentKey);
            if (!folder) {
                break;
            }

            path.unshift(folder);
            currentKey = folder.parentKey;
        }

        return path;
    }

    function getExplorerItems(
        tree: SearchTreeData,
        folderKey: string | null,
    ): SearchTreeRow[] {
        const items: Array<SearchTreeFolderNode | SearchTreeDocumentEntry> =
            folderKey
                ? [
                      ...(tree.folders
                          .get(folderKey)
                          ?.childFolderKeys.map(
                              (childKey) => tree.folders.get(childKey) ?? null,
                          ) ?? []),
                      ...(tree.folders.get(folderKey)?.documentEntries ?? []),
                  ].filter(
                      (
                          item,
                      ): item is
                          | SearchTreeFolderNode
                          | SearchTreeDocumentEntry => item !== null,
                  )
                : [
                      ...tree.rootFolderKeys
                          .map((rootKey) => tree.folders.get(rootKey) ?? null)
                          .filter(
                              (item): item is SearchTreeFolderNode =>
                                  item !== null,
                          ),
                      ...tree.rootDocuments,
                  ];

        return items
            .slice()
            .sort((left, right) => compareTreeItems(left, right, false));
    }

    function compareTreeItems(
        left: SearchTreeFolderNode | SearchTreeDocumentEntry,
        right: SearchTreeFolderNode | SearchTreeDocumentEntry,
        hasSearchQuery: boolean,
    ) {
        if (hasSearchQuery) {
            return getTreeItemOrder(left) - getTreeItemOrder(right);
        }

        if (left.kind !== right.kind) {
            return left.kind === "folder" ? -1 : 1;
        }

        return getTreeItemLabel(left).localeCompare(
            getTreeItemLabel(right),
            undefined,
            { sensitivity: "base" },
        );
    }

    function getTreeItemOrder(
        item: SearchTreeFolderNode | SearchTreeDocumentEntry,
    ) {
        return item.kind === "folder" ? item.firstIndex : item.index;
    }

    function getTreeItemLabel(
        item: SearchTreeFolderNode | SearchTreeDocumentEntry,
    ) {
        if (item.kind === "folder") {
            return item.name;
        }

        return item.result.document.title.trim() || "Untitled";
    }

    function getDraggedEntry() {
        if (!draggingDocumentId) {
            return null;
        }

        return (searchTreeRows.find(
            (candidate) =>
                candidate.kind === "document" &&
                candidate.result.document.id === draggingDocumentId,
        ) ?? null) as SearchTreeDocumentEntry | null;
    }

    function openExplorerFolder(folderKey: string) {
        currentExplorerFolderKey = folderKey;
    }

    function navigateExplorerUp() {
        if (!currentExplorerFolderKey) {
            return;
        }

        currentExplorerFolderKey =
            searchTree.folders.get(currentExplorerFolderKey)?.parentKey ?? null;
    }

    function getExplorerFolderMeta(folder: SearchTreeFolderNode) {
        const folderCount = folder.childFolderKeys.length;
        const documentCount = folder.documentEntries.length;
        const parts: string[] = [];

        if (folderCount > 0) {
            parts.push(`${folderCount} folder${folderCount === 1 ? "" : "s"}`);
        }

        if (documentCount > 0) {
            parts.push(`${documentCount} doc${documentCount === 1 ? "" : "s"}`);
        }

        return parts.join(" • ") || "Empty";
    }

    function handleDocumentDragStart(documentId: string) {
        if (!onMoveResultToFolder || pendingMoveDocumentId) {
            return;
        }

        draggingDocumentId = documentId;
        dragTargetFolderKey = null;
        moveErrorMessage = "";
    }

    function handleDocumentDragEnd() {
        draggingDocumentId = null;
        dragTargetFolderKey = null;
    }

    function handleFolderDragOver(event: DragEvent, folderKey: string) {
        const draggedEntry = getDraggedEntry();
        if (
            !draggedEntry ||
            pendingMoveDocumentId ||
            draggedEntry.folderKey === folderKey
        ) {
            return;
        }

        event.preventDefault();
        dragTargetFolderKey = folderKey;
    }

    function handleFolderDragLeave(folderKey: string) {
        if (dragTargetFolderKey === folderKey) {
            dragTargetFolderKey = null;
        }
    }

    async function handleFolderDrop(event: DragEvent, folderKey: string) {
        event.preventDefault();

        const draggedEntry = getDraggedEntry();
        if (
            !onMoveResultToFolder ||
            !draggedEntry ||
            pendingMoveDocumentId ||
            draggedEntry.folderKey === folderKey
        ) {
            dragTargetFolderKey = null;
            return;
        }

        pendingMoveDocumentId = draggedEntry.result.document.id;
        moveErrorMessage = "";

        try {
            await onMoveResultToFolder(draggedEntry.result.document, folderKey);
        } catch (error) {
            moveErrorMessage =
                error instanceof Error
                    ? error.message
                    : "Failed to move document";
        } finally {
            pendingMoveDocumentId = null;
            draggingDocumentId = null;
            dragTargetFolderKey = null;
        }
    }
</script>

<section class="search-workspace" aria-label="Search documents">
    <div class="search-workspace__inner">
        <div class="search-workspace__input-wrap">
            {#if inputLeading}
                {@render inputLeading()}
            {/if}
            <input
                bind:this={inputElement}
                class="search-workspace__input"
                type="text"
                value={query}
                {placeholder}
                spellcheck="false"
                autocomplete="off"
                oninput={(event) =>
                    onQueryChange(
                        (event.currentTarget as HTMLInputElement).value,
                    )}
                onkeydown={onKeyDown}
            />
        </div>

        {#if !loading && errorMessage}
            <p class="search-workspace__state search-workspace__state--error">
                {errorMessage}
            </p>
        {:else if !loading && !hasVisibleItems}
            <p class="search-workspace__state">{emptyMessage}</p>
        {:else if !loading}
            <div
                class:search-workspace__results--compact={!query.trim()}
                class:search-workspace__results--queried={query.trim()}
                class="search-workspace__results"
            >
                {#if moveErrorMessage}
                    <p
                        class="search-workspace__state search-workspace__state--error"
                    >
                        {moveErrorMessage}
                    </p>
                {/if}
                <div class="search-workspace__results-header">
                    <p class="search-workspace__label">{resultsLabel}</p>
                    <div
                        class="search-workspace__view-toggle"
                        role="group"
                        aria-label="Search result view"
                    >
                        <button
                            type="button"
                            class:search-workspace__view-button--active={viewMode ===
                                "list"}
                            class="search-workspace__view-button"
                            aria-pressed={viewMode === "list"}
                            aria-label="List view"
                            title="List view"
                            onclick={() => {
                                viewMode = "list";
                            }}
                        >
                            <List size={14} strokeWidth={2.2} />
                        </button>
                        <button
                            type="button"
                            class:search-workspace__view-button--active={viewMode ===
                                "tree"}
                            class="search-workspace__view-button"
                            aria-pressed={viewMode === "tree"}
                            aria-label="Tree view"
                            title="Tree view"
                            onclick={() => {
                                viewMode = "tree";
                            }}
                        >
                            <FolderTree size={14} strokeWidth={2.2} />
                        </button>
                        <button
                            type="button"
                            class:search-workspace__view-button--active={viewMode ===
                                "explorer"}
                            class="search-workspace__view-button"
                            aria-pressed={viewMode === "explorer"}
                            aria-label="Explorer view"
                            title={hasSearchQuery
                                ? "Explorer view is available when search is empty"
                                : "Explorer view"}
                            disabled={hasSearchQuery}
                            onclick={() => {
                                viewMode = "explorer";
                            }}
                        >
                            <FolderOpen size={14} strokeWidth={2.2} />
                        </button>
                    </div>
                </div>

                {#if viewMode === "list"}
                    {#each results as result, index (result.document.id)}
                        {@const folderPath =
                            getFolderPath?.(result.document) ?? []}
                        <button
                            type="button"
                            class:selected={index === selectedIndex}
                            class="search-result"
                            onclick={() => onOpenResult(result.document)}
                            onmousemove={() => onHoverResult?.(index)}
                        >
                            <span class="search-result__content">
                                <span class="search-result__title"
                                    >{result.document.title || "Untitled"}</span
                                >
                                <span class="search-result__meta">
                                    Updated {formatUpdatedAt(
                                        result.document.updatedAt,
                                    )}
                                </span>
                            </span>

                            {#if folderPath.length > 0}
                                <span class="search-result__folder-path">
                                    <FolderPathBadge segments={folderPath} />
                                </span>
                            {/if}
                        </button>
                    {/each}
                {:else if viewMode === "explorer"}
                    <div class="search-explorer" aria-label="Browse folders">
                        <div class="search-explorer__toolbar">
                            <button
                                type="button"
                                class="search-explorer__up-button"
                                disabled={!currentExplorerFolderKey}
                                onclick={navigateExplorerUp}
                            >
                                <ChevronLeft size={14} strokeWidth={2.2} />
                                <span>Up</span>
                            </button>

                            <div
                                class="search-explorer__breadcrumbs"
                                aria-label="Current folder"
                            >
                                <button
                                    type="button"
                                    class:search-explorer__crumb--active={!currentExplorerFolderKey}
                                    class="search-explorer__crumb"
                                    onclick={() => {
                                        currentExplorerFolderKey = null;
                                    }}
                                >
                                    Workspace
                                </button>

                                {#each explorerPath as folder (folder.key)}
                                    <span
                                        class="search-explorer__crumb-separator"
                                        aria-hidden="true">/</span
                                    >
                                    <button
                                        type="button"
                                        class:search-explorer__crumb--active={folder.key ===
                                            currentExplorerFolderKey}
                                        class="search-explorer__crumb"
                                        onclick={() => {
                                            currentExplorerFolderKey =
                                                folder.key;
                                        }}
                                    >
                                        {folder.name}
                                    </button>
                                {/each}
                            </div>
                        </div>

                        {#if explorerItems.length === 0}
                            <p class="search-workspace__state">
                                This folder is empty.
                            </p>
                        {:else}
                            <div class="search-explorer__list">
                                {#each explorerItems as item (item.kind === "folder" ? `folder:${item.key}` : `document:${item.key}`)}
                                    {#if item.kind === "folder"}
                                        <button
                                            type="button"
                                            class:search-explorer__item--drop-target={dragTargetFolderKey ===
                                                item.key}
                                            class="search-explorer__item search-explorer__item--folder"
                                            aria-label={`Open folder ${item.name}`}
                                            onclick={() =>
                                                openExplorerFolder(item.key)}
                                            ondragover={(event) =>
                                                handleFolderDragOver(
                                                    event,
                                                    item.key,
                                                )}
                                            ondragleave={() =>
                                                handleFolderDragLeave(item.key)}
                                            ondrop={(event) =>
                                                void handleFolderDrop(
                                                    event,
                                                    item.key,
                                                )}
                                        >
                                            <span
                                                class="search-explorer__item-content"
                                            >
                                                <span
                                                    class="search-explorer__item-title search-explorer__item-title--folder"
                                                >
                                                    {item.name}
                                                </span>
                                                <span
                                                    class="search-explorer__item-meta"
                                                >
                                                    {getExplorerFolderMeta(
                                                        item,
                                                    )}
                                                </span>
                                            </span>
                                            <span
                                                class="search-explorer__item-accessory"
                                            >
                                                <ChevronRight
                                                    size={14}
                                                    strokeWidth={2.2}
                                                />
                                            </span>
                                        </button>
                                    {:else}
                                        <button
                                            type="button"
                                            draggable={onMoveResultToFolder &&
                                                pendingMoveDocumentId === null}
                                            class:selected={item.result.document
                                                .id === selectedDocumentId}
                                            class:search-tree__document--dragging={item
                                                .result.document.id ===
                                                draggingDocumentId}
                                            class="search-explorer__item search-explorer__item--document"
                                            onclick={() =>
                                                onOpenResult(
                                                    item.result.document,
                                                )}
                                            onmousemove={() =>
                                                onHoverResult?.(item.index)}
                                            ondragstart={() =>
                                                handleDocumentDragStart(
                                                    item.result.document.id,
                                                )}
                                            ondragend={handleDocumentDragEnd}
                                        >
                                            <span
                                                class="search-explorer__item-content"
                                            >
                                                <span
                                                    class="search-explorer__item-title"
                                                >
                                                    {item.result.document
                                                        .title || "Untitled"}
                                                </span>
                                                <span
                                                    class="search-explorer__item-meta"
                                                >
                                                    {#if pendingMoveDocumentId === item.result.document.id}
                                                        Moving...
                                                    {:else}
                                                        Updated {formatUpdatedAt(
                                                            item.result.document
                                                                .updatedAt,
                                                        )}
                                                    {/if}
                                                </span>
                                            </span>
                                        </button>
                                    {/if}
                                {/each}
                            </div>
                        {/if}
                    </div>
                {:else}
                    <div
                        class="search-tree"
                        role="tree"
                        aria-label="Search results by folder"
                    >
                        {#each searchTreeRows as row (row.kind === "folder" ? `folder:${row.key}` : `document:${row.key}`)}
                            {#if row.kind === "folder"}
                                <div
                                    class="search-tree__row"
                                    style={`--tree-depth:${row.depth};`}
                                    role="presentation"
                                >
                                    {#if row.childFolderKeys.length > 0 || row.documentEntries.length > 0}
                                        <button
                                            type="button"
                                            class="search-tree__toggle"
                                            aria-label={expandedFolderKeys.has(
                                                row.key,
                                            )
                                                ? `Collapse ${row.name}`
                                                : `Expand ${row.name}`}
                                            aria-expanded={expandedFolderKeys.has(
                                                row.key,
                                            )}
                                            onclick={() =>
                                                toggleFolderExpansion(row.key)}
                                        >
                                            {#if expandedFolderKeys.has(row.key)}
                                                <ChevronDown
                                                    size={12}
                                                    strokeWidth={2.2}
                                                />
                                            {:else}
                                                <ChevronRight
                                                    size={12}
                                                    strokeWidth={2.2}
                                                />
                                            {/if}
                                        </button>
                                    {:else}
                                        <span
                                            class="search-tree__toggle-spacer"
                                            aria-hidden="true"
                                        ></span>
                                    {/if}

                                    <div
                                        class:search-tree__folder--drop-target={dragTargetFolderKey ===
                                            row.key}
                                        class="search-tree__folder"
                                        role="treeitem"
                                        tabindex="-1"
                                        aria-selected="false"
                                        aria-label={`Folder ${row.name}`}
                                        ondragover={(event) =>
                                            handleFolderDragOver(
                                                event,
                                                row.key,
                                            )}
                                        ondragleave={() =>
                                            handleFolderDragLeave(row.key)}
                                        ondrop={(event) =>
                                            void handleFolderDrop(
                                                event,
                                                row.key,
                                            )}
                                    >
                                        <span class="search-tree__folder-name"
                                            >{row.name}</span
                                        >
                                        <span class="search-tree__folder-meta">
                                            {row.documentEntries.length > 0
                                                ? `${row.documentEntries.length} doc${row.documentEntries.length === 1 ? "" : "s"}`
                                                : "Folder"}
                                        </span>
                                    </div>
                                </div>
                            {:else}
                                <div
                                    class="search-tree__row"
                                    style={`--tree-depth:${row.depth};`}
                                    role="presentation"
                                >
                                    <span
                                        class="search-tree__toggle-spacer"
                                        aria-hidden="true"
                                    ></span>
                                    <button
                                        type="button"
                                        role="treeitem"
                                        draggable={onMoveResultToFolder &&
                                            pendingMoveDocumentId === null}
                                        aria-selected={row.result.document
                                            .id === selectedDocumentId}
                                        class:selected={row.result.document
                                            .id === selectedDocumentId}
                                        class:search-tree__document--dragging={row
                                            .result.document.id ===
                                            draggingDocumentId}
                                        class="search-tree__document"
                                        onclick={() =>
                                            onOpenResult(row.result.document)}
                                        onmousemove={() =>
                                            onHoverResult?.(row.index)}
                                        ondragstart={() =>
                                            handleDocumentDragStart(
                                                row.result.document.id,
                                            )}
                                        ondragend={handleDocumentDragEnd}
                                    >
                                        <span
                                            class="search-tree__document-content"
                                        >
                                            <span
                                                class="search-tree__document-title"
                                            >
                                                {row.result.document.title ||
                                                    "Untitled"}
                                            </span>
                                            <span
                                                class="search-tree__document-meta"
                                            >
                                                {#if pendingMoveDocumentId === row.result.document.id}
                                                    Moving...
                                                {:else}
                                                    Updated {formatUpdatedAt(
                                                        row.result.document
                                                            .updatedAt,
                                                    )}
                                                {/if}
                                            </span>
                                        </span>
                                    </button>
                                </div>
                            {/if}
                        {/each}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</section>

<style>
    .search-workspace {
        padding: 24px clamp(18px, 4vw, 56px) 40px;
        min-height: 70vh;
    }

    .search-workspace__inner {
        max-width: 840px;
        margin: 0 auto;
        display: grid;
        gap: 18px;
    }

    .search-workspace__input-wrap {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        align-items: center;
        gap: 14px;
        padding: 0 0 18px;
        border-bottom: 1px solid var(--line);
        color: var(--muted);
    }

    .search-workspace__input {
        width: 100%;
        padding: 0;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--text);
        font-size: clamp(2rem, 4vw, 3.4rem);
        font-weight: 700;
        line-height: 1.02;
        letter-spacing: -0.05em;
    }

    .search-workspace__input::placeholder {
        color: var(--text-placeholder);
    }

    .search-workspace__results {
        display: grid;
        gap: 0;
    }

    .search-workspace__results--compact {
        margin-top: -18px;
    }

    .search-workspace__results--queried {
        margin-top: -18px;
    }

    .search-workspace__results-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 0 0 10px;
    }

    .search-workspace__label {
        margin: 0;
        font-size: 0.72rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--text-soft);
    }

    .search-workspace__view-toggle {
        display: inline-flex;
        align-items: center;
        gap: 2px;
    }

    .search-workspace__view-button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        padding: 0;
        border: 0;
        border-radius: 8px;
        background: transparent;
        color: var(--muted);
        cursor: pointer;
        transition: color 140ms ease;
    }

    .search-workspace__view-button:disabled {
        color: var(--text-placeholder);
        cursor: not-allowed;
    }

    .search-workspace__view-button:hover,
    .search-workspace__view-button:focus-visible {
        color: var(--text);
        outline: none;
    }

    .search-workspace__view-button--active {
        color: var(--text);
    }

    .search-workspace__state {
        margin: 0;
        padding: 4px 0 0;
        color: var(--muted);
        font-size: 0.98rem;
    }

    .search-workspace__state--error {
        color: var(--accent-strong);
    }

    .search-result {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        gap: 12px;
        width: 100%;
        padding: 16px 0;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: var(--text);
        text-align: left;
        cursor: pointer;
        transition: color 120ms ease;
    }

    .search-result:hover,
    .search-result.selected {
        color: var(--text);
    }

    .search-result__content {
        display: grid;
        gap: 4px;
        min-width: 0;
    }

    .search-result__title {
        font-weight: 500;
        font-size: 1.02rem;
    }

    .search-result__meta {
        font-size: 0.82rem;
        color: var(--muted);
    }

    .search-result__folder-path {
        display: inline-flex;
        justify-self: end;
        min-width: 0;
        max-width: min(100%, 22rem);
    }

    .search-explorer {
        display: grid;
        gap: 10px;
    }

    .search-explorer__toolbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding-bottom: 2px;
    }

    .search-explorer__up-button,
    .search-explorer__crumb {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--muted);
        cursor: pointer;
        transition: color 120ms ease;
    }

    .search-explorer__up-button:disabled {
        color: var(--text-placeholder);
        cursor: default;
    }

    .search-explorer__up-button:not(:disabled):hover,
    .search-explorer__up-button:not(:disabled):focus-visible,
    .search-explorer__crumb:hover,
    .search-explorer__crumb:focus-visible,
    .search-explorer__crumb--active {
        color: var(--text);
        outline: none;
    }

    .search-explorer__breadcrumbs {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 6px;
        min-width: 0;
        flex-wrap: wrap;
    }

    .search-explorer__crumb {
        min-width: 0;
    }

    .search-explorer__crumb-separator {
        color: var(--text-placeholder);
    }

    .search-explorer__list {
        display: grid;
        gap: 2px;
    }

    .search-explorer__item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        width: 100%;
        padding: 10px 0;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: var(--text);
        text-align: left;
        cursor: pointer;
        transition:
            color 120ms ease,
            background 120ms ease;
    }

    .search-explorer__item:hover,
    .search-explorer__item.selected {
        color: var(--text);
    }

    .search-explorer__item--drop-target {
        background: var(--surface-overlay-medium);
    }

    .search-explorer__item-content {
        display: grid;
        gap: 3px;
        min-width: 0;
    }

    .search-explorer__item-title {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 1rem;
        font-weight: 500;
    }

    .search-explorer__item-title--folder {
        color: color-mix(in srgb, var(--accent) 44%, var(--text));
    }

    .search-explorer__item-meta {
        color: var(--muted);
        font-size: 0.8rem;
    }

    .search-explorer__item-accessory {
        display: inline-flex;
        align-items: center;
        color: color-mix(in srgb, var(--accent) 38%, var(--text-soft));
    }

    .search-tree {
        display: grid;
        gap: 0;
    }

    .search-tree__row {
        display: grid;
        grid-template-columns: 16px minmax(0, 1fr);
        align-items: start;
        gap: 3px;
        padding-left: calc(var(--tree-depth, 0) * 18px);
    }

    .search-tree__toggle,
    .search-tree__toggle-spacer {
        width: 16px;
        height: 16px;
        margin-top: 8px;
        border-radius: 6px;
        flex-shrink: 0;
    }

    .search-tree__toggle {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 0;
        background: transparent;
        color: color-mix(in srgb, var(--accent) 42%, var(--text-soft));
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .search-tree__toggle:hover,
    .search-tree__toggle:focus-visible {
        background: color-mix(in srgb, var(--line) 50%, transparent);
        color: color-mix(in srgb, var(--accent-strong) 68%, var(--text));
        outline: none;
    }

    .search-tree__folder {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        width: 100%;
        min-width: 0;
        padding: 6px 12px 6px 8px;
        color: color-mix(in srgb, var(--accent) 26%, var(--muted));
        border-radius: 10px;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .search-tree__folder--drop-target {
        background: var(--surface-overlay-medium);
        color: color-mix(in srgb, var(--accent-strong) 46%, var(--text));
    }

    .search-tree__folder--drop-target .search-tree__folder-meta {
        color: var(--muted);
    }

    .search-tree__folder-name {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: color-mix(in srgb, var(--accent) 48%, var(--text));
        font-size: 0.94rem;
        font-weight: 500;
    }

    .search-tree__folder-meta {
        color: color-mix(in srgb, var(--accent) 58%, var(--muted));
        font-size: 0.74rem;
        letter-spacing: 0.04em;
        text-transform: uppercase;
        white-space: nowrap;
    }

    .search-tree__document {
        width: 100%;
        padding: 8px 12px;
        border: 0;
        border-radius: 12px;
        background: transparent;
        color: var(--text);
        text-align: left;
        cursor: pointer;
        transition:
            background 140ms ease,
            color 120ms ease;
    }

    .search-tree__document:hover,
    .search-tree__document.selected {
        background: color-mix(
            in srgb,
            var(--surface-elevated) 78%,
            transparent
        );
        color: var(--text);
    }

    .search-tree__document--dragging {
        opacity: 0.42;
    }

    .search-tree__document-content {
        display: grid;
        gap: 3px;
        min-width: 0;
    }

    .search-tree__document-title {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.98rem;
        font-weight: 500;
    }

    .search-tree__document-meta {
        color: var(--muted);
        font-size: 0.8rem;
    }

    @media (max-width: 1180px) {
        .search-workspace {
            padding-left: 0;
            padding-right: 0;
        }

        .search-workspace__inner {
            gap: 18px;
        }

        .search-workspace__input {
            font-size: clamp(1.6rem, 8vw, 2.4rem);
        }

        .search-workspace__results-header {
            flex-direction: column;
            align-items: flex-start;
        }

        .search-result {
            grid-template-columns: 1fr;
            justify-items: start;
        }

        .search-result__folder-path {
            justify-self: start;
            max-width: 100%;
        }

        .search-explorer__toolbar {
            flex-direction: column;
            align-items: flex-start;
        }

        .search-explorer__breadcrumbs {
            justify-content: flex-start;
        }

        .search-tree__folder {
            align-items: flex-start;
            flex-direction: column;
            gap: 4px;
        }
    }
</style>
