import type { DocumentRecord } from '$lib/types';

import type { PageLoad } from './$types';

type DocumentPayload = {
	id: string;
	title: string;
	updatedAt: string;
	createdAt?: string;
};

type DocumentResponse = {
	document: DocumentPayload;
};

type ErrorEnvelope = {
	error?: {
		message?: string;
	};
};

export const load: PageLoad = async ({ params, fetch }) => {
	let document: DocumentRecord | null = null;
	let loadError = '';

	try {
		const response = await fetch(`/api/documents/${params.id}`);
		const payload = (await response.json()) as DocumentResponse | ErrorEnvelope;

		if (!response.ok) {
			const errorPayload = payload as ErrorEnvelope;
			throw new Error(errorPayload.error?.message ?? 'Failed to open document');
		}

		document = mapDocumentRecord((payload as DocumentResponse).document);
	} catch (error) {
		loadError = error instanceof Error ? error.message : 'Failed to open document';
	}

	return {
		id: params.id,
		document,
		loadError
	};
};

function mapDocumentRecord(payload: DocumentPayload): DocumentRecord {
	return {
		id: payload.id,
		title: payload.title,
		updatedAt: payload.updatedAt,
		createdAt: payload.createdAt ?? payload.updatedAt
	};
}