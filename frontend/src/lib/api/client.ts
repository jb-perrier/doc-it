type ErrorEnvelope = {
    error?: {
        code?: string;
        message?: string;
    };
};

const API_BASE = '/api';

export async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
    const response = await fetch(`${API_BASE}${path}`, init);
    await ensureOk(response);
    return (await response.json()) as T;
}

export async function requestVoid(path: string, init?: RequestInit): Promise<void> {
    const response = await fetch(`${API_BASE}${path}`, init);
    await ensureOk(response);
}

export function jsonRequest(method: string, body?: unknown): RequestInit {
    if (body === undefined) {
        return { method };
    }

    return {
        method,
        headers: {
            'content-type': 'application/json'
        },
        body: JSON.stringify(body)
    };
}

async function ensureOk(response: Response) {
    if (response.ok) {
        return;
    }

    let payload: ErrorEnvelope | null = null;

    try {
        payload = (await response.json()) as ErrorEnvelope;
    } catch {
        payload = null;
    }

    const message = payload?.error?.message ?? 'Request failed';
    throw new Error(message);
}