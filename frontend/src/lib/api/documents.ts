import type { DocumentRecord, DocumentSummary } from '$lib/types';
import { jsonRequest, requestJson, requestVoid } from '$lib/api/client';

type DocumentPayload = {
	id: string;
	folderId: string;
	title: string;
	updatedAt: string;
	createdAt?: string;
};

type DocumentListResponse = {
	documents: DocumentPayload[];
};

type DocumentResponse = {
	document: DocumentPayload;
};

export async function listDocuments(options: { folderId?: string } = {}): Promise<DocumentSummary[]> {
	const search = new URLSearchParams();
	if (options.folderId) {
		search.set('folderId', options.folderId);
	}

	const suffix = search.size > 0 ? `?${search.toString()}` : '';
	const payload = await requestJson<DocumentListResponse>(`/documents${suffix}`);
	return payload.documents.map(mapDocumentSummary);
}

export async function createDocument(title: string, folderId?: string): Promise<DocumentRecord> {
	const payload = await requestJson<DocumentResponse>(
		'/documents',
		jsonRequest('POST', { title, folderId })
	);
	return mapDocumentRecord(payload.document);
}

export async function getDocument(documentId: string): Promise<DocumentRecord> {
	const payload = await requestJson<DocumentResponse>(`/documents/${documentId}`);
	return mapDocumentRecord(payload.document);
}

export async function duplicateDocument(documentId: string): Promise<DocumentRecord> {
	const payload = await requestJson<DocumentResponse>(
		`/documents/${documentId}/duplicate`,
		jsonRequest('POST')
	);
	return mapDocumentRecord(payload.document);
}

export async function renameDocumentTitle(documentId: string, title: string): Promise<DocumentSummary> {
	const payload = await requestJson<DocumentResponse>(
		`/documents/${documentId}/title`,
		jsonRequest('PATCH', { title })
	);
	return mapDocumentSummary(payload.document);
}

export async function moveDocumentToFolder(
	documentId: string,
	folderId: string
): Promise<DocumentRecord> {
	const payload = await requestJson<DocumentResponse>(
		`/documents/${documentId}/folder`,
		jsonRequest('PATCH', { folderId })
	);
	return mapDocumentRecord(payload.document);
}

export async function deleteDocument(documentId: string): Promise<void> {
	await requestVoid(`/documents/${documentId}`, jsonRequest('DELETE'));
}

function mapDocumentSummary(payload: DocumentPayload): DocumentSummary {
	return {
		id: payload.id,
		folderId: payload.folderId,
		title: payload.title,
		updatedAt: payload.updatedAt,
		createdAt: payload.createdAt ?? payload.updatedAt
	};
}

function mapDocumentRecord(payload: DocumentPayload): DocumentRecord {
	return mapDocumentSummary(payload);
}