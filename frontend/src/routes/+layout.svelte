<script lang="ts">
	import "@fontsource/ibm-plex-sans/400.css";
	import "@fontsource/ibm-plex-sans/500.css";
	import "@fontsource/ibm-plex-sans/600.css";
	import "@fontsource/ibm-plex-sans/700.css";
	import { onMount } from "svelte";

	import favicon from "$lib/assets/favicon.svg";
	import { initializeTheme } from "$lib/stores/theme";

	let { children } = $props();

	onMount(() => {
		initializeTheme();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Doc-it</title>
</svelte:head>

{@render children()}

<style>
	:global(:root) {
		font-family: "IBM Plex Sans", "Aptos", "Segoe UI", sans-serif;
		--danger-soft: rgba(255, 107, 122, 0.12);
		--danger-border: rgba(255, 107, 122, 0.22);
	}

	:global(:root[data-theme="dark"]) {
		color-scheme: dark;
		--hero-glow: rgba(224, 138, 74, 0.12);
		--button-accent-border: rgba(229, 161, 109, 0.28);
		--button-accent-contrast: #1a130f;
		--document-row-border-hover: rgba(229, 161, 109, 0.34);
		--blockquote-line: rgba(229, 161, 109, 0.4);
		--bg: #17171a;
		--bg-elevated: #131416;
		--panel: #1a1b1f;
		--panel-strong: #202228;
		--line: rgba(181, 166, 152, 0.14);
		--line-strong: rgba(181, 166, 152, 0.24);
		--text: #efece7;
		--muted: #8b847d;
		--text-placeholder: rgba(239, 236, 231, 0.42);
		--accent: #e08a4a;
		--accent-strong: #f0a56a;
		--toolbar-active: #f1ece5;
		--toolbar-active-text: #151517;
		--surface-overlay: rgba(255, 245, 236, 0.035);
		--surface-overlay-medium: rgba(255, 245, 236, 0.065);
		--surface-overlay-strong: rgba(255, 245, 236, 0.1);
		--surface-elevated: var(--surface-overlay-strong);
		--surface-overlay-border: rgba(255, 245, 236, 0.12);
		--text-soft: rgba(239, 236, 231, 0.7);
		--code-bg: #101114;
		--code-border: rgba(181, 166, 152, 0.12);
		--code-text: #f1ece6;
		--success: #52d6a2;
		--danger: #ff6b7a;
		--shadow: 0 20px 48px rgba(0, 0, 0, 0.34);
		--floating-toolbar-shadow: 0 12px 28px rgba(0, 0, 0, 0.24);
		--page-bg-start: #17181b;
		--page-bg-end: #131417;
		--hero-surface-start: rgba(28, 29, 33, 0.96);
		--hero-surface-end: rgba(18, 19, 22, 0.96);
		--dropdown-panel-bg: rgba(26, 27, 31, 0.9);
		--modal-backdrop: rgba(9, 10, 13, 0.48);
		--modal-bg: rgba(26, 27, 31, 0.96);
		--document-row-bg: rgba(255, 239, 226, 0.03);
	}

	:global(:root[data-theme="light"]) {
		color-scheme: light;
		--hero-glow: rgba(92, 119, 180, 0.1);
		--button-accent-border: rgba(92, 119, 180, 0.24);
		--button-accent-contrast: #f7faff;
		--document-row-border-hover: rgba(92, 119, 180, 0.22);
		--blockquote-line: rgba(92, 119, 180, 0.3);
		--bg: #e4ebf3;
		--bg-elevated: #edf2f7;
		--panel: #f6f9fc;
		--panel-strong: #e9eff6;
		--line: rgba(39, 54, 78, 0.16);
		--line-strong: rgba(39, 54, 78, 0.24);
		--text: #162033;
		--muted: #5f6d82;
		--text-placeholder: rgba(22, 32, 51, 0.38);
		--accent: #5c77b4;
		--accent-strong: #49639b;
		--toolbar-active: #31476f;
		--toolbar-active-text: #f7faff;
		--surface-overlay: rgba(29, 45, 72, 0.045);
		--surface-overlay-medium: rgba(29, 45, 72, 0.085);
		--surface-overlay-strong: rgba(29, 45, 72, 0.12);
		--surface-elevated: var(--surface-overlay-strong);
		--surface-overlay-border: rgba(29, 45, 72, 0.15);
		--text-soft: rgba(22, 32, 51, 0.68);
		--code-bg: #eaf0f7;
		--code-border: rgba(39, 54, 78, 0.11);
		--code-text: #1b2940;
		--success: #157f5b;
		--danger: #c84c63;
		--shadow: 0 20px 48px rgba(23, 35, 56, 0.14);
		--floating-toolbar-shadow: 0 10px 24px rgba(23, 35, 56, 0.1);
		--page-bg-start: #eff4fa;
		--page-bg-end: #dde6f0;
		--hero-surface-start: rgba(246, 249, 253, 0.96);
		--hero-surface-end: rgba(235, 241, 248, 0.98);
		--dropdown-panel-bg: rgba(246, 249, 252, 0.94);
		--modal-backdrop: rgba(24, 37, 58, 0.18);
		--modal-bg: rgba(246, 249, 252, 0.98);
		--document-row-bg: rgba(92, 119, 180, 0.045);
	}

	:global(html) {
		background: var(--page-bg-end);
		min-height: 100%;
	}

	:global(body) {
		margin: 0;
		min-height: 100vh;
		color: var(--text);
		background: var(--page-bg-end);
	}

	:global(*) {
		box-sizing: border-box;
	}

	:global(button),
	:global(input),
	:global(textarea) {
		font: inherit;
	}

	:global(a) {
		color: inherit;
		text-decoration: none;
	}
</style>
