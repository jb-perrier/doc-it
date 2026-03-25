<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import type { Editor } from "@tiptap/core";
	import {
		Bold,
		Check,
		Code2,
		Copy,
		Files,
		Italic,
		Strikethrough,
		Underline,
		X,
	} from "lucide-svelte";
	import type * as Y from "yjs";

	import {
		createEditor,
		getEditorFormattingState,
		getEmptyFormattingState,
		toggleEditorFormatting,
	} from "$lib/editor/tiptap";
	import FolderPathBadge from "$lib/components/FolderPathBadge.svelte";
	import { docToMarkdown } from "$lib/editor/markdown";
	import type { FolderPathSegment } from "$lib/folders/path";
	import type {
		EditorFormattingState,
		FormattingBadgeKey,
		PeerPresence,
	} from "$lib/types";

	let {
		title,
		doc,
		peers,
		folderPath = [],
		duplicatingDocument = false,
		deletingDocument = false,
		onTitleChange,
		onDuplicateDocument,
		onOpenFolderPath,
		onDeleteDocument,
		onSelectionChange,
	} = $props<{
		title: string;
		doc: Y.Doc;
		peers: PeerPresence[];
		folderPath?: FolderPathSegment[];
		duplicatingDocument?: boolean;
		deletingDocument?: boolean;
		onTitleChange: (title: string) => void;
		onDuplicateDocument: () => Promise<void>;
		onOpenFolderPath: () => void;
		onDeleteDocument: () => Promise<void>;
		onSelectionChange: (anchor: number, head: number) => void;
	}>();

	const formatBadges: Array<{
		key: FormattingBadgeKey;
		label: string;
		icon: typeof Bold;
	}> = [
		{ key: "bold", label: "Bold", icon: Bold },
		{ key: "italic", label: "Italic", icon: Italic },
		{ key: "underline", label: "Underline", icon: Underline },
		{ key: "strike", label: "Strike", icon: Strikethrough },
		{ key: "code", label: "Code", icon: Code2 },
	];

	let host = $state<HTMLElement | null>(null);
	let titleElement = $state<HTMLDivElement | null>(null);
	let editor = $state<Editor | null>(null);
	let floatingToolbarElement = $state<HTMLDivElement | null>(null);
	let cursorOverlays = $state<
		Array<{
			clientId: string;
			name: string;
			color: string;
			left: number;
			top: number;
			height: number;
		}>
	>([]);
	let formatting = $state<EditorFormattingState>(getEmptyFormattingState());
	let isEditorFocused = $state(false);
	let isFloatingToolbarHovered = $state(false);
	let floatingToolbarVisible = $state(false);
	let floatingToolbarPosition = $state({ left: 0, top: 0 });
	let lastPointerPosition = $state<{ x: number; y: number } | null>(null);
	let floatingToolbarHideTimer: number | null = null;
	let copyMarkdownFeedbackTimer: number | null = null;
	let copyMarkdownState = $state<"idle" | "copying" | "copied">("idle");

	$effect(() => {
		peers;
		editor;
		rebuildCursorOverlays();
	});

	$effect(() => {
		title;
		if (!titleElement || document.activeElement === titleElement) {
			return;
		}

		const nextTitle = title.trim();
		const currentTitle = (titleElement.textContent ?? "").trim();
		if (currentTitle !== nextTitle) {
			titleElement.textContent = nextTitle;
		}
	});

	$effect(() => {
		floatingToolbarElement;
		syncFloatingToolbar();
	});

	onMount(() => {
		if (!host) {
			return;
		}

		editor = createEditor({
			element: host,
			doc,
			onUpdate(currentEditor) {
				syncEditorUi(currentEditor);
				rebuildCursorOverlays();
			},
			onSelection(anchor, head) {
				const currentEditor = editor;
				onSelectionChange(anchor, head);
				if (currentEditor) {
					syncEditorUi(currentEditor);
				}
				rebuildCursorOverlays();
			},
			onFocusChange(focused, currentEditor) {
				isEditorFocused = focused;
				syncEditorUi(currentEditor);
			},
		});

		if (titleElement) {
			titleElement.textContent = title.trim();
		}

		syncEditorUi(editor);

		const handleViewportChange = () => {
			rebuildCursorOverlays();
			syncFloatingToolbar();
		};
		window.addEventListener("pointermove", handleWindowPointerMove);
		window.addEventListener("resize", handleViewportChange);
		host.addEventListener("scroll", handleViewportChange);

		return () => {
			window.removeEventListener("pointermove", handleWindowPointerMove);
			window.removeEventListener("resize", handleViewportChange);
			host?.removeEventListener("scroll", handleViewportChange);
		};
	});

	onDestroy(() => {
		clearFloatingToolbarHideTimer();
		clearCopyMarkdownFeedbackTimer();
		editor?.destroy();
	});

	const copyMarkdownLabel = $derived.by(() => {
		if (copyMarkdownState === "copying") {
			return "Copying markdown";
		}

		if (copyMarkdownState === "copied") {
			return "Copied markdown";
		}

		return "Copy markdown";
	});

	function rebuildCursorOverlays() {
		const currentEditor = editor;
		if (!currentEditor || !host) {
			cursorOverlays = [];
			return;
		}

		const rootBounds = host.getBoundingClientRect();
		const maxPosition = Math.max(currentEditor.state.doc.content.size, 1);

		cursorOverlays = peers
			.filter((peer: PeerPresence) => typeof peer.anchor === "number")
			.map((peer: PeerPresence) => {
				const position = Math.min(
					Math.max(peer.anchor ?? 1, 1),
					maxPosition,
				);
				const coords = currentEditor.view.coordsAtPos(position);
				return {
					clientId: peer.clientId,
					name: peer.name,
					color: peer.color,
					left: coords.left - rootBounds.left,
					top: coords.top - rootBounds.top,
					height: Math.max(coords.bottom - coords.top, 16),
				};
			});
	}

	function syncEditorUi(currentEditor: Editor | null) {
		if (!currentEditor) {
			formatting = getEmptyFormattingState();
			floatingToolbarVisible = false;
			return;
		}

		formatting = getEditorFormattingState(currentEditor);
		if (formatting.codeBlock) {
			floatingToolbarVisible = false;
			return;
		}

		syncFloatingToolbar();
	}

	function syncFloatingToolbar() {
		const currentEditor = editor;
		if (!currentEditor || !host) {
			floatingToolbarVisible = false;
			return;
		}

		if (currentEditor.isActive("codeBlock")) {
			floatingToolbarVisible = false;
			return;
		}

		const caret = getCaretCoordinates(currentEditor);
		if (!caret) {
			floatingToolbarVisible = false;
			return;
		}

		const toolbarWidth = floatingToolbarElement?.offsetWidth ?? 220;
		const clampedLeft = clamp(
			(caret.left + caret.right) / 2,
			toolbarWidth / 2 + 12,
			Math.max(
				toolbarWidth / 2 + 12,
				window.innerWidth - toolbarWidth / 2 - 12,
			),
		);

		floatingToolbarPosition = {
			left: clampedLeft,
			top: Math.max(caret.top, 12),
		};

		if (isFloatingToolbarHovered) {
			floatingToolbarVisible = isEditorFocused;
			return;
		}

		floatingToolbarVisible = isPointerNearCaret(caret);
	}

	function getCaretCoordinates(currentEditor: Editor) {
		const maxPosition = Math.max(currentEditor.state.doc.content.size, 1);
		const head = Math.min(
			Math.max(currentEditor.state.selection.head, 1),
			maxPosition,
		);

		try {
			return currentEditor.view.coordsAtPos(head);
		} catch {
			return null;
		}
	}

	function isPointerNearCaret(caret: {
		left: number;
		right: number;
		top: number;
		bottom: number;
	}) {
		if (!isEditorFocused || !lastPointerPosition) {
			return false;
		}

		const pointerWithinHorizontalReach =
			lastPointerPosition.x >= caret.left - 132 &&
			lastPointerPosition.x <= caret.right + 132;
		const pointerWithinVerticalReach =
			lastPointerPosition.y >= caret.top - 144 &&
			lastPointerPosition.y <= caret.bottom + 64;

		return pointerWithinHorizontalReach && pointerWithinVerticalReach;
	}

	function handleEditorPointerMove(event: PointerEvent) {
		clearFloatingToolbarHideTimer();
		lastPointerPosition = { x: event.clientX, y: event.clientY };
		syncFloatingToolbar();
	}

	function handleWindowPointerMove(event: PointerEvent) {
		lastPointerPosition = { x: event.clientX, y: event.clientY };

		if (!isFloatingToolbarHovered) {
			syncFloatingToolbar();
		}
	}

	function handleEditorPointerLeave() {
		if (!isFloatingToolbarHovered) {
			scheduleFloatingToolbarHide();
		}
	}

	function handleFloatingToolbarEnter() {
		clearFloatingToolbarHideTimer();
		isFloatingToolbarHovered = true;
		floatingToolbarVisible = isEditorFocused && !formatting.codeBlock;
	}

	function handleFloatingToolbarLeave() {
		isFloatingToolbarHovered = false;
		scheduleFloatingToolbarHide();
	}

	function handleFloatingToolbarPointerDown(event: PointerEvent) {
		event.preventDefault();
	}

	function handleFormattingAction(key: FormattingBadgeKey) {
		const currentEditor = editor;
		if (!currentEditor) {
			return;
		}

		toggleEditorFormatting(currentEditor, key);
		syncEditorUi(currentEditor);
	}

	async function handleCopyMarkdown() {
		const currentEditor = editor;
		if (!currentEditor || copyMarkdownState === "copying") {
			return;
		}

		copyMarkdownState = "copying";

		try {
			await writeTextToClipboard(buildMarkdownExport(currentEditor));
			copyMarkdownState = "copied";
			clearCopyMarkdownFeedbackTimer();
			copyMarkdownFeedbackTimer = window.setTimeout(() => {
				copyMarkdownFeedbackTimer = null;
				copyMarkdownState = "idle";
			}, 950);
		} catch {
			copyMarkdownState = "idle";
		}
	}

	function handleTitleInput() {
		if (!titleElement) {
			return;
		}

		const nextTitle = normalizeTitle(titleElement.textContent ?? "");
		if ((titleElement.textContent ?? "") !== nextTitle) {
			titleElement.textContent = nextTitle;
			placeCaretAtEnd(titleElement);
		}

		onTitleChange(nextTitle);
	}

	function handleTitleKeyDown(event: KeyboardEvent) {
		if (event.key === "Enter") {
			event.preventDefault();
		}
	}

	function handleTitleFocus() {
		isEditorFocused = false;
		floatingToolbarVisible = false;
	}

	function handleTitleBlur() {
		if (editor) {
			syncEditorUi(editor);
		}
	}

	function normalizeTitle(value: string): string {
		return value
			.replace(/[\r\n]+/g, " ")
			.replace(/\s+/g, " ")
			.trim();
	}

	function buildMarkdownExport(currentEditor: Editor): string {
		const normalizedTitle = normalizeTitle(title);
		const bodyMarkdown = docToMarkdown(currentEditor.getJSON()).trim();

		if (normalizedTitle && bodyMarkdown) {
			return `# ${normalizedTitle}\n\n${bodyMarkdown}\n`;
		}

		if (normalizedTitle) {
			return `# ${normalizedTitle}\n`;
		}

		return bodyMarkdown ? `${bodyMarkdown}\n` : "";
	}

	async function writeTextToClipboard(value: string) {
		if (navigator.clipboard?.writeText) {
			await navigator.clipboard.writeText(value);
			return;
		}

		const textarea = document.createElement("textarea");
		textarea.value = value;
		textarea.setAttribute("readonly", "true");
		textarea.style.position = "fixed";
		textarea.style.opacity = "0";
		document.body.append(textarea);
		textarea.select();

		try {
			document.execCommand("copy");
		} finally {
			textarea.remove();
		}
	}

	function placeCaretAtEnd(element: HTMLElement) {
		const selection = window.getSelection();
		if (!selection) {
			return;
		}

		const range = document.createRange();
		range.selectNodeContents(element);
		range.collapse(false);
		selection.removeAllRanges();
		selection.addRange(range);
	}

	function clamp(value: number, min: number, max: number) {
		return Math.min(Math.max(value, min), max);
	}

	function scheduleFloatingToolbarHide() {
		clearFloatingToolbarHideTimer();
		floatingToolbarHideTimer = window.setTimeout(() => {
			floatingToolbarHideTimer = null;
			syncFloatingToolbar();
		}, 140);
	}

	function clearFloatingToolbarHideTimer() {
		if (floatingToolbarHideTimer !== null) {
			clearTimeout(floatingToolbarHideTimer);
			floatingToolbarHideTimer = null;
		}
	}

	function clearCopyMarkdownFeedbackTimer() {
		if (copyMarkdownFeedbackTimer !== null) {
			clearTimeout(copyMarkdownFeedbackTimer);
			copyMarkdownFeedbackTimer = null;
		}
	}
</script>

<div class="editor-shell">
	<div class="editor-canvas">
		<div class="editor-title-wrap">
			<div class="editor-folder-row">
				<div class="editor-folder-path">
					{#if folderPath.length > 0}
						<button
							type="button"
							class="editor-folder-path__button"
							onclick={onOpenFolderPath}
							aria-label="Open this folder in search explorer"
							title="Open this folder in search explorer"
						>
							<FolderPathBadge segments={folderPath} size="sm" />
						</button>
					{/if}
				</div>
				<div class="editor-document-actions">
					<button
						type="button"
						class:editor-action-button--copied={copyMarkdownState ===
							"copied"}
						class="editor-action-button"
						onclick={() => void handleCopyMarkdown()}
						disabled={copyMarkdownState === "copying"}
						aria-label={copyMarkdownLabel}
						title={copyMarkdownLabel}
					>
						{#if copyMarkdownState === "copied"}
							<span
								class="editor-action-button__icon editor-action-button__icon--copied"
							>
								<Check size={14} strokeWidth={2.6} />
							</span>
						{:else}
							<span class="editor-action-button__icon">
								<Copy size={14} strokeWidth={2.2} />
							</span>
						{/if}
					</button>
					<button
						type="button"
						class="editor-action-button"
						onclick={() => void onDuplicateDocument()}
						disabled={duplicatingDocument || deletingDocument}
						aria-label={duplicatingDocument
							? "Duplicating document"
							: "Duplicate document"}
						title={duplicatingDocument
							? "Duplicating document"
							: "Duplicate document"}
					>
						<Files size={14} strokeWidth={2.2} />
					</button>
					<button
						type="button"
						class="editor-action-button editor-delete-button"
						onclick={() => void onDeleteDocument()}
						disabled={deletingDocument || duplicatingDocument}
						aria-label={deletingDocument
							? "Deleting document"
							: "Delete document"}
						title={deletingDocument
							? "Deleting document"
							: "Delete document"}
					>
						<X size={14} strokeWidth={2.2} />
					</button>
				</div>
			</div>
			<div
				bind:this={titleElement}
				class="editor-title"
				contenteditable="true"
				data-placeholder="Untitled"
				role="textbox"
				spellcheck="false"
				tabindex="0"
				onfocus={handleTitleFocus}
				onblur={handleTitleBlur}
				oninput={handleTitleInput}
				onkeydown={handleTitleKeyDown}
			></div>
		</div>

		<div class="editor-host">
			<div
				bind:this={host}
				class="editor-surface"
				role="presentation"
				onpointermove={handleEditorPointerMove}
				onpointerleave={handleEditorPointerLeave}
			></div>
			{#each cursorOverlays as overlay (overlay.clientId)}
				<div
					class="remote-cursor"
					style={`left:${overlay.left}px;top:${overlay.top}px;height:${overlay.height}px;color:${overlay.color};`}
				>
					<span
						class="remote-cursor__label"
						style={`background:${overlay.color};`}
					>
						{overlay.name}
					</span>
				</div>
			{/each}
		</div>
	</div>
</div>

<div
	bind:this={floatingToolbarElement}
	class:floating-toolbar--visible={floatingToolbarVisible}
	class="floating-toolbar"
	role="toolbar"
	aria-label="Text formatting"
	tabindex="-1"
	style={`left:${floatingToolbarPosition.left}px;top:${floatingToolbarPosition.top}px;`}
	onpointerdown={handleFloatingToolbarPointerDown}
	onpointerenter={handleFloatingToolbarEnter}
	onpointerleave={handleFloatingToolbarLeave}
>
	{#each formatBadges as badge (badge.key)}
		<button
			type="button"
			class:format-badge--active={formatting[badge.key]}
			class="format-badge"
			onclick={() => handleFormattingAction(badge.key)}
			title={badge.label}
			aria-label={badge.label}
		>
			<badge.icon size={14} strokeWidth={2.1} />
		</button>
	{/each}
</div>

<style>
	.editor-shell {
		display: block;
	}

	.editor-canvas {
		padding: 24px clamp(18px, 4vw, 56px) 40px;
		min-height: 70vh;
		background: transparent;
	}

	.editor-title-wrap,
	.editor-host {
		max-width: 840px;
		margin: 0 auto;
		width: 100%;
	}

	.editor-title {
		min-height: 2.8rem;
		margin: 0;
		outline: none;
		font-size: 2.5rem;
		font-weight: 700;
		line-height: 1.08;
		letter-spacing: -0.04em;
		color: var(--text);
		white-space: pre-wrap;
		word-break: break-word;
	}

	.editor-title:empty::before {
		content: attr(data-placeholder);
		color: var(--muted);
	}

	.editor-folder-path {
		margin: 0 0 0.85rem;
		min-width: 0;
	}

	.editor-folder-path__button {
		display: inline-flex;
		align-items: center;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
		color: inherit;
	}

	.editor-folder-path__button:hover :global(.folder-path-badge),
	.editor-folder-path__button:focus-visible :global(.folder-path-badge) {
		background: transparent;
		color: var(--text-soft);
	}

	.editor-folder-path__button:focus-visible {
		outline: none;
	}

	.editor-folder-path__button :global(.folder-path-badge) {
		background: transparent;
	}

	.editor-folder-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin: 0 0 0.85rem;
	}

	.editor-folder-row .editor-folder-path {
		margin: 0;
		flex: 1 1 auto;
	}

	.editor-document-actions {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		flex: 0 0 auto;
	}

	.editor-action-button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: 8px;
		background: transparent;
		color: var(--muted);
		cursor: pointer;
		transition:
			background 120ms ease,
			box-shadow 220ms ease,
			color 120ms ease,
			opacity 120ms ease,
			transform 180ms ease;
	}

	.editor-action-button:hover:not(:disabled),
	.editor-action-button:focus-visible {
		background: var(--surface-overlay-medium);
		color: var(--text-soft);
		outline: none;
	}

	.editor-delete-button:hover:not(:disabled),
	.editor-delete-button:focus-visible {
		color: var(--danger);
	}

	.editor-action-button--copied {
		background: color-mix(in srgb, #38c172 18%, transparent);
		color: #38c172;
		box-shadow: 0 0 0 1px color-mix(in srgb, #38c172 42%, transparent);
		animation: copy-success-pulse 520ms ease;
	}

	.editor-action-button__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		transform-origin: center;
	}

	.editor-action-button__icon--copied {
		transform: scale(1);
		animation: copy-check-pop 240ms cubic-bezier(0.2, 0.9, 0.2, 1.2);
	}

	.editor-action-button:disabled {
		cursor: default;
		opacity: 0.56;
	}

	@keyframes copy-success-pulse {
		0% {
			transform: scale(1);
			box-shadow: 0 0 0 0 color-mix(in srgb, #38c172 0%, transparent);
		}

		45% {
			transform: scale(1.08);
			box-shadow: 0 0 0 6px color-mix(in srgb, #38c172 16%, transparent);
		}

		100% {
			transform: scale(1);
			box-shadow: 0 0 0 1px color-mix(in srgb, #38c172 42%, transparent);
		}
	}

	@keyframes copy-check-pop {
		0% {
			opacity: 0;
			transform: scale(0.7);
		}

		100% {
			opacity: 1;
			transform: scale(1);
		}
	}

	.editor-host {
		position: relative;
	}

	.editor-surface {
		width: 100%;
	}

	.floating-toolbar {
		position: fixed;
		z-index: 35;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: nowrap;
		gap: 6px;
		width: max-content;
		max-width: calc(100% - 24px);
		padding: 6px;
		border: 1px solid var(--line);
		border-radius: 10px;
		background: var(--dropdown-panel-bg, var(--panel));
		box-shadow: none;
		backdrop-filter: blur(16px);
		pointer-events: none;
		opacity: 0;
		transform: translate(-50%, calc(-100% - 6px)) scale(0.96);
		transform-origin: bottom center;
		transition:
			opacity 140ms ease,
			transform 140ms ease;
	}

	.floating-toolbar--visible {
		pointer-events: auto;
		opacity: 1;
		transform: translate(-50%, calc(-100% - 8px)) scale(1);
	}

	.format-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: 8px;
		background: transparent;
		color: var(--text);
		cursor: pointer;
		transition:
			background 120ms ease,
			color 120ms ease;
	}

	.format-badge:hover,
	.format-badge--active {
		background: var(--surface-overlay-medium);
	}

	.format-badge--active {
		color: var(--accent-strong);
	}

	:global(.doc-editor) {
		outline: none;
		min-height: 62vh;
		width: 100%;
		font-size: 1.05rem;
		line-height: 1.7;
		color: var(--text);
	}

	:global(.doc-editor h1),
	:global(.doc-editor h2),
	:global(.doc-editor h3) {
		line-height: 1.08;
		letter-spacing: -0.04em;
	}

	:global(.doc-editor h1) {
		font-size: 2.5rem;
	}

	:global(.doc-editor h2) {
		font-size: 1.8rem;
	}

	:global(.doc-editor blockquote) {
		margin: 1.4rem 0;
		padding-left: 1rem;
		border-left: 3px solid var(--blockquote-line);
		color: var(--muted);
	}

	:global(.doc-editor pre) {
		padding: 1rem;
		border-radius: var(--menu-badge-radius, 8px);
		background: var(--code-bg);
		border: 1px solid var(--code-border);
		color: var(--code-text);
		overflow-x: auto;
	}

	:global(.doc-editor .code-block-node) {
		position: relative;
		margin: 1.4rem 0;
		--syntax-text: var(--code-text);
		--syntax-comment: rgba(177, 182, 190, 0.72);
		--syntax-keyword: #ff8f6b;
		--syntax-string: #9ad17d;
		--syntax-number: #f3c969;
		--syntax-title: #73c4ff;
		--syntax-meta: #c7a6ff;
		--syntax-attr: #f7b267;
		--syntax-variable: #ffd3a1;
	}

	:global(:root[data-theme="light"] .doc-editor .code-block-node) {
		--syntax-comment: rgba(91, 104, 123, 0.76);
		--syntax-keyword: #355fc0;
		--syntax-string: #2f7a56;
		--syntax-number: #0f7490;
		--syntax-title: #0d63a8;
		--syntax-meta: #7650b8;
		--syntax-attr: #9b4d8f;
		--syntax-variable: #b0476b;
	}

	:global(.doc-editor .code-block-node__toolbar) {
		position: absolute;
		top: 0.75rem;
		left: 0.85rem;
		z-index: 1;
	}

	:global(.doc-editor .code-block-node__language) {
		appearance: none;
		padding: 0.36rem 1.9rem 0.36rem 0.7rem;
		border-radius: 999px;
		border: none;
		background: transparent;
		color: var(--text-soft);
		font: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		line-height: 1.1;
		cursor: pointer;
		box-shadow: none;
	}

	:global(.doc-editor .code-block-node__language:focus) {
		outline: none;
	}

	:global(.doc-editor .code-block-node pre) {
		margin: 0;
		padding-top: 3rem;
	}

	:global(.doc-editor .code-block-node pre code) {
		display: block;
		padding-inline: 2rem;
		background: transparent;
		color: var(--syntax-text);
	}

	:global(.doc-editor .code-block-node .hljs-comment),
	:global(.doc-editor .code-block-node .hljs-quote) {
		color: var(--syntax-comment);
	}

	:global(.doc-editor .code-block-node .hljs-keyword),
	:global(.doc-editor .code-block-node .hljs-selector-tag),
	:global(.doc-editor .code-block-node .hljs-literal),
	:global(.doc-editor .code-block-node .hljs-name),
	:global(.doc-editor .code-block-node .hljs-section),
	:global(.doc-editor .code-block-node .hljs-link) {
		color: var(--syntax-keyword);
	}

	:global(.doc-editor .code-block-node .hljs-string),
	:global(.doc-editor .code-block-node .hljs-regexp),
	:global(.doc-editor .code-block-node .hljs-addition),
	:global(.doc-editor .code-block-node .hljs-attribute),
	:global(.doc-editor .code-block-node .hljs-template-tag),
	:global(.doc-editor .code-block-node .hljs-template-variable) {
		color: var(--syntax-string);
	}

	:global(.doc-editor .code-block-node .hljs-number),
	:global(.doc-editor .code-block-node .hljs-symbol),
	:global(.doc-editor .code-block-node .hljs-bullet),
	:global(.doc-editor .code-block-node .hljs-built_in),
	:global(.doc-editor .code-block-node .hljs-type) {
		color: var(--syntax-number);
	}

	:global(.doc-editor .code-block-node .hljs-title),
	:global(.doc-editor .code-block-node .hljs-title.class_),
	:global(.doc-editor .code-block-node .hljs-title.function_) {
		color: var(--syntax-title);
	}

	:global(.doc-editor .code-block-node .hljs-meta),
	:global(.doc-editor .code-block-node .hljs-doctag) {
		color: var(--syntax-meta);
	}

	:global(.doc-editor .code-block-node .hljs-attr),
	:global(.doc-editor .code-block-node .hljs-selector-class),
	:global(.doc-editor .code-block-node .hljs-selector-id),
	:global(.doc-editor .code-block-node .hljs-selector-attr) {
		color: var(--syntax-attr);
	}

	:global(.doc-editor .code-block-node .hljs-variable),
	:global(.doc-editor .code-block-node .hljs-property),
	:global(.doc-editor .code-block-node .hljs-params),
	:global(.doc-editor .code-block-node .hljs-deletion) {
		color: var(--syntax-variable);
	}

	:global(.doc-editor p),
	:global(.doc-editor li),
	:global(.doc-editor strong),
	:global(.doc-editor h1),
	:global(.doc-editor h2),
	:global(.doc-editor h3) {
		color: var(--text);
	}

	:global(.doc-editor a) {
		color: var(--accent-strong);
	}

	:global(.doc-editor :not(pre) > code) {
		font-family: "IBM Plex Mono", "Cascadia Code", monospace;
		border-radius: 4px;
		background: var(--surface-overlay-strong);
	}

	:global(.doc-editor li > p),
	:global(.doc-editor li > div > p) {
		margin: 0;
	}

	:global(.doc-editor ul:not([data-type="taskList"])) {
		list-style: none;
		padding-left: 1.45rem;
	}

	:global(.doc-editor ul:not([data-type="taskList"]) > li) {
		position: relative;
	}

	:global(.doc-editor ul:not([data-type="taskList"]) > li + li),
	:global(.doc-editor ol > li + li),
	:global(.doc-editor ul[data-type="taskList"] li + li) {
		margin-top: 0.18rem;
	}

	:global(.doc-editor ul:not([data-type="taskList"]) > li::before) {
		content: "•";
		position: absolute;
		left: -1rem;
		top: 0;
		transform: translateY(0.05rem);
		color: currentColor;
	}

	:global(.doc-editor ul[data-type="taskList"]) {
		list-style: none;
		padding: 0;
	}

	:global(.doc-editor ul[data-type="taskList"] li) {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	:global(.doc-editor ul[data-type="taskList"] li > label) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex: 0 0 auto;
		margin-top: calc(0.5rem);
	}

	:global(
			.doc-editor
				ul[data-type="taskList"]
				li
				> label
				input[type="checkbox"]
		) {
		margin: 0;
	}

	:global(.doc-editor ul[data-type="taskList"] li > div) {
		flex: 1 1 auto;
		min-width: 0;
	}

	:global(.doc-editor ul[data-type="taskList"] li > div > p:first-child) {
		margin-top: 0;
	}

	:global(.doc-editor hr) {
		border: none;
		border-top: 1px solid var(--line-strong);
		margin: 2rem 0;
	}

	.remote-cursor {
		position: absolute;
		width: 2px;
		background: currentColor;
		pointer-events: none;
		z-index: 20;
	}

	.remote-cursor__label {
		position: absolute;
		top: -1.6rem;
		left: -0.2rem;
		padding: 3px 8px;
		border-radius: 999px;
		color: white;
		font-size: 0.72rem;
		font-weight: 600;
		white-space: nowrap;
	}

	@media (max-width: 980px) {
		.editor-canvas {
			padding-left: 16px;
			padding-right: 16px;
		}
	}

	@media (max-width: 720px) {
		.floating-toolbar {
			flex-wrap: wrap;
			width: min(180px, calc(100% - 24px));
		}
	}
</style>
