<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount, tick } from 'svelte';
	import { fade } from 'svelte/transition';
	import { browser } from '$app/environment';
	import {
		ChevronDown,
		Search,
		Sun,
		Moon,
	} from 'lucide-svelte';
	import * as Y from 'yjs';

	import { listDocuments, renameDocumentTitle } from '$lib/api/documents';
	import PresenceBar from '$lib/components/PresenceBar.svelte';
	import EditorShell from '$lib/components/EditorShell.svelte';
	import { RealtimeClient } from '$lib/realtime/client';
	import { ensureSessionProfile } from '$lib/stores/session';
	import { theme, toggleTheme } from '$lib/stores/theme';
	import type {
		DocumentRecord,
		DocumentSummary,
		PeerPresence,
		SessionProfile
	} from '$lib/types';

	type SearchResult = {
		document: DocumentSummary;
		score: number;
	};

	let { data } = $props<{
		data: {
			id: string;
			document: DocumentRecord | null;
			loadError: string;
		};
	}>();

	let document = $state<DocumentRecord | null>(null);
	let session = $state<SessionProfile | null>(null);
	let peers = $state<PeerPresence[]>([]);
	let errorMessage = $state('');
	let loading = $state(true);
	let syncReady = $state(false);
	let titleDraft = $state('');
	let activeTopbarMenu = $state<string | null>(null);
	let searchModalOpen = $state(false);
	let searchQuery = $state('');
	let searchResultsIndex = $state(0);
	let searchDocuments = $state<DocumentSummary[] | null>(null);
	let searchLoading = $state(false);
	let searchErrorMessage = $state('');
	let searchInputElement = $state<HTMLInputElement | null>(null);

	const documentMenu = {
		label: 'Document',
		items: ['Rename', 'Duplicate', 'Export Markdown']
	} as const;

	const shareMenu = {
		label: 'Share',
		items: ['Invite collaborators', 'Copy share link', 'Publish snapshot']
	} as const;

	let ydoc = $state<Y.Doc>(new Y.Doc());
	let client: RealtimeClient | null = null;
	let renameTimer: number | null = null;
	let sessionPromise: Promise<SessionProfile> | null = null;

	onMount(() => {
		if (!browser) {
			return;
		}

		window.addEventListener('pointerdown', handleDocumentPointerDown);

		return () => {
			window.removeEventListener('pointerdown', handleDocumentPointerDown);
		};
	});

	$effect(() => {
		if (!browser) {
			return;
		}

		const documentId = data.id;
		let cancelled = false;

		void loadDocumentPage(documentId, () => cancelled);

		return () => {
			cancelled = true;
			destroyActiveDocumentSession();
		};
	});

	$effect(() => {
		searchModalOpen;
		if (!searchModalOpen) {
			return;
		}

		void tick().then(() => {
			searchInputElement?.focus();
			searchInputElement?.select();
		});
	});

	$effect(() => {
		searchQuery;
		searchResultsIndex = 0;
	});

	async function loadDocumentPage(documentId: string, isCancelled: () => boolean) {
		resetDocumentState();

		const nextYdoc = new Y.Doc();
		ydoc = nextYdoc;
		const initialDocument = data.document;
		const initialError = data.loadError;

		try {
			const nextSession = await getSessionProfile();

			if (isCancelled()) {
				nextYdoc.destroy();
				return;
			}

			if (!initialDocument) {
				errorMessage = initialError || 'Failed to open document';
				loading = false;
				return;
			}

			session = nextSession;
			document = initialDocument;
			titleDraft = initialDocument.title;
			loading = false;

			const nextClient = new RealtimeClient(documentId, nextSession, nextYdoc, {
				onConnectionState() {},
				onPresence(nextPeers) {
					if (isCancelled()) {
						return;
					}

					peers = nextPeers.filter((peer) => peer.clientId !== nextSession.clientId);
				},
				onInitialSync() {
					if (isCancelled()) {
						return;
					}

					syncReady = true;
				}
			});

			client = nextClient;
			await nextClient.connect();
		} catch (error) {
			if (isCancelled()) {
				return;
			}

			errorMessage = error instanceof Error ? error.message : 'Failed to open document';
		} finally {
			if (!isCancelled()) {
				loading = false;
			}
		}
	}

	function destroyActiveDocumentSession() {
		client?.disconnect();
		client = null;
		peers = [];
		syncReady = false;

		if (renameTimer) {
			clearTimeout(renameTimer);
			renameTimer = null;
		}

		ydoc.destroy();
	}

	function resetDocumentState() {
		document = null;
		peers = [];
		errorMessage = '';
		loading = true;
		syncReady = false;
		titleDraft = '';
		activeTopbarMenu = null;
		closeSearchModal();
	}

	async function getSessionProfile() {
		if (session) {
			return session;
		}

		if (!sessionPromise) {
			sessionPromise = ensureSessionProfile().then((profile) => {
				session = profile;
				return profile;
			});
		}

		return sessionPromise;
	}

	function handleSelectionChange(anchor: number, head: number) {
		client?.updatePresence(anchor, head);
	}

	function handleTitleChange(value: string) {
		titleDraft = value;

		if (renameTimer) {
			clearTimeout(renameTimer);
		}

		renameTimer = setTimeout(async () => {
			try {
				if (!document) {
					return;
				}

				const updated = await renameDocumentTitle(document.id, value);
				document = { ...document, ...updated };
			} catch (error) {
				errorMessage = error instanceof Error ? error.message : 'Failed to rename document';
			}
		}, 450);
	}


	function handleTopbarMenuToggle(label: string, isOpen: boolean) {
		activeTopbarMenu = isOpen ? label : activeTopbarMenu === label ? null : activeTopbarMenu;
	}

	async function openSearchModal() {
		activeTopbarMenu = null;
		searchModalOpen = true;
		searchQuery = '';
		searchResultsIndex = 0;
		await ensureSearchDocumentsLoaded();
	}

	function closeSearchModal() {
		searchModalOpen = false;
		searchQuery = '';
		searchResultsIndex = 0;
		searchErrorMessage = '';
	}

	async function ensureSearchDocumentsLoaded() {
		if (searchDocuments || searchLoading) {
			return;
		}

		searchLoading = true;
		searchErrorMessage = '';
		try {
			searchDocuments = await listDocuments();
		} catch (error) {
			searchErrorMessage = error instanceof Error ? error.message : 'Failed to load documents';
		} finally {
			searchLoading = false;
		}
	}

	function getSearchableDocuments() {
		return (searchDocuments ?? []).filter((item) => item.id !== data.id);
	}

	function getRecentDocuments(): SearchResult[] {
		return getSearchableDocuments()
			.slice()
			.sort(
				(left, right) =>
					new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime()
			)
			.slice(0, 8)
			.map((item, index) => ({ document: item, score: 100 - index }));
	}

	function getSearchResults(): SearchResult[] {
		const query = searchQuery.trim();
		if (!query) {
			return getRecentDocuments();
		}

		return getSearchableDocuments()
			.map((item) => ({ document: item, score: getFuzzyMatchScore(item.title, query) }))
			.filter((item) => item.score > Number.NEGATIVE_INFINITY)
			.sort((left, right) => {
				if (right.score !== left.score) {
					return right.score - left.score;
				}

				return new Date(right.document.updatedAt).getTime() - new Date(left.document.updatedAt).getTime();
			})
			.slice(0, 8);
	}

	function getFuzzyMatchScore(title: string, query: string) {
		const normalizedTitle = title.trim().toLowerCase();
		const normalizedQuery = query.trim().toLowerCase();

		if (!normalizedQuery) {
			return 0;
		}

		const directIndex = normalizedTitle.indexOf(normalizedQuery);
		if (directIndex >= 0) {
			return 1000 - directIndex * 4 - normalizedTitle.length;
		}

		let score = 0;
		let searchFrom = 0;
		let previousMatchIndex = -1;

		for (const character of normalizedQuery) {
			const matchIndex = normalizedTitle.indexOf(character, searchFrom);
			if (matchIndex === -1) {
				return Number.NEGATIVE_INFINITY;
			}

			score += previousMatchIndex >= 0 && matchIndex === previousMatchIndex + 1 ? 12 : 4;
			score -= matchIndex;
			searchFrom = matchIndex + 1;
			previousMatchIndex = matchIndex;
		}

		return score - normalizedTitle.length;
	}

	function handleSearchInputKeyDown(event: KeyboardEvent) {
		const results = getSearchResults();

		if (event.key === 'Escape') {
			event.preventDefault();
			closeSearchModal();
			return;
		}

		if (results.length === 0) {
			return;
		}

		if (event.key === 'ArrowDown') {
			event.preventDefault();
			searchResultsIndex = (searchResultsIndex + 1) % results.length;
			return;
		}

		if (event.key === 'ArrowUp') {
			event.preventDefault();
			searchResultsIndex = (searchResultsIndex - 1 + results.length) % results.length;
			return;
		}

		if (event.key === 'Enter') {
			event.preventDefault();
			void openSearchResult(results[searchResultsIndex]?.document ?? null);
		}
	}

	async function openSearchResult(target: DocumentSummary | null) {
		if (!target) {
			return;
		}

		closeSearchModal();
		await tick();

		if (target.id === data.id) {
			return;
		}

		await goto(`/d/${target.id}`);
	}

	function formatSearchUpdatedAt(value: string) {
		return new Date(value).toLocaleString();
	}

	function handleDocumentPointerDown(event: PointerEvent) {
		if (!activeTopbarMenu) {
			return;
		}

		const target = event.target;
		if (!(target instanceof Element) || !target.closest('.dropdown-badge')) {
			activeTopbarMenu = null;
		}
	}
</script>

<svelte:head>
	<title>{document ? `${document.title} | Doc-it` : 'Loading | Doc-it'}</title>
</svelte:head>

<div class="editor-page">
	<section class="editor-pane">
		<div class="editor-content">
			<header class="topbar">
				<div class="topbar-left">
					{#if document}
						<details
							class="dropdown-badge"
							open={activeTopbarMenu === documentMenu.label}
							ontoggle={(event) =>
								handleTopbarMenuToggle(
									documentMenu.label,
									(event.currentTarget as HTMLDetailsElement).open
								)}
						>
							<summary>
								<span>{documentMenu.label}</span>
								<ChevronDown size={14} strokeWidth={2.2} />
							</summary>
							<div class="dropdown-panel">
								<p class="dropdown-label">Placeholder actions</p>
								<div class="dropdown-items">
									{#each documentMenu.items as item (item)}
										<button type="button" class="dropdown-item" disabled>
											{item}
										</button>
									{/each}
								</div>
							</div>
						</details>
					{:else}
						<button type="button" class="menu-badge-button menu-badge-button--disabled" disabled>
							<span>{documentMenu.label}</span>
							<ChevronDown size={14} strokeWidth={2.2} />
						</button>
					{/if}
					<button type="button" class="menu-badge-button" onclick={() => void openSearchModal()}>
						<span>Search</span>
						<Search size={14} strokeWidth={2.2} />
					</button>
					<button
						type="button"
						class="menu-badge-button menu-badge-button--icon"
						onclick={toggleTheme}
						aria-label={$theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
						title={$theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
					>
						{#if $theme === 'dark'}
							<Moon size={14} strokeWidth={2.2} />
						{:else}
							<Sun size={14} strokeWidth={2.2} />
						{/if}
					</button>
				</div>
				<div class="topbar-meta">
					<PresenceBar {peers} />
					{#if document}
						<details
							class="dropdown-badge dropdown-badge--right"
							open={activeTopbarMenu === shareMenu.label}
							ontoggle={(event) =>
								handleTopbarMenuToggle(
									shareMenu.label,
									(event.currentTarget as HTMLDetailsElement).open
								)}
						>
							<summary>
								<span>{shareMenu.label}</span>
								<ChevronDown size={14} strokeWidth={2.2} />
							</summary>
							<div class="dropdown-panel">
								<p class="dropdown-label">Placeholder actions</p>
								<div class="dropdown-items">
									{#each shareMenu.items as item (item)}
										<button type="button" class="dropdown-item" disabled>
											{item}
										</button>
									{/each}
								</div>
							</div>
						</details>
					{:else}
						<button type="button" class="menu-badge-button menu-badge-button--disabled" disabled>
							<span>{shareMenu.label}</span>
							<ChevronDown size={14} strokeWidth={2.2} />
						</button>
					{/if}
				</div>
			</header>

			{#if searchModalOpen}
				<div class="search-modal-layer" transition:fade={{ duration: 120 }}>
					<button
						type="button"
						class="search-modal-backdrop"
						aria-label="Close search"
						onclick={closeSearchModal}
					></button>
					<div class="search-modal" role="dialog" aria-modal="true" aria-label="Search documents">
						<div class="search-modal__input-wrap">
							<Search size={16} strokeWidth={2.1} />
							<input
								bind:this={searchInputElement}
								bind:value={searchQuery}
								class="search-modal__input"
								type="text"
								placeholder="Search documents"
								spellcheck="false"
								autocomplete="off"
								onkeydown={handleSearchInputKeyDown}
							/>
						</div>

						{#if searchLoading}
							<p class="search-modal__state">Loading documents…</p>
						{:else if searchErrorMessage}
							<p class="search-modal__state search-modal__state--error">{searchErrorMessage}</p>
						{:else if getSearchResults().length === 0}
							<p class="search-modal__state">No documents match that search.</p>
						{:else}
							<div class="search-modal__results">
								<p class="search-modal__label">{searchQuery.trim() ? 'Matching documents' : 'Recent documents'}</p>
								{#each getSearchResults() as result, index (result.document.id)}
									<button
										type="button"
										class:selected={index === searchResultsIndex}
										class="search-result"
										onclick={() => void openSearchResult(result.document)}
										onmousemove={() => {
											searchResultsIndex = index;
										}}
									>
										<span class="search-result__title">{result.document.title || 'Untitled'}</span>
										<span class="search-result__meta">
											Updated {formatSearchUpdatedAt(result.document.updatedAt)}
										</span>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			{/if}

			{#if !loading && errorMessage && !document}
				<p class="status-card error">{errorMessage}</p>
			{:else if !loading && document}
				{#key data.id}
					<EditorShell
						title={titleDraft}
						doc={ydoc}
						{peers}
						onTitleChange={handleTitleChange}
						onSelectionChange={handleSelectionChange}
					/>
				{/key}
			{/if}
		</div>
	</section>
</div>

<style>
	.editor-page {
		padding: 0 0 28px;
		--topbar-height: 64px;
		--menu-badge-height: calc(1em + (var(--presence-chip-padding-y, 4px) * 2) + 2px);
		--presence-chip-padding-y: 4px;
		--presence-chip-padding-x: 10px;
		--presence-chip-font-size: 0.84rem;
		--presence-chip-gap: 8px;
		--presence-swatch-size: 10px;
	}

	.editor-pane {
		display: grid;
		gap: 0;
		min-height: 100vh;
	}

	.editor-content {
		display: grid;
		gap: 0;
	}

	.topbar {
		position: sticky;
		top: 0;
		z-index: 30;
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		height: var(--topbar-height);
		box-sizing: border-box;
		padding: 0 32px;
		background: transparent;
	}

	.topbar-left {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.dropdown-badge {
		position: relative;
	}

	.dropdown-badge summary,
	.menu-badge-button {
		list-style: none;
		display: inline-flex;
		align-items: center;
		gap: var(--presence-chip-gap, 8px);
		white-space: nowrap;
		block-size: var(--menu-badge-height);
		line-height: 1;
		padding: 0 var(--presence-chip-padding-x, 10px);
		border: 1px solid var(--line);
		border-radius: 8px;
		background: var(--surface-overlay);
		color: var(--text);
		font-size: var(--presence-chip-font-size, 0.84rem);
		font-weight: 500;
		cursor: pointer;
		transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
	}

	.menu-badge-button {
		appearance: none;
		-webkit-appearance: none;
		margin: 0;
		font-family: inherit;
	}

	.menu-badge-button--icon {
		justify-content: center;
		inline-size: var(--menu-badge-height);
		padding: 0;
		gap: 0;
	}

	.menu-badge-button:disabled,
	.menu-badge-button--disabled {
		cursor: default;
		opacity: 0.72;
	}

	.dropdown-badge summary::-webkit-details-marker {
		display: none;
	}

	.dropdown-badge summary span,
	.dropdown-badge summary :global(svg),
	.menu-badge-button span,
	.menu-badge-button :global(svg) {
		display: block;
		flex-shrink: 0;
	}

	.dropdown-badge summary :global(svg) {
		transform: translateY(1px);
	}

	.dropdown-badge[open] summary,
	.dropdown-badge summary:hover,
	.menu-badge-button:hover {
		background: var(--surface-overlay-medium);
		border-color: var(--surface-overlay-border);
	}

	.dropdown-panel {
		position: absolute;
		top: calc(100% + 10px);
		left: 0;
		min-width: 220px;
		padding: 12px;
		border: 1px solid var(--line);
		border-radius: 16px;
		background: var(--dropdown-panel-bg);
		box-shadow: var(--shadow);
		backdrop-filter: blur(16px);
	}

	.dropdown-badge--right .dropdown-panel {
		left: auto;
		right: 0;
	}

	.dropdown-label {
		margin: 0 0 8px;
		font-size: 0.72rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-soft);
	}

	.dropdown-items {
		display: grid;
		gap: 6px;
	}

	.dropdown-item {
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--surface-overlay);
		color: var(--text);
		text-align: left;
		cursor: not-allowed;
		opacity: 0.72;
	}

	.topbar-meta {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 12px;
		justify-self: end;
		flex-shrink: 0;
	}

	.search-modal-layer {
		position: fixed;
		inset: 0;
		z-index: 80;
		display: grid;
		place-items: start center;
		padding: 88px 16px 24px;
	}

	.search-modal-backdrop {
		position: absolute;
		inset: 0;
		border: 0;
		padding: 0;
		background: var(--modal-backdrop);
		cursor: default;
	}

	.search-modal__results {
		display: grid;
		gap: 6px;
		padding-top: 12px;
	}

	.search-modal__label {
		margin: 0;
		padding: 0 4px 6px;
		font-size: 0.72rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-soft);
	}

	.search-modal__state {
		margin: 0;
		padding: 18px 6px 8px;
		color: var(--muted);
	}

	.search-modal__state--error {
		color: var(--accent-strong);
	}

	.search-result {
		display: grid;
		gap: 4px;
		width: 100%;
		padding: 12px 14px;
		border: 1px solid transparent;
		border-radius: 14px;
		background: transparent;
		color: var(--text);
		text-align: left;
		cursor: pointer;
		transition: background 120ms ease, border-color 120ms ease;
	}

	.search-result:hover,
	.search-result.selected {
		background: var(--surface-overlay-medium);
		border-color: var(--surface-overlay-border);
	}

	.search-result__title {
		font-weight: 500;
	}

	.search-result__meta {
		font-size: 0.82rem;
		color: var(--muted);
	}

	.status-card {
		padding: 22px 24px;
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: 16px;
		color: var(--muted);
	}

	.error {
		color: var(--accent-strong);
	}

	@media (max-width: 980px) {
		.topbar {
			padding-left: 16px;
			padding-right: 16px;
			grid-template-columns: minmax(0, 1fr) auto;
			grid-template-areas:
				'left meta';
			height: auto;
			padding-top: 10px;
			padding-bottom: 10px;
		}

		.topbar-left {
			grid-area: left;
			overflow-x: auto;
			scrollbar-width: none;
		}

		.topbar-left::-webkit-scrollbar {
			display: none;
		}

		.topbar-meta {
			grid-area: meta;
			justify-content: flex-end;
		}

		.dropdown-panel {
			left: 0;
			right: auto;
		}

		.dropdown-badge--right .dropdown-panel {
			left: auto;
			right: 0;
		}
	}

	@media (min-resolution: 1.5dppx) {
		.editor-page {
			--topbar-height: 40px;
			--presence-chip-padding-y: 5px;
			--presence-chip-padding-x: 11px;
			--presence-chip-font-size: 0.87rem;
			--presence-swatch-size: 11px;
		}
	}

	@media (min-resolution: 2dppx) {
		.editor-page {
			--topbar-height: 44px;
			--presence-chip-padding-y: 6px;
			--presence-chip-padding-x: 12px;
			--presence-chip-font-size: 0.9rem;
			--presence-swatch-size: 12px;
		}
	}
</style>