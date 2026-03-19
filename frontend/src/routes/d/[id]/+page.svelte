<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount, tick } from "svelte";
	import { fade } from "svelte/transition";
	import { browser } from "$app/environment";
	import { ChevronDown, Search, Settings2, Sun, Moon } from "lucide-svelte";
	import * as Y from "yjs";

	import { listDocuments, renameDocumentTitle } from "$lib/api/documents";
	import PresenceBar from "$lib/components/PresenceBar.svelte";
	import EditorShell from "$lib/components/EditorShell.svelte";
	import { RealtimeClient } from "$lib/realtime/client";
	import {
		ensureSessionProfile,
		updateSessionProfileName,
	} from "$lib/stores/session";
	import { theme, toggleTheme } from "$lib/stores/theme";
	import type {
		DocumentRecord,
		DocumentSummary,
		PeerPresence,
		SessionProfile,
	} from "$lib/types";

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
	let errorMessage = $state("");
	let loading = $state(true);
	let syncReady = $state(false);
	let titleDraft = $state("");
	let activeTopbarMenu = $state<string | null>(null);
	let searchModalOpen = $state(false);
	let searchQuery = $state("");
	let searchResultsIndex = $state(0);
	let searchDocuments = $state<DocumentSummary[] | null>(null);
	let searchLoading = $state(false);
	let searchErrorMessage = $state("");
	let searchInputElement = $state<HTMLInputElement | null>(null);
	let usernameDraft = $state("");
	let usernameSaving = $state(false);
	let usernameErrorMessage = $state("");

	const documentMenu = {
		label: "Document",
		items: ["Rename", "Duplicate", "Export Markdown", "Delete"],
	} as const;

	const shareMenu = {
		label: "Share",
		items: ["Invite collaborators", "Copy share link", "Publish snapshot"],
	} as const;

	const settingsMenuLabel = "Settings";

	let ydoc = $state<Y.Doc>(new Y.Doc());
	let client: RealtimeClient | null = null;
	let renameTimer: number | null = null;
	let sessionPromise: Promise<SessionProfile> | null = null;

	onMount(() => {
		if (!browser) {
			return;
		}

		window.addEventListener("pointerdown", handleDocumentPointerDown);

		return () => {
			window.removeEventListener(
				"pointerdown",
				handleDocumentPointerDown,
			);
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

	async function loadDocumentPage(
		documentId: string,
		isCancelled: () => boolean,
	) {
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
				errorMessage = initialError || "Failed to open document";
				loading = false;
				return;
			}

			session = nextSession;
			usernameDraft = nextSession.name;
			document = initialDocument;
			titleDraft = initialDocument.title;

			await connectRealtimeClient(
				documentId,
				nextSession,
				nextYdoc,
				isCancelled,
			);

			if (!isCancelled()) {
				loading = false;
			}
		} catch (error) {
			if (isCancelled()) {
				return;
			}

			errorMessage =
				error instanceof Error
					? error.message
					: "Failed to open document";
			loading = false;
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
		errorMessage = "";
		loading = true;
		syncReady = false;
		titleDraft = "";
		usernameErrorMessage = "";
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
				usernameDraft = profile.name;
				return profile;
			});
		}

		return sessionPromise;
	}

	async function connectRealtimeClient(
		documentId: string,
		nextSession: SessionProfile,
		targetDoc: Y.Doc,
		isCancelled: () => boolean = () => false,
	) {
		const nextClient = new RealtimeClient(
			documentId,
			nextSession,
			targetDoc,
			{
				onConnectionState() {},
				onPresence(nextPeers) {
					if (isCancelled()) {
						return;
					}

					peers = nextPeers.filter(
						(peer) => peer.clientId !== nextSession.clientId,
					);
				},
				onInitialSync() {
					if (isCancelled()) {
						return;
					}

					syncReady = true;
				},
			},
		);

		client = nextClient;
		await nextClient.connect();
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
				errorMessage =
					error instanceof Error
						? error.message
						: "Failed to rename document";
			}
		}, 450);
	}

	function handleTopbarMenuToggle(label: string, isOpen: boolean) {
		activeTopbarMenu = isOpen
			? label
			: activeTopbarMenu === label
				? null
				: activeTopbarMenu;

		if (label === settingsMenuLabel && isOpen && session) {
			usernameDraft = session.name;
			usernameErrorMessage = "";
		}
	}

	async function handleUsernameSubmit() {
		if (!session || usernameSaving) {
			return;
		}

		const nextName = usernameDraft.trim();
		if (!nextName) {
			usernameErrorMessage = "Username cannot be empty";
			return;
		}

		if (nextName === session.name) {
			activeTopbarMenu = null;
			usernameErrorMessage = "";
			return;
		}

		usernameSaving = true;
		usernameErrorMessage = "";

		try {
			const nextSession = await updateSessionProfileName(nextName);
			session = nextSession;
			sessionPromise = Promise.resolve(nextSession);
			usernameDraft = nextSession.name;

			if (document) {
				client?.disconnect();
				client = null;
				syncReady = false;
				await connectRealtimeClient(data.id, nextSession, ydoc);
			}

			activeTopbarMenu = null;
		} catch (error) {
			usernameErrorMessage =
				error instanceof Error
					? error.message
					: "Failed to update username";
		} finally {
			usernameSaving = false;
		}
	}

	async function openSearchModal() {
		activeTopbarMenu = null;
		searchModalOpen = true;
		searchQuery = "";
		searchResultsIndex = 0;
		await ensureSearchDocumentsLoaded();
	}

	function closeSearchModal() {
		searchModalOpen = false;
		searchQuery = "";
		searchResultsIndex = 0;
		searchErrorMessage = "";
	}

	async function ensureSearchDocumentsLoaded() {
		if (searchDocuments || searchLoading) {
			return;
		}

		searchLoading = true;
		searchErrorMessage = "";
		try {
			searchDocuments = await listDocuments();
		} catch (error) {
			searchErrorMessage =
				error instanceof Error
					? error.message
					: "Failed to load documents";
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
					new Date(right.updatedAt).getTime() -
					new Date(left.updatedAt).getTime(),
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
			.map((item) => ({
				document: item,
				score: getFuzzyMatchScore(item.title, query),
			}))
			.filter((item) => item.score > Number.NEGATIVE_INFINITY)
			.sort((left, right) => {
				if (right.score !== left.score) {
					return right.score - left.score;
				}

				return (
					new Date(right.document.updatedAt).getTime() -
					new Date(left.document.updatedAt).getTime()
				);
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

			score +=
				previousMatchIndex >= 0 && matchIndex === previousMatchIndex + 1
					? 12
					: 4;
			score -= matchIndex;
			searchFrom = matchIndex + 1;
			previousMatchIndex = matchIndex;
		}

		return score - normalizedTitle.length;
	}

	function handleSearchInputKeyDown(event: KeyboardEvent) {
		const results = getSearchResults();

		if (event.key === "Escape") {
			event.preventDefault();
			closeSearchModal();
			return;
		}

		if (results.length === 0) {
			return;
		}

		if (event.key === "ArrowDown") {
			event.preventDefault();
			searchResultsIndex = (searchResultsIndex + 1) % results.length;
			return;
		}

		if (event.key === "ArrowUp") {
			event.preventDefault();
			searchResultsIndex =
				(searchResultsIndex - 1 + results.length) % results.length;
			return;
		}

		if (event.key === "Enter") {
			event.preventDefault();
			void openSearchResult(
				results[searchResultsIndex]?.document ?? null,
			);
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
		if (
			!(target instanceof Element) ||
			!target.closest(".dropdown-badge")
		) {
			activeTopbarMenu = null;
		}
	}
</script>

<svelte:head>
	<title>{document ? `${document.title} | Doc-it` : "Loading | Doc-it"}</title
	>
</svelte:head>

<svelte:document onpointerdown={handleDocumentPointerDown} />

<div class="editor-page">
	<section class="editor-layout">
		<aside class="side-rail side-rail--left">
			<div class="side-rail__inner">
				<div class="topbar-left">
					{#if document}
						<details
							class="dropdown-badge"
							open={activeTopbarMenu === documentMenu.label}
							ontoggle={(event) =>
								handleTopbarMenuToggle(
									documentMenu.label,
									(event.currentTarget as HTMLDetailsElement)
										.open,
								)}
						>
							<summary>
								<span>{documentMenu.label}</span>
								<ChevronDown size={14} strokeWidth={2.2} />
							</summary>
							<div class="dropdown-panel">
								<div class="dropdown-items">
									{#each documentMenu.items as item (item)}
										<button
											type="button"
											class:dropdown-item--danger={item === "Delete"}
											class="dropdown-item"
										>
											{item}
										</button>
									{/each}
								</div>
							</div>
						</details>
					{:else}
						<button
							type="button"
							class="menu-badge-button menu-badge-button--disabled"
							disabled
						>
							<span>{documentMenu.label}</span>
							<ChevronDown size={14} strokeWidth={2.2} />
						</button>
					{/if}
					<button
						type="button"
						class="menu-badge-button"
						onclick={() => void openSearchModal()}
					>
						<span>Search</span>
						<Search size={14} strokeWidth={2.2} />
					</button>
					<button
						type="button"
						class="menu-badge-button menu-badge-button--icon"
						onclick={toggleTheme}
						aria-label={$theme === "dark"
							? "Switch to light theme"
							: "Switch to dark theme"}
						title={$theme === "dark"
							? "Switch to light theme"
							: "Switch to dark theme"}
					>
						{#if $theme === "dark"}
							<Moon size={14} strokeWidth={2.2} />
						{:else}
							<Sun size={14} strokeWidth={2.2} />
						{/if}
					</button>
				</div>
			</div>
		</aside>

		<section class="editor-pane">
			<div class="editor-content">
				{#if searchModalOpen}
					<div
						class="search-modal-layer"
						transition:fade={{ duration: 120 }}
					>
						<button
							type="button"
							class="search-modal-backdrop"
							aria-label="Close search"
							onclick={closeSearchModal}
						></button>
						<div
							class="search-modal"
							role="dialog"
							aria-modal="true"
							aria-label="Search documents"
						>
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
								<p class="search-modal__state">
									Loading documents…
								</p>
							{:else if searchErrorMessage}
								<p
									class="search-modal__state search-modal__state--error"
								>
									{searchErrorMessage}
								</p>
							{:else if getSearchResults().length === 0}
								<p class="search-modal__state">
									No documents match that search.
								</p>
							{:else}
								<div class="search-modal__results">
									<p class="search-modal__label">
										{searchQuery.trim()
											? "Matching documents"
											: "Recent documents"}
									</p>
									{#each getSearchResults() as result, index (result.document.id)}
										<button
											type="button"
											class:selected={index ===
												searchResultsIndex}
											class="search-result"
											onclick={() =>
												void openSearchResult(
													result.document,
												)}
											onmousemove={() => {
												searchResultsIndex = index;
											}}
										>
											<span class="search-result__title"
												>{result.document.title ||
													"Untitled"}</span
											>
											<span class="search-result__meta">
												Updated {formatSearchUpdatedAt(
													result.document.updatedAt,
												)}
											</span>
										</button>
									{/each}
								</div>
							{/if}
						</div>
					</div>
				{/if}

				{#if !loading && errorMessage}
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

		<aside class="side-rail side-rail--right">
			<div class="side-rail__inner side-rail__inner--right">
				<div class="topbar-meta">
					<PresenceBar {peers} />
					<div class="side-rail__actions side-rail__actions--right">
						{#if document}
							<details
								class="dropdown-badge dropdown-badge--right"
								open={activeTopbarMenu === shareMenu.label}
								ontoggle={(event) =>
									handleTopbarMenuToggle(
										shareMenu.label,
										(
											event.currentTarget as HTMLDetailsElement
										).open,
									)}
							>
								<summary>
									<span>{shareMenu.label}</span>
									<ChevronDown size={14} strokeWidth={2.2} />
								</summary>
								<div class="dropdown-panel">
									<div class="dropdown-items">
										{#each shareMenu.items as item (item)}
											<button
												type="button"
												class="dropdown-item"
											>
												{item}
											</button>
										{/each}
									</div>
								</div>
							</details>
							{#if session}
								<details
									class="dropdown-badge dropdown-badge--right dropdown-badge--icon"
									open={activeTopbarMenu ===
										settingsMenuLabel}
									ontoggle={(event) =>
										handleTopbarMenuToggle(
											settingsMenuLabel,
											(
												event.currentTarget as HTMLDetailsElement
											).open,
										)}
								>
									<summary
										aria-label="Open settings"
										title="Settings"
									>
										<Settings2
											size={14}
											strokeWidth={2.2}
										/>
									</summary>
									<div class="dropdown-panel settings-panel">
										<p class="dropdown-label">Profile</p>
										<form
											class="settings-panel__form"
											onsubmit={(event) => {
												event.preventDefault();
												void handleUsernameSubmit();
											}}
										>
											<label
												class="settings-panel__field"
											>
												<span
													class="settings-panel__label"
													>Username</span
												>
												<input
													bind:value={usernameDraft}
													class="settings-panel__input"
													type="text"
													maxlength="32"
													autocomplete="nickname"
													spellcheck="false"
													placeholder="Guest"
												/>
											</label>
											{#if usernameErrorMessage}
												<p
													class="settings-panel__error"
												>
													{usernameErrorMessage}
												</p>
											{/if}
											<div
												class="settings-panel__actions"
											>
												<button
													type="submit"
													class="settings-panel__submit"
													disabled={usernameSaving}
												>
													{usernameSaving
														? "Saving..."
														: "Save"}
												</button>
											</div>
										</form>
									</div>
								</details>
							{/if}
						{:else}
							<button
								type="button"
								class="menu-badge-button menu-badge-button--disabled"
								disabled
							>
								<span>{shareMenu.label}</span>
								<ChevronDown size={14} strokeWidth={2.2} />
							</button>
						{/if}
					</div>
				</div>
			</div>
		</aside>
	</section>
</div>

<style>
	.editor-page {
		padding: 0 0 28px;
		--editor-column-width: 960px;
		--editor-column-min-width: 840px;
		--side-rail-min-width: 0px;
		--side-rail-top-offset: 22px;
		--menu-badge-height: calc(
			1em + (var(--presence-chip-padding-y, 4px) * 2) + 2px
		);
		--menu-badge-radius: 8px;
		--presence-chip-padding-y: 4px;
		--presence-chip-padding-x: 10px;
		--presence-chip-font-size: 0.84rem;
		--presence-chip-gap: 8px;
		--presence-swatch-size: 10px;
	}

	.editor-layout {
		display: grid;
		grid-template-columns:
			minmax(var(--side-rail-min-width), 1fr)
			minmax(var(--editor-column-min-width), var(--editor-column-width))
			minmax(var(--side-rail-min-width), 1fr);
		gap: clamp(18px, 2.4vw, 36px);
		align-items: start;
		min-height: 100vh;
		padding: 0 clamp(16px, 2vw, 28px);
		box-sizing: border-box;
	}

	.side-rail {
		min-width: 0;
		align-self: stretch;
	}

	.side-rail__inner {
		position: sticky;
		top: var(--side-rail-top-offset);
		display: grid;
		gap: 14px;
		align-content: start;
		padding-top: 0;
	}

	.side-rail__inner--right {
		justify-items: end;
	}

	.side-rail__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
	}

	.side-rail__actions--right {
		justify-content: flex-end;
	}

	.editor-content {
		display: grid;
		gap: 0;
	}

	.editor-pane {
		width: 100%;
		min-width: 0;
	}

	.topbar-left {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		align-items: flex-start;
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
		border-radius: var(--menu-badge-radius);
		background: var(--surface-overlay);
		color: var(--text);
		font-size: var(--presence-chip-font-size, 0.84rem);
		font-weight: 500;
		cursor: pointer;
		transition:
			background 120ms ease,
			border-color 120ms ease,
			color 120ms ease;
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

	.dropdown-badge--icon summary {
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
		padding: 0;
		border: 1px solid var(--line);
		border-radius: var(--menu-badge-radius);
		background: var(--surface-overlay);
		box-shadow: var(--shadow);
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
		gap: 0;
	}

	.settings-panel {
		min-width: 260px;
	}

	.settings-panel__form {
		display: grid;
		gap: 10px;
	}

	.settings-panel__field {
		display: grid;
		gap: 6px;
	}

	.settings-panel__label {
		font-size: 0.8rem;
		color: var(--muted);
	}

	.settings-panel__input {
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--surface-overlay);
		color: var(--text);
		outline: 0;
		transition:
			border-color 120ms ease,
			background 120ms ease;
	}

	.settings-panel__input:focus {
		border-color: var(--surface-overlay-border);
		background: var(--surface-overlay-medium);
	}

	.settings-panel__actions {
		display: flex;
		justify-content: flex-end;
	}

	.settings-panel__submit {
		padding: 9px 12px;
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--surface-overlay);
		color: var(--text);
		font-size: 0.82rem;
		font-weight: 500;
		cursor: pointer;
		transition:
			background 120ms ease,
			border-color 120ms ease;
	}

	.settings-panel__submit:hover:not(:disabled),
	.settings-panel__submit:focus-visible {
		background: var(--surface-overlay-medium);
		border-color: var(--surface-overlay-border);
	}

	.settings-panel__submit:disabled {
		cursor: wait;
		opacity: 0.72;
	}

	.settings-panel__error {
		margin: 0;
		font-size: 0.78rem;
		color: var(--accent-strong);
	}

	.dropdown-item {
		width: 100%;
		display: flex;
		align-items: center;
		block-size: var(--menu-badge-height);
		padding: 0 10px;
		border: 0;
		border-radius: var(--menu-badge-radius);
		background: transparent;
		color: var(--text);
		font-size: var(--presence-chip-font-size, 0.84rem);
		text-align: left;
		cursor: pointer;
		transition: background 120ms ease, color 120ms ease;
	}

	.dropdown-item:hover,
	.dropdown-item:focus-visible {
		background: var(--surface-overlay-medium);
		border-radius: 0;
	}

	.dropdown-item--danger {
		border-top: 1px solid var(--line);
		border-radius: 0;
		color: var(--danger);
	}

	.topbar-meta {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: flex-end;
		gap: 12px;
		min-width: 0;
	}

	.search-modal-layer {
		position: fixed;
		inset: 0;
		z-index: 80;
		display: grid;
		place-items: start center;
		padding: 48px 16px 24px;
	}

	.search-modal-backdrop {
		position: absolute;
		inset: 0;
		border: 0;
		padding: 0;
		background: var(--modal-backdrop);
		cursor: default;
	}

	.search-modal {
		position: relative;
		z-index: 1;
		width: min(560px, 100%);
		padding: 14px;
		border: 1px solid var(--line);
		border-radius: 22px;
		background: var(--modal-bg);
		box-shadow: var(--shadow);
		backdrop-filter: blur(18px);
	}

	.search-modal__input-wrap {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: 10px;
		padding: 0 4px 12px;
		border-bottom: 1px solid var(--line);
		color: var(--muted);
	}

	.search-modal__input {
		width: 100%;
		padding: 10px 0;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--text);
		font-size: 1rem;
	}

	.search-modal__input::placeholder {
		color: var(--muted);
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
		transition:
			background 120ms ease,
			border-color 120ms ease;
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

	@media (max-width: 1320px) {
		.editor-page {
			--editor-column-width: 920px;
			--editor-column-min-width: 760px;
		}
	}

	@media (max-width: 1180px) {
		.editor-page {
			--side-rail-top-offset: 16px;
		}

		.editor-layout {
			grid-template-columns: minmax(0, 1fr);
			gap: 18px;
			padding-left: 16px;
			padding-right: 16px;
		}

		.side-rail--left {
			order: 1;
		}

		.side-rail--right {
			order: 2;
		}

		.editor-pane {
			order: 3;
		}

		.side-rail__inner {
			position: static;
			min-height: auto;
			padding-top: 0;
		}

		.side-rail__inner--right {
			justify-items: start;
		}

		.topbar-left {
			flex-direction: row;
			flex-wrap: wrap;
		}

		.topbar-meta {
			align-items: flex-start;
		}

		.side-rail__actions--right {
			justify-content: flex-start;
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
			--presence-chip-padding-y: 5px;
			--presence-chip-padding-x: 11px;
			--presence-chip-font-size: 0.87rem;
			--presence-swatch-size: 11px;
		}
	}

	@media (min-resolution: 2dppx) {
		.editor-page {
			--presence-chip-padding-y: 6px;
			--presence-chip-padding-x: 12px;
			--presence-chip-font-size: 0.9rem;
			--presence-swatch-size: 12px;
		}
	}
</style>
