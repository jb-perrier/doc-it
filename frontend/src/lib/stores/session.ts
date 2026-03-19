import { browser } from '$app/environment';

import type { SessionProfile } from '$lib/types';

const STORAGE_KEY = 'docit.session';
const palette = ['#bb5a2b', '#0f766e', '#1d4ed8', '#7c3aed', '#c2410c', '#15803d'];

export async function ensureSessionProfile(): Promise<SessionProfile> {
	if (!browser) {
		return {
			clientId: 'server-session',
			name: 'Server',
			color: palette[0]
		};
	}

	const existing = window.localStorage.getItem(STORAGE_KEY);
	if (existing) {
		return JSON.parse(existing) as SessionProfile;
	}

	const proposedName = window.prompt('Choose a display name for collaborative editing', 'Guest');
	const name = (proposedName?.trim() || `Guest ${Math.floor(Math.random() * 90 + 10)}`).slice(0, 32);
	const session = {
		clientId: `c_${createSessionId()}`,
		name,
		color: palette[Math.abs(hashString(name)) % palette.length]
	} satisfies SessionProfile;

	window.localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
	return session;
}

export async function updateSessionProfileName(nextName: string): Promise<SessionProfile> {
	const existing = await ensureSessionProfile();
	const name = normalizeSessionName(nextName);
	const session = {
		...existing,
		name,
		color: palette[Math.abs(hashString(name)) % palette.length]
	} satisfies SessionProfile;

	if (browser) {
		window.localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
	}

	return session;
}

function normalizeSessionName(value: string): string {
	return (value.trim() || `Guest ${Math.floor(Math.random() * 90 + 10)}`).slice(0, 32);
}

function hashString(value: string): number {
	let hash = 0;
	for (const character of value) {
		hash = (hash << 5) - hash + character.charCodeAt(0);
		hash |= 0;
	}
	return hash;
}

function createSessionId(): string {
	const secureCrypto = globalThis.crypto;
	if (secureCrypto?.randomUUID) {
		return secureCrypto.randomUUID();
	}

	if (secureCrypto?.getRandomValues) {
		const bytes = secureCrypto.getRandomValues(new Uint8Array(16));
		bytes[6] = (bytes[6] & 0x0f) | 0x40;
		bytes[8] = (bytes[8] & 0x3f) | 0x80;
		return [
			toHex(bytes.subarray(0, 4)),
			toHex(bytes.subarray(4, 6)),
			toHex(bytes.subarray(6, 8)),
			toHex(bytes.subarray(8, 10)),
			toHex(bytes.subarray(10, 16))
		].join('-');
	}

	return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`;
}

function toHex(bytes: Uint8Array): string {
	return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}