<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	import { createDocument, listDocuments } from '$lib/api/documents';
	import DocumentList from '$lib/components/DocumentList.svelte';
	import type { DocumentSummary } from '$lib/types';

	let documents = $state<DocumentSummary[]>([]);
	let loading = $state(true);
	let creating = $state(false);
	let errorMessage = $state('');

	onMount(async () => {
		await refreshDocuments();
	});

	async function refreshDocuments() {
		loading = true;
		errorMessage = '';

		try {
			documents = await listDocuments();
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Failed to load documents';
		} finally {
			loading = false;
		}
	}

	async function handleCreate() {
		creating = true;
		try {
			const document = await createDocument('Untitled');
			await goto(`/d/${document.id}`);
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Failed to create document';
			creating = false;
		}
	}
</script>

<svelte:head>
	<title>Doc-it | Documents</title>
</svelte:head>

<div class="page-shell">
	<section class="hero-card">
		<div>
			<p class="eyebrow">Realtime Markdown-backed docs</p>
			<h1>Write in a calm space. Sync in realtime.</h1>
			<p class="lede">
				Create lightweight collaborative documents with a focused editor, live presence, and
				automatic persistence.
			</p>
		</div>
		<button class="primary-action" onclick={handleCreate} disabled={creating}>
			{creating ? 'Creating…' : 'Create document'}
		</button>
	</section>

	<DocumentList {documents} {loading} {creating} onCreate={handleCreate} />

	{#if errorMessage}
		<p class="error-banner">{errorMessage}</p>
	{/if}
</div>

<style>
	.page-shell {
		max-width: 1120px;
		margin: 0 auto;
		padding: 48px 20px 64px;
		display: grid;
		gap: 28px;
	}

	.hero-card {
		display: flex;
		justify-content: space-between;
		gap: 24px;
		align-items: end;
		padding: 32px;
		border: 1px solid var(--line);
		border-radius: 28px;
		background:
			radial-gradient(circle at top left, var(--hero-glow), transparent 34%),
			linear-gradient(135deg, var(--hero-surface-start), var(--hero-surface-end));
		box-shadow: var(--shadow);
	}

	.eyebrow {
		margin: 0 0 10px;
		text-transform: uppercase;
		letter-spacing: 0.16em;
		font-size: 0.72rem;
		color: var(--accent-strong);
	}

	h1 {
		margin: 0;
		max-width: 12ch;
		font-size: clamp(2.6rem, 8vw, 4.8rem);
		line-height: 0.92;
		letter-spacing: -0.05em;
	}

	.lede {
		margin: 16px 0 0;
		max-width: 54ch;
		font-size: 1.05rem;
		line-height: 1.6;
		color: var(--muted);
	}

	.primary-action {
		border: 1px solid var(--button-accent-border);
		border-radius: 999px;
		padding: 14px 22px;
		background: var(--accent);
		color: var(--button-accent-contrast);
		font-weight: 600;
		cursor: pointer;
		transition:
			transform 160ms ease,
			background 160ms ease;
	}

	.primary-action:hover:not(:disabled) {
		transform: translateY(-1px);
		background: var(--accent-strong);
	}

	.primary-action:disabled {
		opacity: 0.6;
		cursor: wait;
	}

	.error-banner {
		margin: 0;
		padding: 12px 16px;
		border-radius: 16px;
		background: var(--danger-soft);
		border: 1px solid var(--danger-border);
		color: var(--accent-strong);
	}

	@media (max-width: 820px) {
		.hero-card {
			flex-direction: column;
			align-items: start;
		}

		h1 {
			max-width: none;
		}
	}
</style>
