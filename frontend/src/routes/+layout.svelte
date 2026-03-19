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
		--hero-glow: rgba(224, 138, 74, 0.12);
		--button-accent-border: rgba(229, 161, 109, 0.28);
		--button-accent-contrast: #1a130f;
		--danger-soft: rgba(255, 107, 122, 0.12);
		--danger-border: rgba(255, 107, 122, 0.22);
		--document-row-border-hover: rgba(229, 161, 109, 0.34);
		--blockquote-line: rgba(229, 161, 109, 0.4);
	}

	:global(:root[data-theme="dark"]) {
		color-scheme: dark;
		--bg: #17171a;
		--bg-elevated: #131416;
		--panel: #1a1b1f;
		--panel-strong: #202228;
		--line: rgba(181, 166, 152, 0.14);
		--line-strong: rgba(181, 166, 152, 0.24);
		--text: #efece7;
		--muted: #9c948d;
		--accent: #e08a4a;
		--accent-strong: #f0a56a;
		--toolbar-active: #f1ece5;
		--toolbar-active-text: #151517;
		--surface-overlay: rgba(255, 245, 236, 0.035);
		--surface-overlay-medium: rgba(255, 245, 236, 0.065);
		--surface-overlay-strong: rgba(255, 245, 236, 0.1);
		--surface-overlay-border: rgba(255, 245, 236, 0.12);
		--text-soft: rgba(239, 236, 231, 0.7);
		--code-bg: #101114;
		--code-border: rgba(181, 166, 152, 0.12);
		--code-text: #f1ece6;
		--success: #52d6a2;
		--danger: #ff6b7a;
		--shadow: 0 20px 48px rgba(0, 0, 0, 0.34);
		--floating-toolbar-shadow: 0 12px 28px rgba(0, 0, 0, 0.24);
		--page-bg-start: #151517;
		--page-bg-end: #101113;
		--hero-surface-start: rgba(28, 29, 33, 0.96);
		--hero-surface-end: rgba(18, 19, 22, 0.96);
		--dropdown-panel-bg: rgba(26, 27, 31, 0.9);
		--modal-backdrop: rgba(9, 10, 13, 0.48);
		--modal-bg: rgba(26, 27, 31, 0.96);
		--document-row-bg: rgba(255, 239, 226, 0.03);
	}

	:global(:root[data-theme="light"]) {
		color-scheme: light;
		--bg: #f3eee6;
		--bg-elevated: #fcf8f2;
		--panel: #fffaf4;
		--panel-strong: #f5eee5;
		--line: rgba(84, 64, 44, 0.14);
		--line-strong: rgba(84, 64, 44, 0.22);
		--text: #241c15;
		--muted: #75685c;
		--accent: #d07a35;
		--accent-strong: #ad5a1c;
		--toolbar-active: #2b2118;
		--toolbar-active-text: #fdf9f3;
		--surface-overlay: rgba(71, 47, 24, 0.04);
		--surface-overlay-medium: rgba(71, 47, 24, 0.08);
		--surface-overlay-strong: rgba(71, 47, 24, 0.1);
		--surface-overlay-border: rgba(71, 47, 24, 0.14);
		--text-soft: rgba(36, 28, 21, 0.68);
		--code-bg: #f5ede4;
		--code-border: rgba(84, 64, 44, 0.1);
		--code-text: #2d231a;
		--success: #157f5b;
		--danger: #c84c63;
		--shadow: 0 20px 48px rgba(86, 65, 44, 0.14);
		--floating-toolbar-shadow: 0 10px 24px rgba(86, 65, 44, 0.1);
		--page-bg-start: #f7f2ea;
		--page-bg-end: #ece2d6;
		--hero-surface-start: rgba(255, 250, 243, 0.96);
		--hero-surface-end: rgba(244, 236, 226, 0.98);
		--dropdown-panel-bg: rgba(255, 250, 244, 0.94);
		--modal-backdrop: rgba(68, 49, 31, 0.2);
		--modal-bg: rgba(255, 250, 244, 0.98);
		--document-row-bg: rgba(208, 122, 53, 0.05);
	}

	:global(html) {
		background: var(--bg);
		min-height: 100%;
	}

	:global(body) {
		margin: 0;
		min-height: 100vh;
		color: var(--text);
		background: linear-gradient(
			180deg,
			var(--page-bg-start) 0%,
			var(--bg) 28%,
			var(--page-bg-end) 100%
		);
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
