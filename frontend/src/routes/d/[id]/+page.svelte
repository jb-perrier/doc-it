<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount, tick } from "svelte";
	import { browser } from "$app/environment";
	import { ChevronDown, Search, Sun, Moon } from "lucide-svelte";
	import * as Y from "yjs";

	import { listDocuments, renameDocumentTitle } from "$lib/api/documents";
	import DocumentSearchWorkspace from "$lib/components/DocumentSearchWorkspace.svelte";
	import ProfileSettingsMenu from "$lib/components/ProfileSettingsMenu.svelte";
	import PresenceBar from "$lib/components/PresenceBar.svelte";
	import WorkspaceShell from "$lib/components/WorkspaceShell.svelte";
	import EditorShell from "$lib/components/EditorShell.svelte";
	import { RealtimeClient } from "$lib/realtime/client";
	import { getDocumentSearchResults } from "$lib/search/documents";
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
	let searchModeOpen = $state(false);
	let searchQuery = $state("");
	let searchResultsIndex = $state(0);
	let searchDocuments = $state<DocumentSummary[] | null>(null);
	let searchLoading = $state(false);
	let searchErrorMessage = $state("");
	let savedDocumentScrollY = 0;
	let hasSavedDocumentScroll = false;
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
		closeSearchMode({ restoreScroll: false });
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

	async function openSearchMode() {
		activeTopbarMenu = null;

		if (browser && !searchModeOpen) {
			savedDocumentScrollY = window.scrollY;
			hasSavedDocumentScroll = true;
		}

		searchModeOpen = true;
		searchQuery = "";
		searchResultsIndex = 0;

		if (browser) {
			await tick();
			window.scrollTo({ top: 0, behavior: "auto" });
		}

		await ensureSearchDocumentsLoaded();
	}

	function closeSearchMode(options: { restoreScroll?: boolean } = {}) {
		const { restoreScroll = true } = options;
		const scrollTarget =
			browser && restoreScroll && hasSavedDocumentScroll
				? savedDocumentScrollY
				: null;

		searchModeOpen = false;
		searchQuery = "";
		searchResultsIndex = 0;
		searchErrorMessage = "";
		hasSavedDocumentScroll = false;
		savedDocumentScrollY = 0;

		if (scrollTarget !== null) {
			void tick().then(() => {
				window.scrollTo({ top: scrollTarget, behavior: "auto" });
			});
		}
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

	function getSearchResults() {
		return getDocumentSearchResults(searchDocuments ?? [], searchQuery, {
			excludeDocumentId: data.id,
		});
	}

	function handleSearchInputKeyDown(event: KeyboardEvent) {
		const results = getSearchResults();

		if (event.key === "Escape") {
			event.preventDefault();
			closeSearchMode();
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

	function handleSearchQueryChange(value: string) {
		searchQuery = value;
	}

	function handleSearchResultHover(index: number) {
		searchResultsIndex = index;
	}

	async function openSearchResult(target: DocumentSummary | null) {
		if (!target) {
			return;
		}

		if (target.id === data.id) {
			closeSearchMode({ restoreScroll: false });
			return;
		}

		await goto(`/d/${target.id}`);
		closeSearchMode({ restoreScroll: false });
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

<WorkspaceShell>
	{#snippet leftRail()}
		<div class="topbar-left">
			{#if document}
				<details
					class="dropdown-badge"
					open={activeTopbarMenu === documentMenu.label}
					ontoggle={(event) =>
						handleTopbarMenuToggle(
							documentMenu.label,
							(event.currentTarget as HTMLDetailsElement).open,
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
									class:dropdown-item--danger={item ===
										"Delete"}
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
			{#if searchModeOpen}
				<button
					type="button"
					class="menu-badge-button"
					onclick={() => closeSearchMode()}
				>
					<span>Back to document</span>
				</button>
			{:else}
				<button
					type="button"
					class="menu-badge-button"
					onclick={() => void openSearchMode()}
				>
					<span>Search</span>
					<Search size={14} strokeWidth={2.2} />
				</button>
			{/if}
		</div>
	{/snippet}

	{#snippet stage()}
		<div class="editor-content">
			{#snippet searchInputLeading()}
				<Search size={18} strokeWidth={2.1} />
			{/snippet}

			{#if searchModeOpen}
				<DocumentSearchWorkspace
					query={searchQuery}
					results={getSearchResults()}
					selectedIndex={searchResultsIndex}
					loading={searchLoading}
					errorMessage={searchErrorMessage}
					inputLeading={searchInputLeading}
					onQueryChange={handleSearchQueryChange}
					onKeyDown={handleSearchInputKeyDown}
					onOpenResult={(target) => void openSearchResult(target)}
					onHoverResult={handleSearchResultHover}
				/>
			{:else}
				<div class="editor-stage">
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
			{/if}
		</div>
	{/snippet}

	{#snippet rightRail()}
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
								(event.currentTarget as HTMLDetailsElement)
									.open,
							)}
					>
						<summary>
							<span>{shareMenu.label}</span>
							<ChevronDown size={14} strokeWidth={2.2} />
						</summary>
						<div class="dropdown-panel">
							<div class="dropdown-items">
								{#each shareMenu.items as item (item)}
									<button type="button" class="dropdown-item">
										{item}
									</button>
								{/each}
							</div>
						</div>
					</details>
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
					{#if session}
						<ProfileSettingsMenu
							open={activeTopbarMenu === settingsMenuLabel}
							onToggle={(isOpen) =>
								handleTopbarMenuToggle(
									settingsMenuLabel,
									isOpen,
								)}
							username={usernameDraft}
							saving={usernameSaving}
							errorMessage={usernameErrorMessage}
							onUsernameInput={(value) => {
								usernameDraft = value;
							}}
							onSubmit={() => void handleUsernameSubmit()}
						/>
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
	{/snippet}
</WorkspaceShell>

<style>
	.editor-content {
		position: relative;
		display: grid;
		gap: 0;
		min-height: 70vh;
	}

	.editor-stage {
		min-height: 70vh;
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
	}
</style>
