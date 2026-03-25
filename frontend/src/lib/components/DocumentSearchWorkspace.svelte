<script lang="ts">
    import type { Snippet } from "svelte";
    import { onMount, tick, untrack } from "svelte";
    import {
        ChevronDown,
        ChevronLeft,
        ChevronRight,
        FolderPlus,
        FolderOpen,
        FolderTree,
        List,
        ListChevronsDownUp,
        ListChevronsUpDown,
        Pencil,
        Trash2,
    } from "lucide-svelte";

    import FolderPathBadge from "$lib/components/FolderPathBadge.svelte";
    import {
        createFolderLookup,
        getFolderPathSegments,
        type FolderPathSegment,
    } from "$lib/folders/path";
    import type { DocumentSearchResult } from "$lib/search/documents";
    import type { DocumentSummary, FolderSummary } from "$lib/types";

    type SearchViewMode = "list" | "tree" | "explorer";
    type WorkspaceMode = "search" | "create" | "explore";

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

    const ROOT_FOLDER_KEY = "workspace-root";
    let {
        mode = "search",
        query,
        results,
        folders = [],
        viewMode = "list",
        selectedIndex,
        loading,
        errorMessage,
        emptyMessage = "No documents match that search.",
        placeholder = "Find document",
        titleValue = "Untitled",
        titlePlaceholder = "Untitled",
        autoFocus = true,
        inputLeading,
        explorerToolbarAction,
        getFolderPath,
        initialExplorerFolderId = null,
        onQueryChange,
        onTitleChange,
        onKeyDown,
        onViewModeChange,
        onOpenResult,
        onExplorerFolderChange,
        onHoverResult,
        onMoveResultToFolder,
        onCreateSubfolder,
        onRenameFolder,
        onDeleteFolder,
        onRenameDocument,
        onDeleteDocument,
    } = $props<{
        mode?: WorkspaceMode;
        query: string;
        results: DocumentSearchResult[];
        folders?: FolderSummary[];
        viewMode?: SearchViewMode;
        selectedIndex: number;
        loading: boolean;
        errorMessage: string;
        emptyMessage?: string;
        placeholder?: string;
        titleValue?: string;
        titlePlaceholder?: string;
        autoFocus?: boolean;
        inputLeading?: Snippet;
        explorerToolbarAction?: Snippet;
        getFolderPath?: (
            document: DocumentSearchResult["document"],
        ) => FolderPathSegment[];
        initialExplorerFolderId?: string | null;
        onQueryChange?: (value: string) => void;
        onTitleChange?: (value: string) => void;
        onKeyDown?: (event: KeyboardEvent) => void;
        onViewModeChange?: (mode: SearchViewMode) => void;
        onOpenResult?: (result: DocumentSearchResult["document"]) => void;
        onExplorerFolderChange?: (folderId: string | null) => void;
        onHoverResult?: (index: number) => void;
        onMoveResultToFolder?: (
            document: DocumentSearchResult["document"],
            folderId: string,
        ) => Promise<void>;
        onCreateSubfolder?: (parentFolderId: string) => Promise<FolderSummary>;
        onRenameFolder?: (
            folderId: string,
            name: string,
        ) => Promise<FolderSummary>;
        onDeleteFolder?: (folderId: string) => Promise<void>;
        onRenameDocument?: (
            documentId: string,
            title: string,
        ) => Promise<DocumentSummary>;
        onDeleteDocument?: (documentId: string) => Promise<void>;
    }>();

    let inputElement = $state<HTMLInputElement | null>(null);
    let renameInputElement = $state<HTMLInputElement | null>(null);
    let activeViewMode = $state<SearchViewMode>("list");
    let expandedFolderKeys = $state<Set<string>>(new Set());
    let browseExpandedFolderKeys = $state<Set<string>>(new Set());
    let hadSearchQuery = $state(false);
    let currentExplorerFolderKey = $state<string | null>(null);
    let draggingDocumentId = $state<string | null>(null);
    let pendingMoveDocumentId = $state<string | null>(null);
    let dragTargetFolderKey = $state<string | null>(null);
    let moveErrorMessage = $state("");
    let hoveredFolderKey = $state<string | null>(null);
    let hoveredDocumentId = $state<string | null>(null);
    let editingFolderKey = $state<string | null>(null);
    let editingDocumentId = $state<string | null>(null);
    let editingValue = $state("");
    let savingFolderKey = $state<string | null>(null);
    let deletingFolderKey = $state<string | null>(null);
    let creatingFolderParentKey = $state<string | null>(null);
    let folderActionError = $state("");
    let documentActionError = $state("");
    let savingDocumentId = $state<string | null>(null);
    let deletingDocumentId = $state<string | null>(null);

    const isCreateMode = $derived(mode === "create");
    const isExploreMode = $derived(mode === "explore");
    const hasFixedExplorerMode = $derived(isCreateMode || isExploreMode);
    const hasSearchQuery = $derived(query.trim().length > 0);
    const hasExplorerToolbarAction = $derived(Boolean(explorerToolbarAction));
    const documentsAreInteractive = $derived(
        !isCreateMode && Boolean(onOpenResult),
    );

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
        isCreateMode
            ? "SELECT FOLDER"
            : activeViewMode === "explorer"
              ? "Explorer"
              : activeViewMode === "tree"
                ? "Tree"
                : "List",
    );

    $effect(() => {
        if (hasFixedExplorerMode) {
            if (activeViewMode !== "explorer") {
                activeViewMode = "explorer";
            }

            return;
        }

        if (activeViewMode !== viewMode) {
            activeViewMode = viewMode;
        }
    });

    $effect(() => {
        const nextExplorerFolderKey =
            initialExplorerFolderId &&
            searchTree.folders.has(initialExplorerFolderId)
                ? initialExplorerFolderId
                : null;
        const currentLocalExplorerFolderKey = untrack(
            () => currentExplorerFolderKey,
        );

        if (currentLocalExplorerFolderKey !== nextExplorerFolderKey) {
            currentExplorerFolderKey = nextExplorerFolderKey;
        }
    });

    $effect(() => {
        searchTree;

        const wasSearching = untrack(() => hadSearchQuery);
        const nextBrowseExpandedSource = untrack(
            () => browseExpandedFolderKeys,
        );

        if (hasSearchQuery) {
            expandedFolderKeys = new Set(searchTree.defaultExpandedKeys);
        } else {
            const nextBrowseExpanded = new Set(
                [...nextBrowseExpandedSource].filter((folderKey) =>
                    searchTree.folders.has(folderKey),
                ),
            );

            if (
                wasSearching ||
                nextBrowseExpanded.size !== nextBrowseExpandedSource.size
            ) {
                browseExpandedFolderKeys = nextBrowseExpanded;
            }

            expandedFolderKeys = new Set(nextBrowseExpanded);
        }

        hadSearchQuery = hasSearchQuery;

        if (editingFolderKey && !searchTree.folders.has(editingFolderKey)) {
            editingFolderKey = null;
            editingValue = "";
        }

        if (
            editingDocumentId &&
            !results.some(
                (result: DocumentSearchResult) =>
                    result.document.id === editingDocumentId,
            )
        ) {
            editingDocumentId = null;
            editingValue = "";
        }

        if (
            currentExplorerFolderKey &&
            !searchTree.folders.has(currentExplorerFolderKey)
        ) {
            currentExplorerFolderKey =
                hasFixedExplorerMode &&
                initialExplorerFolderId &&
                searchTree.folders.has(initialExplorerFolderId)
                    ? initialExplorerFolderId
                    : null;
        }
    });

    $effect(() => {
        onExplorerFolderChange?.(currentExplorerFolderKey);
    });

    $effect(() => {
        editingFolderKey;
        editingDocumentId;

        void tick().then(() => {
            if (editingFolderKey || editingDocumentId) {
                renameInputElement?.focus();
                renameInputElement?.select();
            }
        });
    });

    $effect(() => {
        if (!hasSearchQuery || hasFixedExplorerMode) {
            return;
        }

        if (activeViewMode === "explorer") {
            setViewMode("list");
        }
    });

    function setViewMode(nextViewMode: SearchViewMode) {
        if (hasFixedExplorerMode) {
            activeViewMode = "explorer";
            return;
        }

        if (activeViewMode === nextViewMode) {
            return;
        }

        activeViewMode = nextViewMode;
        onViewModeChange?.(nextViewMode);
    }

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

        if (!hasSearchQuery) {
            browseExpandedFolderKeys = new Set(nextExpanded);
        }
    }

    function collapseAllFolders() {
        if (expandedFolderKeys.size === 0) {
            return;
        }

        expandedFolderKeys = new Set();

        if (!hasSearchQuery) {
            browseExpandedFolderKeys = new Set();
        }
    }

    function expandAllFolders() {
        if (
            searchTree.folders.size === 0 ||
            expandedFolderKeys.size === searchTree.folders.size
        ) {
            return;
        }

        const nextExpanded = new Set(searchTree.folders.keys());

        expandedFolderKeys = nextExpanded;

        if (!hasSearchQuery) {
            browseExpandedFolderKeys = new Set(nextExpanded);
        }
    }

    function handleTreeFolderRowClick(
        event: MouseEvent,
        folder: SearchTreeFolderNode,
    ) {
        if (
            editingFolderKey ||
            editingDocumentId ||
            savingFolderKey ||
            savingDocumentId ||
            deletingFolderKey ||
            deletingDocumentId ||
            creatingFolderParentKey ||
            pendingMoveDocumentId
        ) {
            return;
        }

        const target = event.target;
        if (
            target instanceof Element &&
            target.closest(
                "button, input, textarea, select, option, a, label, summary",
            )
        ) {
            return;
        }

        if (
            folder.childFolderKeys.length === 0 &&
            folder.documentEntries.length === 0
        ) {
            return;
        }

        toggleFolderExpansion(folder.key);
    }

    function handleTreeFolderRowKeyDown(
        event: KeyboardEvent,
        folder: SearchTreeFolderNode,
    ) {
        if (event.key !== "Enter" && event.key !== " ") {
            return;
        }

        event.preventDefault();
        handleTreeFolderRowClick(event as unknown as MouseEvent, folder);
    }

    function canRenameFolder(folderKey: string) {
        return Boolean(onRenameFolder) && folderKey !== ROOT_FOLDER_KEY;
    }

    function canRenameDocument(documentId: string) {
        return !isCreateMode && Boolean(onRenameDocument);
    }

    function canCreateFolder(parentFolderKey: string) {
        return Boolean(onCreateSubfolder);
    }

    function canDeleteFolder(folderKey: string) {
        return Boolean(onDeleteFolder) && folderKey !== ROOT_FOLDER_KEY;
    }

    function canDeleteDocument(documentId: string) {
        return !isCreateMode && Boolean(onDeleteDocument);
    }

    function handleOpenResult(result: DocumentSearchResult["document"]) {
        if (!documentsAreInteractive) {
            return;
        }

        onOpenResult?.(result);
    }

    function showFolderActions(folderKey: string) {
        return hoveredFolderKey === folderKey || editingFolderKey === folderKey;
    }

    function showDocumentActions(documentId: string) {
        return (
            hoveredDocumentId === documentId || editingDocumentId === documentId
        );
    }

    function startFolderRename(folder: SearchTreeFolderNode) {
        if (
            !canRenameFolder(folder.key) ||
            savingFolderKey ||
            deletingFolderKey ||
            creatingFolderParentKey ||
            pendingMoveDocumentId ||
            savingDocumentId ||
            deletingDocumentId
        ) {
            return;
        }

        editingFolderKey = folder.key;
        editingDocumentId = null;
        editingValue = folder.name;
        folderActionError = "";
        documentActionError = "";
    }

    function cancelFolderRename() {
        editingFolderKey = null;
        editingValue = "";
        folderActionError = "";
    }

    function startDocumentRename(document: DocumentSearchResult["document"]) {
        if (
            !canRenameDocument(document.id) ||
            savingFolderKey ||
            deletingFolderKey ||
            creatingFolderParentKey ||
            pendingMoveDocumentId ||
            savingDocumentId ||
            deletingDocumentId
        ) {
            return;
        }

        editingDocumentId = document.id;
        editingFolderKey = null;
        editingValue = document.title;
        documentActionError = "";
        folderActionError = "";
    }

    function cancelDocumentRename() {
        editingDocumentId = null;
        editingValue = "";
        documentActionError = "";
    }

    async function submitFolderRename() {
        if (!editingFolderKey || !onRenameFolder || savingFolderKey) {
            return;
        }

        const nextName = editingValue.trim();
        if (!nextName) {
            folderActionError = "Folder name cannot be empty";
            return;
        }

        const folderKey = editingFolderKey;
        savingFolderKey = folderKey;
        folderActionError = "";
        documentActionError = "";

        try {
            await onRenameFolder(folderKey, nextName);
            editingFolderKey = null;
            editingValue = "";
        } catch (error) {
            folderActionError =
                error instanceof Error
                    ? error.message
                    : "Failed to rename folder";
        } finally {
            savingFolderKey = null;
        }
    }

    async function submitDocumentRename() {
        if (!editingDocumentId || !onRenameDocument || savingDocumentId) {
            return;
        }

        const nextTitle = editingValue.trim();
        if (!nextTitle) {
            documentActionError = "Document title cannot be empty";
            return;
        }

        const documentId = editingDocumentId;
        savingDocumentId = documentId;
        documentActionError = "";
        folderActionError = "";

        try {
            await onRenameDocument(documentId, nextTitle);
            editingDocumentId = null;
            editingValue = "";
        } catch (error) {
            documentActionError =
                error instanceof Error
                    ? error.message
                    : "Failed to rename document";
        } finally {
            savingDocumentId = null;
        }
    }

    async function handleDeleteFolder(folder: SearchTreeFolderNode) {
        if (
            !onDeleteFolder ||
            !canDeleteFolder(folder.key) ||
            savingFolderKey ||
            deletingFolderKey ||
            creatingFolderParentKey ||
            pendingMoveDocumentId ||
            savingDocumentId ||
            deletingDocumentId
        ) {
            return;
        }

        if (
            typeof window !== "undefined" &&
            !window.confirm(`Delete folder \"${folder.name}\"?`)
        ) {
            return;
        }

        deletingFolderKey = folder.key;
        folderActionError = "";

        try {
            await onDeleteFolder(folder.key);

            if (editingFolderKey === folder.key) {
                cancelFolderRename();
            }

            if (hoveredFolderKey === folder.key) {
                hoveredFolderKey = null;
            }
        } catch (error) {
            folderActionError =
                error instanceof Error
                    ? error.message
                    : "Failed to delete folder";
        } finally {
            deletingFolderKey = null;
        }
    }

    async function handleDeleteDocumentRow(
        document: DocumentSearchResult["document"],
    ) {
        if (
            !onDeleteDocument ||
            !canDeleteDocument(document.id) ||
            savingFolderKey ||
            deletingFolderKey ||
            creatingFolderParentKey ||
            pendingMoveDocumentId ||
            savingDocumentId ||
            deletingDocumentId
        ) {
            return;
        }

        if (
            typeof window !== "undefined" &&
            !window.confirm(
                `Delete document \"${document.title || "Untitled"}\"? This action cannot be undone.`,
            )
        ) {
            return;
        }

        deletingDocumentId = document.id;
        documentActionError = "";
        folderActionError = "";

        try {
            await onDeleteDocument(document.id);

            if (editingDocumentId === document.id) {
                cancelDocumentRename();
            }

            if (hoveredDocumentId === document.id) {
                hoveredDocumentId = null;
            }
        } catch (error) {
            documentActionError =
                error instanceof Error
                    ? error.message
                    : "Failed to delete document";
        } finally {
            deletingDocumentId = null;
        }
    }

    async function handleCreateFolder(parentFolderKey: string) {
        if (
            !onCreateSubfolder ||
            !canCreateFolder(parentFolderKey) ||
            savingFolderKey ||
            deletingFolderKey ||
            creatingFolderParentKey ||
            pendingMoveDocumentId ||
            savingDocumentId ||
            deletingDocumentId
        ) {
            return;
        }

        creatingFolderParentKey = parentFolderKey;
        folderActionError = "";

        try {
            const created = await onCreateSubfolder(parentFolderKey);

            if (parentFolderKey !== ROOT_FOLDER_KEY) {
                const nextExpanded = new Set(expandedFolderKeys);
                nextExpanded.add(parentFolderKey);
                expandedFolderKeys = nextExpanded;

                if (!hasSearchQuery) {
                    browseExpandedFolderKeys = new Set(nextExpanded);
                }
            }

            editingFolderKey = created.id;
            editingValue = created.name;
            hoveredFolderKey = created.id;
        } catch (error) {
            folderActionError =
                error instanceof Error
                    ? error.message
                    : "Failed to create folder";
        } finally {
            creatingFolderParentKey = null;
        }
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
            defaultExpandedKeys: hasSearchQuery
                ? new Set(folders.keys())
                : new Set(),
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

    function getFolderMeta(folder: SearchTreeFolderNode) {
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
        if (
            !onMoveResultToFolder ||
            pendingMoveDocumentId ||
            editingFolderKey ||
            editingDocumentId ||
            savingFolderKey ||
            savingDocumentId ||
            creatingFolderParentKey ||
            deletingFolderKey ||
            deletingDocumentId
        ) {
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
            editingFolderKey ||
            editingDocumentId ||
            savingFolderKey ||
            savingDocumentId ||
            creatingFolderParentKey ||
            deletingFolderKey ||
            deletingDocumentId ||
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
            editingFolderKey ||
            editingDocumentId ||
            savingFolderKey ||
            savingDocumentId ||
            creatingFolderParentKey ||
            deletingFolderKey ||
            deletingDocumentId ||
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

<section
    class="search-workspace"
    aria-label={isCreateMode
        ? "Choose destination folder"
        : isExploreMode
          ? "Browse documents"
          : "Search documents"}
>
    <div class="search-workspace__inner">
        {#if !isExploreMode}
            <div
                class:search-workspace__input-wrap--create={isCreateMode}
                class="search-workspace__input-wrap"
            >
                {#if inputLeading}
                    {@render inputLeading()}
                {/if}
                {#if isCreateMode}
                    <input
                        bind:this={inputElement}
                        class:search-workspace__input--create={isCreateMode}
                        class="search-workspace__input"
                        type="text"
                        value={titleValue}
                        placeholder={titlePlaceholder}
                        spellcheck="false"
                        autocomplete="off"
                        oninput={(event) =>
                            onTitleChange?.(
                                (event.currentTarget as HTMLInputElement).value,
                            )}
                        onkeydown={onKeyDown}
                    />
                {:else}
                    <input
                        bind:this={inputElement}
                        class="search-workspace__input"
                        type="text"
                        value={query}
                        {placeholder}
                        spellcheck="false"
                        autocomplete="off"
                        oninput={(event) =>
                            onQueryChange?.(
                                (event.currentTarget as HTMLInputElement).value,
                            )}
                        onkeydown={onKeyDown}
                    />
                {/if}
            </div>
        {/if}

        {#if !loading && errorMessage}
            <p class="search-workspace__state search-workspace__state--error">
                {errorMessage}
            </p>
        {:else if !loading && !hasVisibleItems}
            <p class="search-workspace__state">{emptyMessage}</p>
        {:else if !loading}
            <div
                class:search-workspace__results--compact={!query.trim()}
                class:search-workspace__results--list={activeViewMode ===
                    "list"}
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
                {#if folderActionError}
                    <p
                        class="search-workspace__state search-workspace__state--error"
                    >
                        {folderActionError}
                    </p>
                {/if}
                {#if documentActionError}
                    <p
                        class="search-workspace__state search-workspace__state--error"
                    >
                        {documentActionError}
                    </p>
                {/if}
                <div class="search-workspace__results-header">
                    <p class="search-workspace__label">{resultsLabel}</p>
                    {#if !isCreateMode && !isExploreMode}
                        <div class="search-workspace__results-header-actions">
                            <div
                                class="search-workspace__view-toggle"
                                role="group"
                                aria-label="Search result view"
                            >
                                <button
                                    type="button"
                                    class:search-workspace__view-button--active={activeViewMode ===
                                        "list"}
                                    class="search-workspace__view-button"
                                    aria-pressed={activeViewMode === "list"}
                                    aria-label="List view"
                                    title="List view"
                                    onclick={() => {
                                        setViewMode("list");
                                    }}
                                >
                                    <List size={14} strokeWidth={2.2} />
                                </button>
                                <button
                                    type="button"
                                    class:search-workspace__view-button--active={activeViewMode ===
                                        "tree"}
                                    class="search-workspace__view-button"
                                    aria-pressed={activeViewMode === "tree"}
                                    aria-label="Tree view"
                                    title="Tree view"
                                    onclick={() => {
                                        setViewMode("tree");
                                    }}
                                >
                                    <FolderTree size={14} strokeWidth={2.2} />
                                </button>
                            </div>
                        </div>
                    {/if}
                </div>

                {#if activeViewMode === "list"}
                    {#each results as result, index (result.document.id)}
                        {@const folderPath =
                            getFolderPath?.(result.document) ?? []}
                        <div
                            class="search-result-row"
                            role="presentation"
                            onmouseenter={() => {
                                hoveredDocumentId = result.document.id;
                            }}
                            onmouseleave={() => {
                                if (hoveredDocumentId === result.document.id) {
                                    hoveredDocumentId = null;
                                }
                            }}
                        >
                            {#if editingDocumentId === result.document.id}
                                <div
                                    class:selected={index === selectedIndex}
                                    class:search-result--hovered={hoveredDocumentId ===
                                        result.document.id}
                                    class="search-result search-result--editing"
                                >
                                    <span class="search-result__content">
                                        <input
                                            bind:this={renameInputElement}
                                            class="search-folder-management__input search-folder-management__input--document"
                                            type="text"
                                            value={editingValue}
                                            spellcheck="false"
                                            maxlength="160"
                                            oninput={(event) => {
                                                editingValue = (
                                                    event.currentTarget as HTMLInputElement
                                                ).value;
                                            }}
                                            onkeydown={(event) => {
                                                event.stopPropagation();
                                                if (event.key === "Enter") {
                                                    event.preventDefault();
                                                    void submitDocumentRename();
                                                }

                                                if (event.key === "Escape") {
                                                    event.preventDefault();
                                                    cancelDocumentRename();
                                                }
                                            }}
                                            onblur={() => {
                                                void submitDocumentRename();
                                            }}
                                        />
                                        <span class="search-result__meta">
                                            Updated {formatUpdatedAt(
                                                result.document.updatedAt,
                                            )}
                                        </span>
                                    </span>

                                    {#if folderPath.length > 0}
                                        <span
                                            class="search-result__folder-path"
                                        >
                                            <FolderPathBadge
                                                segments={folderPath}
                                            />
                                        </span>
                                    {/if}
                                </div>
                            {:else}
                                <button
                                    type="button"
                                    class:selected={index === selectedIndex}
                                    class:search-result--hovered={hoveredDocumentId ===
                                        result.document.id}
                                    class="search-result"
                                    disabled={!documentsAreInteractive}
                                    onclick={() =>
                                        handleOpenResult(result.document)}
                                    onmousemove={() => {
                                        onHoverResult?.(index);
                                    }}
                                >
                                    <span class="search-result__content">
                                        <span class="search-result__title"
                                            >{result.document.title ||
                                                "Untitled"}</span
                                        >
                                        <span class="search-result__meta">
                                            Updated {formatUpdatedAt(
                                                result.document.updatedAt,
                                            )}
                                        </span>
                                    </span>

                                    {#if folderPath.length > 0}
                                        <span
                                            class="search-result__folder-path"
                                        >
                                            <FolderPathBadge
                                                segments={folderPath}
                                            />
                                        </span>
                                    {/if}
                                </button>
                            {/if}

                            {#if canRenameDocument(result.document.id) || canDeleteDocument(result.document.id)}
                                <div
                                    class:search-folder-management__actions--visible={showDocumentActions(
                                        result.document.id,
                                    )}
                                    class="search-folder-management__actions"
                                >
                                    {#if canRenameDocument(result.document.id)}
                                        <button
                                            type="button"
                                            class="search-folder-management__action"
                                            aria-label={`Rename ${result.document.title || "Untitled"}`}
                                            title="Rename document"
                                            disabled={savingFolderKey !==
                                                null ||
                                                deletingFolderKey !== null ||
                                                savingDocumentId !== null ||
                                                deletingDocumentId !== null}
                                            onclick={() =>
                                                startDocumentRename(
                                                    result.document,
                                                )}
                                        >
                                            <Pencil
                                                size={12}
                                                strokeWidth={2.1}
                                            />
                                        </button>
                                    {/if}
                                    {#if canDeleteDocument(result.document.id)}
                                        <button
                                            type="button"
                                            class="search-folder-management__action search-folder-management__action--danger"
                                            aria-label={`Delete ${result.document.title || "Untitled"}`}
                                            title="Delete document"
                                            disabled={savingFolderKey !==
                                                null ||
                                                deletingFolderKey !== null ||
                                                savingDocumentId !== null ||
                                                deletingDocumentId !== null}
                                            onclick={() =>
                                                void handleDeleteDocumentRow(
                                                    result.document,
                                                )}
                                        >
                                            <Trash2
                                                size={12}
                                                strokeWidth={2.1}
                                            />
                                        </button>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                    {/each}
                {:else if activeViewMode === "explorer"}
                    <div class="search-explorer" aria-label="Browse folders">
                        <div
                            class:search-explorer__toolbar--with-action={hasExplorerToolbarAction}
                            class="search-explorer__toolbar"
                        >
                            <div class="search-explorer__toolbar-main">
                                <div class="search-explorer__toolbar-start">
                                    <button
                                        type="button"
                                        class="search-workspace__toolbar-action search-workspace__toolbar-action--icon"
                                        disabled={!currentExplorerFolderKey}
                                        aria-label="Up"
                                        title="Up"
                                        onclick={navigateExplorerUp}
                                    >
                                        <ChevronLeft
                                            size={14}
                                            strokeWidth={2.2}
                                        />
                                    </button>

                                    {#if canCreateFolder(currentExplorerFolderKey ?? ROOT_FOLDER_KEY)}
                                        <button
                                            type="button"
                                            class="search-workspace__toolbar-action search-workspace__toolbar-action--subtle search-workspace__toolbar-action--icon"
                                            disabled={savingFolderKey !==
                                                null ||
                                                deletingFolderKey !== null ||
                                                creatingFolderParentKey !==
                                                    null}
                                            aria-label="Create folder in current location"
                                            title="New folder"
                                            onclick={() =>
                                                void handleCreateFolder(
                                                    currentExplorerFolderKey ??
                                                        ROOT_FOLDER_KEY,
                                                )}
                                        >
                                            <FolderPlus
                                                size={14}
                                                strokeWidth={2.1}
                                            />
                                        </button>
                                    {/if}
                                </div>

                                <div
                                    class="search-explorer__breadcrumbs"
                                    aria-label="Current folder"
                                >
                                    {#each explorerPath as folder (folder.key)}
                                        {#if folder.key !== explorerPath[0]?.key}
                                            <span
                                                class="search-explorer__crumb-separator"
                                                aria-hidden="true">/</span
                                            >
                                        {/if}
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

                            {#if explorerToolbarAction}
                                <div class="search-explorer__toolbar-end">
                                    {@render explorerToolbarAction()}
                                </div>
                            {/if}
                        </div>

                        {#if explorerItems.length === 0}
                            <p class="search-workspace__state">
                                This folder is empty.
                            </p>
                        {:else}
                            <div class="search-explorer__list">
                                {#each explorerItems as item (item.kind === "folder" ? `folder:${item.key}` : `document:${item.key}`)}
                                    {#if item.kind === "folder"}
                                        <div
                                            class:search-explorer__entry--drop-target={dragTargetFolderKey ===
                                                item.key}
                                            class="search-explorer__entry"
                                            role="presentation"
                                            onmouseenter={() => {
                                                hoveredFolderKey = item.key;
                                            }}
                                            onmouseleave={() => {
                                                if (
                                                    hoveredFolderKey ===
                                                    item.key
                                                ) {
                                                    hoveredFolderKey = null;
                                                }
                                            }}
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
                                            {#if editingFolderKey === item.key}
                                                <div
                                                    class="search-explorer__item search-explorer__item--folder search-explorer__item--editing"
                                                >
                                                    <span
                                                        class="search-explorer__item-content"
                                                    >
                                                        <input
                                                            bind:this={
                                                                renameInputElement
                                                            }
                                                            class="search-folder-management__input"
                                                            type="text"
                                                            value={editingValue}
                                                            spellcheck="false"
                                                            maxlength="80"
                                                            oninput={(
                                                                event,
                                                            ) => {
                                                                editingValue = (
                                                                    event.currentTarget as HTMLInputElement
                                                                ).value;
                                                            }}
                                                            onkeydown={(
                                                                event,
                                                            ) => {
                                                                event.stopPropagation();
                                                                if (
                                                                    event.key ===
                                                                    "Enter"
                                                                ) {
                                                                    event.preventDefault();
                                                                    void submitFolderRename();
                                                                }

                                                                if (
                                                                    event.key ===
                                                                    "Escape"
                                                                ) {
                                                                    event.preventDefault();
                                                                    cancelFolderRename();
                                                                }
                                                            }}
                                                            onblur={() => {
                                                                void submitFolderRename();
                                                            }}
                                                        />
                                                        <span
                                                            class="search-explorer__item-meta"
                                                        >
                                                            {getFolderMeta(
                                                                item,
                                                            )}
                                                        </span>
                                                    </span>
                                                </div>
                                            {:else}
                                                <button
                                                    type="button"
                                                    class="search-explorer__item search-explorer__item--folder"
                                                    aria-label={`Open folder ${item.name}`}
                                                    disabled={Boolean(
                                                        editingFolderKey,
                                                    ) ||
                                                        savingFolderKey !==
                                                            null ||
                                                        deletingFolderKey !==
                                                            null}
                                                    onclick={() =>
                                                        openExplorerFolder(
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
                                                            {getFolderMeta(
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
                                            {/if}

                                            {#if canCreateFolder(item.key) || canRenameFolder(item.key) || canDeleteFolder(item.key)}
                                                <div
                                                    class:search-folder-management__actions--visible={showFolderActions(
                                                        item.key,
                                                    )}
                                                    class="search-folder-management__actions"
                                                >
                                                    {#if canCreateFolder(item.key)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action"
                                                            aria-label={`Create subfolder in ${item.name}`}
                                                            title="Create subfolder"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null ||
                                                                creatingFolderParentKey !==
                                                                    null}
                                                            onclick={() =>
                                                                void handleCreateFolder(
                                                                    item.key,
                                                                )}
                                                        >
                                                            <FolderPlus
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                    {#if canRenameFolder(item.key)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action"
                                                            aria-label={`Rename ${item.name}`}
                                                            title="Rename folder"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null}
                                                            onclick={() =>
                                                                startFolderRename(
                                                                    item,
                                                                )}
                                                        >
                                                            <Pencil
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                    {#if canDeleteFolder(item.key)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action search-folder-management__action--danger"
                                                            aria-label={`Delete ${item.name}`}
                                                            title="Delete folder"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null}
                                                            onclick={() =>
                                                                void handleDeleteFolder(
                                                                    item,
                                                                )}
                                                        >
                                                            <Trash2
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                </div>
                                            {/if}
                                        </div>
                                    {:else}
                                        <div
                                            class="search-explorer__entry"
                                            role="presentation"
                                            onmouseenter={() => {
                                                hoveredDocumentId =
                                                    item.result.document.id;
                                            }}
                                            onmouseleave={() => {
                                                if (
                                                    hoveredDocumentId ===
                                                    item.result.document.id
                                                ) {
                                                    hoveredDocumentId = null;
                                                }
                                            }}
                                        >
                                            {#if editingDocumentId === item.result.document.id}
                                                <div
                                                    class:selected={item.result
                                                        .document.id ===
                                                        selectedDocumentId}
                                                    class:search-explorer__item--hovered={hoveredDocumentId ===
                                                        item.result.document.id}
                                                    class="search-explorer__item search-explorer__item--document search-explorer__item--editing"
                                                >
                                                    <span
                                                        class="search-explorer__item-content"
                                                    >
                                                        <input
                                                            bind:this={
                                                                renameInputElement
                                                            }
                                                            class="search-folder-management__input search-folder-management__input--document"
                                                            type="text"
                                                            value={editingValue}
                                                            spellcheck="false"
                                                            maxlength="160"
                                                            oninput={(
                                                                event,
                                                            ) => {
                                                                editingValue = (
                                                                    event.currentTarget as HTMLInputElement
                                                                ).value;
                                                            }}
                                                            onkeydown={(
                                                                event,
                                                            ) => {
                                                                event.stopPropagation();
                                                                if (
                                                                    event.key ===
                                                                    "Enter"
                                                                ) {
                                                                    event.preventDefault();
                                                                    void submitDocumentRename();
                                                                }

                                                                if (
                                                                    event.key ===
                                                                    "Escape"
                                                                ) {
                                                                    event.preventDefault();
                                                                    cancelDocumentRename();
                                                                }
                                                            }}
                                                            onblur={() => {
                                                                void submitDocumentRename();
                                                            }}
                                                        />
                                                        <span
                                                            class="search-explorer__item-meta"
                                                        >
                                                            {#if pendingMoveDocumentId === item.result.document.id}
                                                                Moving...
                                                            {:else}
                                                                Updated {formatUpdatedAt(
                                                                    item.result
                                                                        .document
                                                                        .updatedAt,
                                                                )}
                                                            {/if}
                                                        </span>
                                                    </span>
                                                </div>
                                            {:else}
                                                <button
                                                    type="button"
                                                    draggable={documentsAreInteractive &&
                                                        onMoveResultToFolder &&
                                                        pendingMoveDocumentId ===
                                                            null}
                                                    class:selected={item.result
                                                        .document.id ===
                                                        selectedDocumentId}
                                                    class:search-explorer__item--hovered={hoveredDocumentId ===
                                                        item.result.document.id}
                                                    class:search-tree__document--dragging={item
                                                        .result.document.id ===
                                                        draggingDocumentId}
                                                    class="search-explorer__item search-explorer__item--document"
                                                    disabled={!documentsAreInteractive}
                                                    onclick={() =>
                                                        handleOpenResult(
                                                            item.result
                                                                .document,
                                                        )}
                                                    onmousemove={() => {
                                                        onHoverResult?.(
                                                            item.index,
                                                        );
                                                    }}
                                                    ondragstart={() =>
                                                        handleDocumentDragStart(
                                                            item.result.document
                                                                .id,
                                                        )}
                                                    ondragend={handleDocumentDragEnd}
                                                >
                                                    <span
                                                        class="search-explorer__item-content"
                                                    >
                                                        <span
                                                            class="search-explorer__item-title"
                                                        >
                                                            {item.result
                                                                .document
                                                                .title ||
                                                                "Untitled"}
                                                        </span>
                                                        <span
                                                            class="search-explorer__item-meta"
                                                        >
                                                            {#if pendingMoveDocumentId === item.result.document.id}
                                                                Moving...
                                                            {:else}
                                                                Updated {formatUpdatedAt(
                                                                    item.result
                                                                        .document
                                                                        .updatedAt,
                                                                )}
                                                            {/if}
                                                        </span>
                                                    </span>
                                                </button>
                                            {/if}

                                            {#if canRenameDocument(item.result.document.id) || canDeleteDocument(item.result.document.id)}
                                                <div
                                                    class:search-folder-management__actions--visible={showDocumentActions(
                                                        item.result.document.id,
                                                    )}
                                                    class="search-folder-management__actions"
                                                >
                                                    {#if canRenameDocument(item.result.document.id)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action"
                                                            aria-label={`Rename ${item.result.document.title || "Untitled"}`}
                                                            title="Rename document"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null ||
                                                                savingDocumentId !==
                                                                    null ||
                                                                deletingDocumentId !==
                                                                    null}
                                                            onclick={() =>
                                                                startDocumentRename(
                                                                    item.result
                                                                        .document,
                                                                )}
                                                        >
                                                            <Pencil
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                    {#if canDeleteDocument(item.result.document.id)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action search-folder-management__action--danger"
                                                            aria-label={`Delete ${item.result.document.title || "Untitled"}`}
                                                            title="Delete document"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null ||
                                                                savingDocumentId !==
                                                                    null ||
                                                                deletingDocumentId !==
                                                                    null}
                                                            onclick={() =>
                                                                void handleDeleteDocumentRow(
                                                                    item.result
                                                                        .document,
                                                                )}
                                                        >
                                                            <Trash2
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                </div>
                                            {/if}
                                        </div>
                                    {/if}
                                {/each}
                            </div>
                        {/if}
                    </div>
                {:else}
                    <div class="search-tree-wrap">
                        <div class="search-tree__toolbar">
                            <div class="search-tree__toolbar-start">
                                <button
                                    type="button"
                                    class="search-workspace__toolbar-action search-workspace__toolbar-action--subtle search-workspace__toolbar-action--icon"
                                    disabled={searchTree.folders.size === 0 ||
                                        expandedFolderKeys.size ===
                                            searchTree.folders.size}
                                    aria-label="Expand all folders"
                                    title="Expand all"
                                    onclick={expandAllFolders}
                                >
                                    <ListChevronsUpDown
                                        size={14}
                                        strokeWidth={2.1}
                                    />
                                </button>

                                <button
                                    type="button"
                                    class="search-workspace__toolbar-action search-workspace__toolbar-action--subtle search-workspace__toolbar-action--icon"
                                    disabled={expandedFolderKeys.size === 0}
                                    aria-label="Collapse all folders"
                                    title="Collapse all"
                                    onclick={collapseAllFolders}
                                >
                                    <ListChevronsDownUp
                                        size={14}
                                        strokeWidth={2.1}
                                    />
                                </button>
                            </div>
                        </div>

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
                                                    toggleFolderExpansion(
                                                        row.key,
                                                    )}
                                            >
                                                {#if expandedFolderKeys.has(row.key)}
                                                    <ChevronDown
                                                        size={16}
                                                        strokeWidth={2.5}
                                                    />
                                                {:else}
                                                    <ChevronRight
                                                        size={16}
                                                        strokeWidth={2.5}
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
                                            class="search-tree__folder-row"
                                            role="presentation"
                                            onmouseenter={() => {
                                                hoveredFolderKey = row.key;
                                            }}
                                            onmouseleave={() => {
                                                if (
                                                    hoveredFolderKey === row.key
                                                ) {
                                                    hoveredFolderKey = null;
                                                }
                                            }}
                                        >
                                            <div
                                                class:search-tree__folder--drop-target={dragTargetFolderKey ===
                                                    row.key}
                                                class:search-tree__folder--editing={editingFolderKey ===
                                                    row.key}
                                                class="search-tree__folder"
                                                role="treeitem"
                                                tabindex="-1"
                                                aria-selected="false"
                                                aria-label={`Folder ${row.name}`}
                                                onclick={(event) =>
                                                    handleTreeFolderRowClick(
                                                        event,
                                                        row,
                                                    )}
                                                onkeydown={(event) =>
                                                    handleTreeFolderRowKeyDown(
                                                        event,
                                                        row,
                                                    )}
                                                ondragover={(event) =>
                                                    handleFolderDragOver(
                                                        event,
                                                        row.key,
                                                    )}
                                                ondragleave={() =>
                                                    handleFolderDragLeave(
                                                        row.key,
                                                    )}
                                                ondrop={(event) =>
                                                    void handleFolderDrop(
                                                        event,
                                                        row.key,
                                                    )}
                                            >
                                                {#if editingFolderKey === row.key}
                                                    <span
                                                        class="search-tree__folder-content"
                                                    >
                                                        <input
                                                            bind:this={
                                                                renameInputElement
                                                            }
                                                            class="search-folder-management__input search-folder-management__input--tree"
                                                            type="text"
                                                            value={editingValue}
                                                            spellcheck="false"
                                                            maxlength="80"
                                                            oninput={(
                                                                event,
                                                            ) => {
                                                                editingValue = (
                                                                    event.currentTarget as HTMLInputElement
                                                                ).value;
                                                            }}
                                                            onkeydown={(
                                                                event,
                                                            ) => {
                                                                event.stopPropagation();
                                                                if (
                                                                    event.key ===
                                                                    "Enter"
                                                                ) {
                                                                    event.preventDefault();
                                                                    void submitFolderRename();
                                                                }

                                                                if (
                                                                    event.key ===
                                                                    "Escape"
                                                                ) {
                                                                    event.preventDefault();
                                                                    cancelFolderRename();
                                                                }
                                                            }}
                                                            onblur={() => {
                                                                void submitFolderRename();
                                                            }}
                                                        />

                                                        <span
                                                            class="search-tree__folder-meta"
                                                        >
                                                            {getFolderMeta(row)}
                                                        </span>
                                                    </span>
                                                {:else}
                                                    <span
                                                        class="search-tree__folder-content"
                                                    >
                                                        <span
                                                            class="search-tree__folder-name"
                                                            >{row.name}</span
                                                        >

                                                        <span
                                                            class="search-tree__folder-meta"
                                                        >
                                                            {getFolderMeta(row)}
                                                        </span>
                                                    </span>
                                                {/if}
                                            </div>

                                            {#if canCreateFolder(row.key) || canRenameFolder(row.key) || canDeleteFolder(row.key)}
                                                <div
                                                    class:search-folder-management__actions--visible={showFolderActions(
                                                        row.key,
                                                    )}
                                                    class="search-folder-management__actions search-folder-management__actions--tree"
                                                >
                                                    {#if canCreateFolder(row.key)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action"
                                                            aria-label={`Create subfolder in ${row.name}`}
                                                            title="Create subfolder"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null ||
                                                                creatingFolderParentKey !==
                                                                    null}
                                                            onclick={() =>
                                                                void handleCreateFolder(
                                                                    row.key,
                                                                )}
                                                        >
                                                            <FolderPlus
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                    {#if canRenameFolder(row.key)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action"
                                                            aria-label={`Rename ${row.name}`}
                                                            title="Rename folder"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null}
                                                            onclick={() =>
                                                                startFolderRename(
                                                                    row,
                                                                )}
                                                        >
                                                            <Pencil
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                    {#if canDeleteFolder(row.key)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action search-folder-management__action--danger"
                                                            aria-label={`Delete ${row.name}`}
                                                            title="Delete folder"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null}
                                                            onclick={() =>
                                                                void handleDeleteFolder(
                                                                    row,
                                                                )}
                                                        >
                                                            <Trash2
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                </div>
                                            {/if}
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
                                        <div
                                            class="search-tree__document-row"
                                            role="presentation"
                                            onmouseenter={() => {
                                                hoveredDocumentId =
                                                    row.result.document.id;
                                            }}
                                            onmouseleave={() => {
                                                if (
                                                    hoveredDocumentId ===
                                                    row.result.document.id
                                                ) {
                                                    hoveredDocumentId = null;
                                                }
                                            }}
                                        >
                                            {#if editingDocumentId === row.result.document.id}
                                                <div
                                                    aria-selected={row.result
                                                        .document.id ===
                                                        selectedDocumentId}
                                                    class:selected={row.result
                                                        .document.id ===
                                                        selectedDocumentId}
                                                    class:search-tree__document--hovered={hoveredDocumentId ===
                                                        row.result.document.id}
                                                    class="search-tree__document search-tree__document--editing"
                                                >
                                                    <span
                                                        class="search-tree__document-content"
                                                    >
                                                        <input
                                                            bind:this={
                                                                renameInputElement
                                                            }
                                                            class="search-folder-management__input search-folder-management__input--document"
                                                            type="text"
                                                            value={editingValue}
                                                            spellcheck="false"
                                                            maxlength="160"
                                                            oninput={(
                                                                event,
                                                            ) => {
                                                                editingValue = (
                                                                    event.currentTarget as HTMLInputElement
                                                                ).value;
                                                            }}
                                                            onkeydown={(
                                                                event,
                                                            ) => {
                                                                event.stopPropagation();
                                                                if (
                                                                    event.key ===
                                                                    "Enter"
                                                                ) {
                                                                    event.preventDefault();
                                                                    void submitDocumentRename();
                                                                }

                                                                if (
                                                                    event.key ===
                                                                    "Escape"
                                                                ) {
                                                                    event.preventDefault();
                                                                    cancelDocumentRename();
                                                                }
                                                            }}
                                                            onblur={() => {
                                                                void submitDocumentRename();
                                                            }}
                                                        />
                                                        <span
                                                            class="search-tree__document-meta"
                                                        >
                                                            {#if pendingMoveDocumentId === row.result.document.id}
                                                                Moving...
                                                            {:else}
                                                                Updated {formatUpdatedAt(
                                                                    row.result
                                                                        .document
                                                                        .updatedAt,
                                                                )}
                                                            {/if}
                                                        </span>
                                                    </span>
                                                </div>
                                            {:else}
                                                <button
                                                    type="button"
                                                    role="treeitem"
                                                    draggable={documentsAreInteractive &&
                                                        onMoveResultToFolder &&
                                                        pendingMoveDocumentId ===
                                                            null}
                                                    aria-selected={row.result
                                                        .document.id ===
                                                        selectedDocumentId}
                                                    class:selected={row.result
                                                        .document.id ===
                                                        selectedDocumentId}
                                                    class:search-tree__document--hovered={hoveredDocumentId ===
                                                        row.result.document.id}
                                                    class:search-tree__document--dragging={row
                                                        .result.document.id ===
                                                        draggingDocumentId}
                                                    class="search-tree__document"
                                                    disabled={!documentsAreInteractive}
                                                    onclick={() =>
                                                        handleOpenResult(
                                                            row.result.document,
                                                        )}
                                                    onmousemove={() => {
                                                        onHoverResult?.(
                                                            row.index,
                                                        );
                                                    }}
                                                    ondragstart={() =>
                                                        handleDocumentDragStart(
                                                            row.result.document
                                                                .id,
                                                        )}
                                                    ondragend={handleDocumentDragEnd}
                                                >
                                                    <span
                                                        class="search-tree__document-content"
                                                    >
                                                        <span
                                                            class="search-tree__document-title"
                                                        >
                                                            {row.result.document
                                                                .title ||
                                                                "Untitled"}
                                                        </span>
                                                        <span
                                                            class="search-tree__document-meta"
                                                        >
                                                            {#if pendingMoveDocumentId === row.result.document.id}
                                                                Moving...
                                                            {:else}
                                                                Updated {formatUpdatedAt(
                                                                    row.result
                                                                        .document
                                                                        .updatedAt,
                                                                )}
                                                            {/if}
                                                        </span>
                                                    </span>
                                                </button>
                                            {/if}

                                            {#if canRenameDocument(row.result.document.id) || canDeleteDocument(row.result.document.id)}
                                                <div
                                                    class:search-folder-management__actions--visible={showDocumentActions(
                                                        row.result.document.id,
                                                    )}
                                                    class="search-folder-management__actions search-folder-management__actions--tree"
                                                >
                                                    {#if canRenameDocument(row.result.document.id)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action"
                                                            aria-label={`Rename ${row.result.document.title || "Untitled"}`}
                                                            title="Rename document"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null ||
                                                                savingDocumentId !==
                                                                    null ||
                                                                deletingDocumentId !==
                                                                    null}
                                                            onclick={() =>
                                                                startDocumentRename(
                                                                    row.result
                                                                        .document,
                                                                )}
                                                        >
                                                            <Pencil
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                    {#if canDeleteDocument(row.result.document.id)}
                                                        <button
                                                            type="button"
                                                            class="search-folder-management__action search-folder-management__action--danger"
                                                            aria-label={`Delete ${row.result.document.title || "Untitled"}`}
                                                            title="Delete document"
                                                            disabled={savingFolderKey !==
                                                                null ||
                                                                deletingFolderKey !==
                                                                    null ||
                                                                savingDocumentId !==
                                                                    null ||
                                                                deletingDocumentId !==
                                                                    null}
                                                            onclick={() =>
                                                                void handleDeleteDocumentRow(
                                                                    row.result
                                                                        .document,
                                                                )}
                                                        >
                                                            <Trash2
                                                                size={12}
                                                                strokeWidth={2.1}
                                                            />
                                                        </button>
                                                    {/if}
                                                </div>
                                            {/if}
                                        </div>
                                    </div>
                                {/if}
                            {/each}
                        </div>
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</section>

<style>
    .search-workspace {
        --search-result-block-gap: 2px;
        --search-result-padding-y: 6px;
        --search-row-padding-x: 12px;
        --search-row-radius: 12px;
        --search-tree-row-min-height: 48px;
        --search-row-hover-bg: color-mix(
            in srgb,
            var(--surface-elevated) 78%,
            transparent
        );
        --search-folder-row-bg: color-mix(
            in srgb,
            var(--surface-overlay-medium) 48%,
            transparent
        );
        --search-folder-row-hover-bg: color-mix(
            in srgb,
            var(--surface-overlay-strong) 66%,
            transparent
        );
        --search-folder-row-text: color-mix(
            in srgb,
            var(--text) 84%,
            var(--muted)
        );
        --search-document-row-bg: color-mix(
            in srgb,
            var(--surface-overlay) 0%,
            transparent
        );
        --search-document-row-hover-bg: color-mix(
            in srgb,
            var(--surface-overlay-medium) 58%,
            transparent
        );
        --search-document-row-text: color-mix(
            in srgb,
            var(--text) 88%,
            var(--muted)
        );
        --search-document-title-text: color-mix(
            in srgb,
            var(--text) 82%,
            var(--muted)
        );
        --search-document-meta-text: color-mix(
            in srgb,
            var(--muted) 84%,
            var(--muted)
        );
        --search-row-title-size: 0.98rem;
        --search-row-meta-size: 0.8rem;
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
        gap: 12px;
        padding: 0 0 14px;
        border-bottom: 1px solid var(--line);
        color: var(--muted);
    }

    .search-workspace__input-wrap--create {
        grid-template-columns: minmax(0, 1fr);
    }

    .search-workspace__input {
        display: block;
        width: 100%;
        min-width: 0;
        padding: 0;
        border: 0;
        outline: 0;
        background: transparent;
        box-sizing: border-box;
        color: var(--text);
        font-size: clamp(1.9rem, 3.8vw, 3.2rem);
        font-weight: 700;
        line-height: 0.94;
        letter-spacing: -0.05em;
    }

    .search-workspace__input--create {
        grid-column: 1 / -1;
    }

    .search-workspace__input::placeholder {
        color: color-mix(in srgb, var(--text-placeholder) 20%, transparent);
    }

    .search-workspace__input:focus::placeholder {
        color: transparent;
    }

    .search-workspace__results {
        display: grid;
        gap: 0;
    }

    .search-workspace__results--list {
        gap: var(--search-result-block-gap);
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

    .search-workspace__results-header-actions {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        min-width: 0;
    }

    .search-workspace__view-toggle {
        display: inline-flex;
        align-items: center;
        gap: 2px;
    }

    .search-workspace__toolbar-action {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--muted);
        cursor: pointer;
        transition: color 120ms ease;
        font: inherit;
    }

    .search-workspace__toolbar-action:hover,
    .search-workspace__toolbar-action:focus-visible {
        color: var(--text);
        outline: none;
    }

    .search-workspace__toolbar-action:disabled {
        color: var(--text-placeholder);
    }

    .search-workspace__toolbar-action--subtle {
        color: color-mix(in srgb, var(--accent) 36%, var(--text-soft));
    }

    .search-workspace__toolbar-action--icon {
        justify-content: center;
        width: 28px;
        height: 28px;
        border-radius: 8px;
    }

    .search-workspace__toolbar-action--icon:hover,
    .search-workspace__toolbar-action--icon:focus-visible {
        background: color-mix(
            in srgb,
            var(--surface-elevated) 72%,
            transparent
        );
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
        padding: var(--search-result-padding-y) var(--search-row-padding-x);
        border: 0;
        border-radius: var(--search-row-radius);
        background: transparent;
        color: var(--text);
        text-align: left;
        cursor: pointer;
        transition:
            background 140ms ease,
            color 120ms ease;
    }

    .search-result:hover,
    .search-result--hovered {
        background: var(--search-row-hover-bg);
        color: var(--text);
    }

    .search-result.selected {
        color: var(--text);
    }

    .search-result-row {
        display: flex;
        align-items: center;
        gap: 8px;
        min-width: 0;
    }

    .search-result--editing {
        cursor: default;
    }

    .search-result__content {
        display: grid;
        gap: 4px;
        min-width: 0;
    }

    .search-result__title {
        font-weight: 500;
        font-size: var(--search-row-title-size);
    }

    .search-result__meta {
        font-size: var(--search-row-meta-size);
        color: var(--muted);
    }

    .search-result__folder-path {
        display: inline-flex;
        justify-self: end;
        min-width: 0;
        max-width: min(100%, 22rem);
    }

    .search-result__folder-path :global(.folder-path-badge) {
        background: transparent;
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

    .search-explorer__toolbar-main {
        display: flex;
        align-items: center;
        gap: 12px;
        flex: 1 1 auto;
        min-width: 0;
    }

    .search-explorer__toolbar-start {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        min-width: 0;
    }

    .search-explorer__toolbar-end {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        flex: 0 0 auto;
    }

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
        flex: 1 1 auto;
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
        gap: var(--search-result-block-gap);
    }

    .search-explorer__entry {
        display: flex;
        align-items: center;
        gap: 8px;
        min-width: 0;
        border-radius: 12px;
    }

    .search-explorer__entry--drop-target .search-explorer__item {
        background: var(--surface-overlay-medium);
    }

    .search-explorer__item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        min-width: 0;
        flex: 1 1 auto;
        width: 100%;
        padding: var(--search-result-padding-y) var(--search-row-padding-x);
        border: 0;
        border-radius: var(--search-row-radius);
        background: transparent;
        color: var(--text);
        text-align: left;
        cursor: pointer;
        transition:
            color 120ms ease,
            background 120ms ease;
    }

    .search-explorer__item--folder {
        background: var(--search-folder-row-bg);
        color: var(--search-folder-row-text);
    }

    .search-explorer__item--document {
        background: var(--search-document-row-bg);
        color: var(--search-document-row-text);
    }

    .search-explorer__item:hover,
    .search-explorer__item--hovered {
        background: var(--search-row-hover-bg);
        color: var(--text);
    }

    .search-explorer__item--folder:hover {
        background: var(--search-folder-row-hover-bg);
    }

    .search-explorer__item--document:hover,
    .search-explorer__item--document.search-explorer__item--hovered {
        background: var(--search-document-row-hover-bg);
    }

    .search-explorer__item.selected {
        color: var(--text);
    }

    .search-explorer__item--editing {
        cursor: default;
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
        font-size: var(--search-row-title-size);
        font-weight: 500;
        color: var(--search-document-title-text);
    }

    .search-explorer__item-title--folder {
        color: color-mix(in srgb, var(--text) 86%, var(--muted));
    }

    .search-explorer__item-meta {
        color: var(--search-document-meta-text);
        font-size: var(--search-row-meta-size);
    }

    .search-explorer__item-accessory {
        display: inline-flex;
        align-items: center;
        color: color-mix(in srgb, var(--text) 42%, var(--text-soft));
    }

    .search-tree {
        display: grid;
        gap: var(--search-result-block-gap);
    }

    .search-tree-wrap {
        display: grid;
        gap: 10px;
    }

    .search-tree__toolbar {
        display: flex;
        align-items: center;
        justify-content: flex-start;
        gap: 12px;
        padding-bottom: 2px;
    }

    .search-tree__toolbar-start {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        min-width: 0;
    }

    .search-tree__row {
        display: grid;
        grid-template-columns: 20px minmax(0, 1fr);
        align-items: center;
        gap: 6px;
        padding-left: calc(var(--tree-depth, 0) * 18px);
    }

    .search-tree__toggle,
    .search-tree__toggle-spacer {
        width: 20px;
        height: 20px;
        border-radius: 6px;
        flex-shrink: 0;
    }

    .search-tree__toggle {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 0;
        background: transparent;
        color: color-mix(in srgb, var(--accent-strong) 54%, var(--text));
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .search-tree__toggle:hover,
    .search-tree__toggle:focus-visible {
        background: color-mix(in srgb, var(--line) 50%, transparent);
        color: color-mix(in srgb, var(--accent-strong) 82%, var(--text));
        outline: none;
    }

    .search-tree__folder {
        display: flex;
        flex: 1 1 auto;
        align-items: center;
        justify-content: flex-start;
        gap: 12px;
        min-height: var(--search-tree-row-min-height);
        min-width: 0;
        padding: var(--search-result-padding-y) var(--search-row-padding-x);
        background: var(--search-folder-row-bg);
        color: var(--search-folder-row-text);
        border-radius: var(--search-row-radius);
        cursor: pointer;
        transition:
            background 140ms ease,
            color 120ms ease;
    }

    .search-tree__folder-row {
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        flex: 1 1 auto;
        min-width: 0;
    }

    .search-tree__folder--editing {
        cursor: default;
    }

    .search-tree__folder--drop-target {
        background: var(--surface-overlay-medium);
        color: var(--text);
    }

    .search-tree__folder--editing {
        background: color-mix(
            in srgb,
            var(--surface-elevated) 82%,
            transparent
        );
    }

    .search-tree__folder-row:hover .search-tree__folder {
        background: var(--search-folder-row-hover-bg);
    }

    .search-tree__folder--drop-target .search-tree__folder-meta {
        color: var(--muted);
    }

    .search-tree__folder-content {
        display: grid;
        gap: 3px;
        flex: 1 1 auto;
        min-width: 0;
    }

    .search-tree__folder-name {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: color-mix(in srgb, var(--text) 88%, var(--muted));
        font-size: var(--search-row-title-size);
        font-weight: 500;
    }

    .search-tree__folder-meta {
        color: var(--muted);
        font-size: var(--search-row-meta-size);
        white-space: nowrap;
    }

    .search-folder-management__actions {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        opacity: 0;
        pointer-events: none;
        transition: opacity 120ms ease;
    }

    .search-folder-management__actions--tree {
        justify-content: flex-end;
        flex: 0 0 92px;
        min-width: 92px;
    }

    .search-folder-management__actions--visible {
        opacity: 1;
        pointer-events: auto;
    }

    .search-folder-management__action {
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
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .search-folder-management__action:hover,
    .search-folder-management__action:focus-visible {
        background: color-mix(
            in srgb,
            var(--surface-elevated) 72%,
            transparent
        );
        color: var(--text);
        outline: none;
    }

    .search-folder-management__action--danger:hover,
    .search-folder-management__action--danger:focus-visible {
        color: var(--accent-strong);
    }

    .search-folder-management__action:disabled {
        color: var(--text-placeholder);
    }

    .search-folder-management__input {
        width: 100%;
        min-width: 0;
        padding: 0;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--text);
        font: inherit;
        font-weight: 500;
    }

    .search-folder-management__input--tree {
        flex: 1 1 auto;
        min-width: 0;
        color: color-mix(in srgb, var(--text) 88%, var(--muted));
        font-size: 0.94rem;
    }

    .search-folder-management__input--document {
        font-size: var(--search-row-title-size);
    }

    .search-folder-management__input::placeholder {
        color: var(--text-placeholder);
    }

    .search-tree__document {
        flex: 1 1 auto;
        min-height: var(--search-tree-row-min-height);
        padding: var(--search-result-padding-y) var(--search-row-padding-x);
        border: 0;
        border-radius: var(--search-row-radius);
        background: var(--search-document-row-bg);
        color: var(--search-document-row-text);
        text-align: left;
        cursor: pointer;
        transition:
            background 140ms ease,
            color 120ms ease;
    }

    .search-tree__document-row {
        display: flex;
        align-items: center;
        gap: 8px;
        min-width: 0;
        width: 100%;
    }

    .search-tree__document:hover,
    .search-tree__document--hovered {
        background: var(--search-document-row-hover-bg);
        color: var(--text);
    }

    .search-tree__document.selected {
        color: var(--text);
    }

    .search-tree__document--editing {
        cursor: default;
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
        font-size: var(--search-row-title-size);
        font-weight: 500;
        color: var(--search-document-title-text);
    }

    .search-tree__document-meta {
        color: var(--search-document-meta-text);
        font-size: var(--search-row-meta-size);
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

        .search-workspace__results-header-actions {
            flex-wrap: wrap;
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

        .search-explorer__toolbar-main {
            width: 100%;
            flex-wrap: wrap;
        }

        .search-explorer__toolbar-start {
            flex-wrap: wrap;
        }

        .search-explorer__breadcrumbs {
            justify-content: flex-start;
        }

        .search-explorer__toolbar-end {
            align-self: flex-start;
        }

        .search-explorer__entry,
        .search-tree__folder-row {
            align-items: flex-start;
        }

        .search-folder-management__actions {
            opacity: 1;
            pointer-events: auto;
        }

        .search-tree__folder {
            align-items: center;
        }
    }
</style>
