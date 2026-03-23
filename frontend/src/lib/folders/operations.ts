import { moveDocumentToFolder } from '$lib/api/documents';
import { createFolder, deleteFolder, renameFolder } from '$lib/api/folders';
import type { DocumentRecord, FolderSummary } from '$lib/types';

type FolderCollections = {
    folders: FolderSummary[];
    searchFolders: FolderSummary[] | null;
};

export async function moveDocumentWithinFolders(
    document: DocumentRecord,
    nextFolderId: string
): Promise<DocumentRecord> {
    return moveDocumentToFolder(document.id, nextFolderId);
}

export async function renameFolderInCollections(
    collections: FolderCollections,
    folderId: string,
    name: string
): Promise<FolderCollections & { updated: FolderSummary }> {
    const updated = await renameFolder(folderId, name);

    return {
        updated,
        folders: replaceFolder(collections.folders, updated),
        searchFolders: replaceFolderInOptionalCollection(collections.searchFolders, updated)
    };
}

export async function createSubfolderInCollections(
    collections: FolderCollections,
    parentFolderId: string
): Promise<FolderCollections & { created: FolderSummary }> {
    const created = await createFolder('Untitled folder', parentFolderId);

    return {
        created,
        folders: [...collections.folders, created],
        searchFolders: collections.searchFolders ? [...collections.searchFolders, created] : null
    };
}

export async function deleteFolderInCollections(
    collections: FolderCollections,
    folderId: string
): Promise<FolderCollections> {
    await deleteFolder(folderId);

    const deletedFolderIds = collectDeletedFolderIds(collections.folders, folderId);

    return {
        folders: removeDeletedFolders(collections.folders, deletedFolderIds),
        searchFolders: removeDeletedFoldersInOptionalCollection(
            collections.searchFolders,
            deletedFolderIds
        )
    };
}

function replaceFolder(items: FolderSummary[], updated: FolderSummary): FolderSummary[] {
    return items.map((folder) => (folder.id === updated.id ? updated : folder));
}

function collectDeletedFolderIds(items: FolderSummary[], folderId: string): Set<string> {
    const deletedFolderIds = new Set<string>([folderId]);
    let changed = true;

    while (changed) {
        changed = false;

        for (const folder of items) {
            if (
                folder.parentFolderId &&
                deletedFolderIds.has(folder.parentFolderId) &&
                !deletedFolderIds.has(folder.id)
            ) {
                deletedFolderIds.add(folder.id);
                changed = true;
            }
        }
    }

    return deletedFolderIds;
}

function removeDeletedFolders(items: FolderSummary[], deletedFolderIds: Set<string>): FolderSummary[] {
    return items.filter((folder) => !deletedFolderIds.has(folder.id));
}

function replaceFolderInOptionalCollection(
    items: FolderSummary[] | null,
    updated: FolderSummary
): FolderSummary[] | null {
    return items ? replaceFolder(items, updated) : null;
}

function removeDeletedFoldersInOptionalCollection(
    items: FolderSummary[] | null,
    deletedFolderIds: Set<string>
): FolderSummary[] | null {
    return items ? removeDeletedFolders(items, deletedFolderIds) : null;
}