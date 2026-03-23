import type { FolderSummary } from '$lib/types';
import { jsonRequest, requestJson, requestVoid } from '$lib/api/client';

type FolderPayload = {
    id: string;
    parentFolderId: string | null;
    name: string;
    createdAt: string;
    updatedAt: string;
};

type FolderListResponse = {
    folders: FolderPayload[];
};

type FolderResponse = {
    folder: FolderPayload;
};

export async function listFolders(): Promise<FolderSummary[]> {
    const payload = await requestJson<FolderListResponse>('/folders');
    return payload.folders.map(mapFolderSummary);
}

export async function createFolder(
    name: string,
    parentFolderId?: string | null,
): Promise<FolderSummary> {
    const payload = await requestJson<FolderResponse>(
        '/folders',
        jsonRequest('POST', {
            name,
            parentFolderId: parentFolderId ?? null
        }),
    );
    return mapFolderSummary(payload.folder);
}

export async function renameFolder(
    folderId: string,
    name: string,
): Promise<FolderSummary> {
    const payload = await requestJson<FolderResponse>(
        `/folders/${folderId}`,
        jsonRequest('PATCH', { name }),
    );
    return mapFolderSummary(payload.folder);
}

export async function deleteFolder(folderId: string): Promise<void> {
    await requestVoid(`/folders/${folderId}`, jsonRequest('DELETE'));
}

function mapFolderSummary(payload: FolderPayload): FolderSummary {
    return {
        id: payload.id,
        parentFolderId: payload.parentFolderId,
        name: payload.name,
        createdAt: payload.createdAt,
        updatedAt: payload.updatedAt
    };
}