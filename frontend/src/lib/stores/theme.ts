import { browser } from '$app/environment';
import { get, writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'docit.theme';
const DEFAULT_THEME: Theme = 'dark';

export const theme = writable<Theme>(DEFAULT_THEME);

export function initializeTheme(): Theme {
	const nextTheme = readStoredTheme();
	theme.set(nextTheme);
	applyTheme(nextTheme);
	return nextTheme;
}

export function setTheme(nextTheme: Theme) {
	theme.set(nextTheme);
	applyTheme(nextTheme);

	if (browser) {
		window.localStorage.setItem(STORAGE_KEY, nextTheme);
	}
}

export function toggleTheme() {
	setTheme(get(theme) === 'dark' ? 'light' : 'dark');
}

function readStoredTheme(): Theme {
	if (!browser) {
		return DEFAULT_THEME;
	}

	const storedTheme = window.localStorage.getItem(STORAGE_KEY);
	return storedTheme === 'light' || storedTheme === 'dark' ? storedTheme : DEFAULT_THEME;
}

function applyTheme(nextTheme: Theme) {
	if (!browser) {
		return;
	}

	document.documentElement.dataset.theme = nextTheme;
	document.documentElement.style.colorScheme = nextTheme;
}