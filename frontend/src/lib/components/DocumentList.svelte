<script lang="ts">
	import { goto } from '$app/navigation';

	import type { DocumentSummary } from '$lib/types';

	let {
		documents,
		loading,
		creating,
		onCreate
	} = $props<{
		documents: DocumentSummary[];
		loading: boolean;
		creating: boolean;
		onCreate: () => void;
	}>();
</script>

<section class="list-card">
	<div class="section-head">
		<div>
			<p class="kicker">Documents</p>
			<h2>Recent work</h2>
		</div>
		<button class="ghost-action" onclick={onCreate} disabled={creating}>
			{creating ? 'Creating…' : 'New'}
		</button>
	</div>

	{#if loading}
		<p class="empty-state">Loading documents…</p>
	{:else if documents.length === 0}
		<p class="empty-state">No documents yet. Create one to start editing.</p>
	{:else}
		<div class="document-grid">
			{#each documents as document (document.id)}
				<button class="document-row" onclick={() => goto(`/d/${document.id}`)}>
					<strong>{document.title}</strong>
					<span>Updated {new Date(document.updatedAt).toLocaleString()}</span>
				</button>
			{/each}
		</div>
	{/if}
</section>

<style>
	.list-card {
		padding: 28px;
		border-radius: 28px;
		border: 1px solid var(--line);
		background: var(--panel);
		box-shadow: var(--shadow);
	}

	.section-head {
		display: flex;
		justify-content: space-between;
		gap: 18px;
		align-items: center;
		margin-bottom: 18px;
	}

	.kicker {
		margin: 0 0 6px;
		font-size: 0.78rem;
		text-transform: uppercase;
		letter-spacing: 0.16em;
		color: var(--muted);
	}

	h2 {
		margin: 0;
		font-size: 1.5rem;
	}

	.ghost-action {
		border: 1px solid var(--line);
		background: var(--bg-elevated);
		color: var(--text);
		padding: 10px 16px;
		border-radius: 999px;
		cursor: pointer;
	}

	.document-grid {
		display: grid;
		gap: 12px;
	}

	.document-row {
		text-align: left;
		padding: 18px 20px;
		border-radius: 18px;
		border: 1px solid var(--line);
		background: var(--document-row-bg);
		color: var(--text);
		cursor: pointer;
		display: grid;
		gap: 6px;
		transition:
			transform 160ms ease,
			border-color 160ms ease,
			background 160ms ease;
	}

	.document-row:hover {
		transform: translateY(-1px);
		border-color: var(--document-row-border-hover);
		background: var(--surface-overlay);
	}

	.document-row span,
	.empty-state {
		color: var(--muted);
	}
</style>