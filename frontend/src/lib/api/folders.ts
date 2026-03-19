import type { FolderSummary } from '$lib/types';

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

type ErrorEnvelope = {
    error?: {
        message: string;
    };
};

const API_BASE = '/api';

export async function listFolders(): Promise<FolderSummary[]> {
    const response = await fetch(`${API_BASE}/folders`);
    const payload = (await parseJson(response)) as FolderListResponse;
    return payload.folders.map(mapFolderSummary);
}

async function parseJson(response: Response) {
    const payload = (await response.json()) as unknown;
    if (!response.ok) {
        const message = (payload as ErrorEnvelope).error?.message ?? 'Request failed';
        throw new Error(message);
    }

    return payload;
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