import type { DocumentRecord, DocumentSummary } from '$lib/types';

type DocumentPayload = {
	id: string;
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

type ErrorEnvelope = {
	error?: {
		code: string;
		message: string;
	};
};

const API_BASE = '/api';

export async function listDocuments(): Promise<DocumentSummary[]> {
	const response = await fetch(`${API_BASE}/documents`);
	const payload = (await parseJson(response)) as DocumentListResponse;
	return payload.documents.map(mapDocumentSummary);
}

export async function createDocument(title: string): Promise<DocumentRecord> {
	const response = await fetch(`${API_BASE}/documents`, {
		method: 'POST',
		headers: {
			'content-type': 'application/json'
		},
		body: JSON.stringify({ title })
	});

	const payload = (await parseJson(response)) as DocumentResponse;
	return mapDocumentRecord(payload.document);
}

export async function getDocument(documentId: string): Promise<DocumentRecord> {
	const response = await fetch(`${API_BASE}/documents/${documentId}`);
	const payload = (await parseJson(response)) as DocumentResponse;
	return mapDocumentRecord(payload.document);
}

export async function renameDocumentTitle(documentId: string, title: string): Promise<DocumentSummary> {
	const response = await fetch(`${API_BASE}/documents/${documentId}/title`, {
		method: 'PATCH',
		headers: {
			'content-type': 'application/json'
		},
		body: JSON.stringify({ title })
	});

	const payload = (await parseJson(response)) as DocumentResponse;
	return mapDocumentSummary(payload.document);
}

async function parseJson(response: Response) {
	const payload = (await response.json()) as unknown;
	if (!response.ok) {
		const message = (payload as ErrorEnvelope).error?.message ?? 'Request failed';
		throw new Error(message);
	}

	return payload;
}

function mapDocumentSummary(payload: DocumentPayload): DocumentSummary {
	return {
		id: payload.id,
		title: payload.title,
		updatedAt: payload.updatedAt,
		createdAt: payload.createdAt ?? payload.updatedAt
	};
}

function mapDocumentRecord(payload: DocumentPayload): DocumentRecord {
	return mapDocumentSummary(payload);
}