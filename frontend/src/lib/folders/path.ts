import type { FolderSummary } from '$lib/types';

export type FolderPathSegment = {
    id: string;
    name: string;
};

type FolderLookup = ReadonlyMap<string, FolderSummary>;

export function createFolderLookup(folders: FolderSummary[]): FolderLookup {
    return new Map(folders.map((folder) => [folder.id, folder]));
}

export function getFolderPathSegments(
    folderId: string,
    folders: FolderSummary[] | FolderLookup
): FolderPathSegment[] {
    const folderLookup: FolderLookup = Array.isArray(folders)
        ? createFolderLookup(folders)
        : folders;
    const segments: FolderPathSegment[] = [];
    const visited = new Set<string>();

    let currentFolderId: string | null = folderId;

    while (currentFolderId) {
        if (visited.has(currentFolderId)) {
            break;
        }

        visited.add(currentFolderId);

        const folder: FolderSummary | undefined = folderLookup.get(currentFolderId);
        if (!folder) {
            break;
        }

        segments.unshift({ id: folder.id, name: folder.name });
        currentFolderId = folder.parentFolderId;
    }

    return segments;
}