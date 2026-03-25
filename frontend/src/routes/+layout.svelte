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
		--hero-glow: rgba(227, 233, 243, 0.18);
		--button-accent-border: rgba(227, 233, 243, 0.34);
		--button-accent-contrast: #101827;
		--document-row-border-hover: rgba(227, 233, 243, 0.3);
		--blockquote-line: rgba(227, 233, 243, 0.34);
		--bg: #111215;
		--bg-elevated: #15171b;
		--panel: #1b1e23;
		--panel-strong: #242832;
		--line: rgba(208, 193, 178, 0.18);
		--line-strong: rgba(208, 193, 178, 0.32);
		--text: #f7f3ee;
		--muted: #a39a90;
		--text-placeholder: rgba(247, 243, 238, 0.5);
		--accent: #c6cedc;
		--accent-strong: #e3e9f3;
		--toolbar-active: #edf1f7;
		--toolbar-active-text: #101827;
		--surface-overlay: rgba(227, 233, 243, 0.06);
		--surface-overlay-medium: rgba(227, 233, 243, 0.1);
		--surface-overlay-strong: rgba(227, 233, 243, 0.14);
		--surface-elevated: var(--surface-overlay-strong);
		--surface-overlay-border: rgba(227, 233, 243, 0.16);
		--text-soft: rgba(247, 243, 238, 0.78);
		--code-bg: #0d0f12;
		--code-border: rgba(208, 193, 178, 0.15);
		--code-text: #f1ece6;
		--success: #52d6a2;
		--danger: #ff6b7a;
		--shadow: 0 24px 60px rgba(0, 0, 0, 0.42);
		--floating-toolbar-shadow: 0 16px 40px rgba(0, 0, 0, 0.32);
		--page-bg-start: #121418;
		--page-bg-end: #0d1014;
		--hero-surface-start: rgba(31, 34, 40, 0.98);
		--hero-surface-end: rgba(20, 22, 27, 0.98);
		--dropdown-panel-bg: rgba(22, 24, 29, 0.96);
		--modal-backdrop: rgba(6, 8, 10, 0.58);
		--modal-bg: rgba(24, 26, 31, 0.98);
		--document-row-bg: rgba(227, 233, 243, 0.05);
	}

	:global(:root[data-theme="light"]) {
		color-scheme: light;
		--hero-glow: rgba(40, 46, 54, 0.08);
		--button-accent-border: rgba(40, 46, 54, 0.18);
		--button-accent-contrast: #ffffff;
		--document-row-border-hover: rgba(40, 46, 54, 0.14);
		--blockquote-line: rgba(40, 46, 54, 0.22);
		--bg: #f3f4f3;
		--bg-elevated: #f8f9f8;
		--panel: #ffffff;
		--panel-strong: #f0f2f1;
		--line: rgba(29, 33, 39, 0.12);
		--line-strong: rgba(29, 33, 39, 0.18);
		--text: #171b20;
		--muted: #5f6771;
		--text-placeholder: rgba(23, 27, 32, 0.4);
		--accent: #66707b;
		--accent-strong: #313943;
		--toolbar-active: #313943;
		--toolbar-active-text: #ffffff;
		--surface-overlay: rgba(29, 33, 39, 0.04);
		--surface-overlay-medium: rgba(29, 33, 39, 0.07);
		--surface-overlay-strong: rgba(29, 33, 39, 0.11);
		--surface-elevated: var(--surface-overlay-strong);
		--surface-overlay-border: rgba(29, 33, 39, 0.13);
		--text-soft: rgba(23, 27, 32, 0.72);
		--code-bg: #f5f7f8;
		--code-border: rgba(29, 33, 39, 0.1);
		--code-text: #212831;
		--success: #2f7a57;
		--danger: #ba4f5c;
		--shadow: 0 20px 48px rgba(20, 24, 30, 0.1);
		--floating-toolbar-shadow: 0 12px 28px rgba(20, 24, 30, 0.08);
		--page-bg-start: #ffffff;
		--page-bg-end: #eef0ef;
		--hero-surface-start: rgba(255, 255, 255, 0.98);
		--hero-surface-end: rgba(245, 247, 246, 0.99);
		--dropdown-panel-bg: rgba(255, 255, 255, 0.98);
		--modal-backdrop: rgba(24, 29, 35, 0.14);
		--modal-bg: rgba(255, 255, 255, 0.98);
		--document-row-bg: rgba(29, 33, 39, 0.04);
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
