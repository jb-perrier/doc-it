import { moveDocumentToFolder } from '$lib/api/documents';
import { createFolder, renameFolder } from '$lib/api/folders';
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

function replaceFolder(items: FolderSummary[], updated: FolderSummary): FolderSummary[] {
    return items.map((folder) => (folder.id === updated.id ? updated : folder));
}

function replaceFolderInOptionalCollection(
    items: FolderSummary[] | null,
    updated: FolderSummary
): FolderSummary[] | null {
    return items ? replaceFolder(items, updated) : null;
}