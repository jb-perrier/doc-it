<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { Editor } from '@tiptap/core';
	import {
		Bold,
		Code2,
		Italic,
		Strikethrough,
		Underline
	} from 'lucide-svelte';
	import type * as Y from 'yjs';

	import {
		createEditor,
		getEditorFormattingState,
		getEmptyFormattingState,
		toggleEditorFormatting
	} from '$lib/editor/tiptap';
	import type { EditorFormattingState, FormattingBadgeKey, PeerPresence } from '$lib/types';

	let {
		title,
		doc,
		peers,
		onTitleChange,
		onSelectionChange
	} = $props<{
		title: string;
		doc: Y.Doc;
		peers: PeerPresence[];
		onTitleChange: (title: string) => void;
		onSelectionChange: (anchor: number, head: number) => void;
	}>();

	const formatBadges: Array<{
		key: FormattingBadgeKey;
		label: string;
		icon: typeof Bold;
	}> = [
		{ key: 'bold', label: 'Bold', icon: Bold },
		{ key: 'italic', label: 'Italic', icon: Italic },
		{ key: 'underline', label: 'Underline', icon: Underline },
		{ key: 'strike', label: 'Strike', icon: Strikethrough },
		{ key: 'code', label: 'Code', icon: Code2 }
	];

	let host = $state<HTMLElement | null>(null);
	let titleElement = $state<HTMLDivElement | null>(null);
	let editor = $state<Editor | null>(null);
	let floatingToolbarElement = $state<HTMLDivElement | null>(null);
	let cursorOverlays = $state<Array<{ clientId: string; name: string; color: string; left: number; top: number }>>([]);
	let formatting = $state<EditorFormattingState>(getEmptyFormattingState());
	let isEditorFocused = $state(false);
	let isFloatingToolbarHovered = $state(false);
	let floatingToolbarVisible = $state(false);
	let floatingToolbarPosition = $state({ left: 0, top: 0 });
	let lastPointerPosition = $state<{ x: number; y: number } | null>(null);
	let floatingToolbarHideTimer: number | null = null;

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
		const currentTitle = (titleElement.textContent ?? '').trim();
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
			}
		});

		if (titleElement) {
			titleElement.textContent = title.trim();
		}

		syncEditorUi(editor);

		const handleViewportChange = () => {
			rebuildCursorOverlays();
			syncFloatingToolbar();
		};
		window.addEventListener('pointermove', handleWindowPointerMove);
		window.addEventListener('resize', handleViewportChange);
		host.addEventListener('scroll', handleViewportChange);

		return () => {
			window.removeEventListener('pointermove', handleWindowPointerMove);
			window.removeEventListener('resize', handleViewportChange);
			host?.removeEventListener('scroll', handleViewportChange);
		};
	});

	onDestroy(() => {
		clearFloatingToolbarHideTimer();
		editor?.destroy();
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
			.filter((peer: PeerPresence) => typeof peer.anchor === 'number')
			.map((peer: PeerPresence) => {
				const position = Math.min(Math.max(peer.anchor ?? 1, 1), maxPosition);
				const coords = currentEditor.view.coordsAtPos(position);
				return {
					clientId: peer.clientId,
					name: peer.name,
					color: peer.color,
					left: coords.left - rootBounds.left,
					top: coords.top - rootBounds.top
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
		syncFloatingToolbar();
	}

	function syncFloatingToolbar() {
		const currentEditor = editor;
		if (!currentEditor || !host) {
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
			Math.max(toolbarWidth / 2 + 12, window.innerWidth - toolbarWidth / 2 - 12)
		);

		floatingToolbarPosition = {
			left: clampedLeft,
			top: Math.max(caret.top, 12)
		};

		if (isFloatingToolbarHovered) {
			floatingToolbarVisible = isEditorFocused;
			return;
		}

		floatingToolbarVisible = isPointerNearCaret(caret);
	}

	function getCaretCoordinates(currentEditor: Editor) {
		const maxPosition = Math.max(currentEditor.state.doc.content.size, 1);
		const head = Math.min(Math.max(currentEditor.state.selection.head, 1), maxPosition);

		try {
			return currentEditor.view.coordsAtPos(head);
		} catch {
			return null;
		}
	}

	function isPointerNearCaret(caret: { left: number; right: number; top: number; bottom: number }) {
		if (!isEditorFocused || !lastPointerPosition) {
			return false;
		}

		const pointerWithinHorizontalReach =
			lastPointerPosition.x >= caret.left - 132 && lastPointerPosition.x <= caret.right + 132;
		const pointerWithinVerticalReach =
			lastPointerPosition.y >= caret.top - 144 && lastPointerPosition.y <= caret.bottom + 64;

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
		floatingToolbarVisible = isEditorFocused;
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

	function handleTitleInput() {
		if (!titleElement) {
			return;
		}

		const nextTitle = normalizeTitle(titleElement.textContent ?? '');
		if ((titleElement.textContent ?? '') !== nextTitle) {
			titleElement.textContent = nextTitle;
			placeCaretAtEnd(titleElement);
		}

		onTitleChange(nextTitle);
	}

	function handleTitleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
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
		return value.replace(/[\r\n]+/g, ' ').replace(/\s+/g, ' ').trim();
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
</script>

<div class="editor-shell">
	<div class="editor-canvas">
		<div class="editor-title-wrap">
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
				<div class="remote-cursor" style={`left:${overlay.left}px;top:${overlay.top}px;color:${overlay.color};`}>
					<span class="remote-cursor__label" style={`background:${overlay.color};`}>
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
		margin: 0 0 1.2rem;
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
		box-shadow: var(--shadow);
		backdrop-filter: blur(16px);
		pointer-events: none;
		opacity: 0;
		transform: translate(-50%, calc(-100% - 6px)) scale(0.96);
		transform-origin: bottom center;
		transition: opacity 140ms ease, transform 140ms ease;
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
		transition: background 120ms ease, color 120ms ease;
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
		border-radius: 18px;
		background: var(--code-bg);
		border: 1px solid var(--code-border);
		color: var(--code-text);
		overflow-x: auto;
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

	:global(.doc-editor code) {
		font-family: 'IBM Plex Mono', 'Cascadia Code', monospace;
		border-radius: 4px;
		background: var(--surface-overlay-strong);
	}

	:global(.doc-editor li > p),
	:global(.doc-editor li > div > p) {
		margin: 0;
	}

	:global(.doc-editor ul:not([data-type='taskList'])) {
		list-style: none;
		padding-left: 1.45rem;
	}

	:global(.doc-editor ul:not([data-type='taskList']) > li) {
		position: relative;
	}

	:global(.doc-editor ul:not([data-type='taskList']) > li + li),
	:global(.doc-editor ol > li + li),
	:global(.doc-editor ul[data-type='taskList'] li + li) {
		margin-top: 0.18rem;
	}

	:global(.doc-editor ul:not([data-type='taskList']) > li::before) {
		content: '•';
		position: absolute;
		left: -1rem;
		top: 0;
		transform: translateY(0.05rem);
		color: currentColor;
	}

	:global(.doc-editor ul[data-type='taskList']) {
		list-style: none;
		padding: 0;
	}

	:global(.doc-editor ul[data-type='taskList'] li) {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	:global(.doc-editor ul[data-type='taskList'] li > label) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex: 0 0 auto;
		margin-top: calc(0.5rem);
	}

	:global(.doc-editor ul[data-type='taskList'] li > label input[type='checkbox']) {
		margin: 0;
	}

	:global(.doc-editor ul[data-type='taskList'] li > div) {
		flex: 1 1 auto;
		min-width: 0;
	}

	:global(.doc-editor ul[data-type='taskList'] li > div > p:first-child) {
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
		height: 1.5rem;
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