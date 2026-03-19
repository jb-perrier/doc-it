import {
	Editor,
	Extension,
	textblockTypeInputRule,
	type InputRule,
	type JSONContent,
	type NodeViewRendererProps,
	wrappingInputRule
} from '@tiptap/core';
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import Collaboration from '@tiptap/extension-collaboration';
import HorizontalRule from '@tiptap/extension-horizontal-rule';
import Link from '@tiptap/extension-link';
import TaskItem from '@tiptap/extension-task-item';
import TaskList from '@tiptap/extension-task-list';
import Underline from '@tiptap/extension-underline';
import StarterKit from '@tiptap/starter-kit';
import { common, createLowlight } from 'lowlight';
import type * as Y from 'yjs';

import { markdownToDoc } from '$lib/editor/markdown';
import type { EditorFormattingState, FormattingBadgeKey } from '$lib/types';

const lowlight = createLowlight(common);

lowlight.registerAlias({
	bash: ['sh', 'zsh'],
	javascript: ['js'],
	markdown: ['md'],
	plaintext: ['plain', 'text', 'txt'],
	typescript: ['ts'],
	yaml: ['yml']
});

const CODE_BLOCK_LANGUAGE_OPTIONS = [
	{ value: 'plaintext', label: 'Plain text' },
	{ value: 'bash', label: 'Bash' },
	{ value: 'javascript', label: 'JavaScript' },
	{ value: 'typescript', label: 'TypeScript' },
	{ value: 'json', label: 'JSON' },
	{ value: 'markdown', label: 'Markdown' },
	{ value: 'html', label: 'HTML' },
	{ value: 'css', label: 'CSS' },
	{ value: 'scss', label: 'SCSS' },
	{ value: 'sql', label: 'SQL' },
	{ value: 'python', label: 'Python' },
	{ value: 'rust', label: 'Rust' },
	{ value: 'go', label: 'Go' },
	{ value: 'yaml', label: 'YAML' }
] as const;

const HighlightedCodeBlock = CodeBlockLowlight.configure({
	defaultLanguage: 'plaintext',
	lowlight
}).extend({
	addNodeView() {
		return ({ node, getPos, editor }: NodeViewRendererProps) => {
			let currentNode = node;

			const dom = document.createElement('div');
			dom.className = 'code-block-node';

			const toolbar = document.createElement('div');
			toolbar.className = 'code-block-node__toolbar';

			const languageSelect = document.createElement('select');
			languageSelect.className = 'code-block-node__language';
			languageSelect.setAttribute('aria-label', 'Code block language');

			for (const option of CODE_BLOCK_LANGUAGE_OPTIONS) {
				const element = document.createElement('option');
				element.value = option.value;
				element.textContent = option.label;
				languageSelect.append(element);
			}

			toolbar.append(languageSelect);

			const pre = document.createElement('pre');
			const code = document.createElement('code');
			pre.append(code);
			dom.append(toolbar, pre);

			const syncLanguageUi = () => {
				const language = normalizeCodeBlockLanguage(currentNode.attrs.language as string | null | undefined);
				languageSelect.value = language;
				code.className = getCodeBlockClassName(language);
				dom.dataset.language = language;
			};

			languageSelect.addEventListener('change', () => {
				const nextLanguage = normalizeCodeBlockLanguage(languageSelect.value);

				try {
					const position = getPos();
					if (typeof position !== 'number') {
						return;
					}

					const transaction = editor.state.tr.setNodeMarkup(position, undefined, {
						...currentNode.attrs,
						language: nextLanguage
					});
					editor.view.dispatch(transaction);
				} catch {
					languageSelect.value = normalizeCodeBlockLanguage(currentNode.attrs.language as string | null | undefined);
				}
			});

			syncLanguageUi();

			return {
				dom,
				contentDOM: code,
				update(updatedNode) {
					if (updatedNode.type.name !== currentNode.type.name) {
						return false;
					}

					currentNode = updatedNode;
					syncLanguageUi();
					return true;
				},
				stopEvent(event) {
					return event.target instanceof HTMLElement && languageSelect.contains(event.target);
				}
			};
		};
	}
});

const MarkdownShortcuts = Extension.create({
	name: 'markdownShortcuts',

	addInputRules() {
		const rules: InputRule[] = [];
		const { nodes } = this.editor.schema;

		if (nodes.heading) {
			rules.push(
				textblockTypeInputRule({
					find: /^(#{1,3})\s$/,
					type: nodes.heading,
					getAttributes: (match) => ({ level: match[1].length })
				})
			);
		}

		if (nodes.blockquote) {
			rules.push(
				wrappingInputRule({
					find: /^>\s$/,
					type: nodes.blockquote
				})
			);
		}

		if (nodes.bulletList) {
			rules.push(
				wrappingInputRule({
					find: /^([-+*])\s$/,
					type: nodes.bulletList
				})
			);
		}

		if (nodes.orderedList) {
			rules.push(
				wrappingInputRule({
					find: /^(\d+)\.\s$/,
					type: nodes.orderedList,
					getAttributes: (match) => ({ start: Number(match[1]) || 1 })
				})
			);
		}

		return rules;
	},

	addKeyboardShortcuts() {
		return {
			Space: () => applyMarkdownShortcut(this.editor)
		};
	}
});

export function createEditor(options: {
	element: HTMLElement;
	doc: Y.Doc;
	onUpdate: (editor: Editor) => void;
	onSelection: (anchor: number, head: number) => void;
	onFocusChange: (focused: boolean, editor: Editor) => void;
}): Editor {
	let editor!: Editor;

	editor = new Editor({
		element: options.element,
		extensions: [
			MarkdownShortcuts,
			StarterKit.configure({
				codeBlock: false,
				heading: { levels: [1, 2, 3] }
			}),
			HighlightedCodeBlock,
			Underline,
			HorizontalRule,
			TaskList,
			TaskItem.configure({ nested: false }),
			Link.configure({
				autolink: true,
				openOnClick: false,
				linkOnPaste: true
			}),
			Collaboration.configure({
				document: options.doc,
				field: 'content'
			})
		],
		editorProps: {
			attributes: {
				class: 'doc-editor ProseMirror'
			},
			handlePaste(view, event) {
				if (editor.isActive('codeBlock')) {
					return false;
				}

				const text = event.clipboardData?.getData('text/plain')?.replace(/\r\n/g, '\n') ?? '';
				const normalizedText = normalizeMarkdownPaste(text);
				if (!shouldParseMarkdownPaste(normalizedText)) {
					return false;
				}

				const content = markdownToDoc(normalizedText).content ?? [];
				if (!content.length) {
					return false;
				}

				event.preventDefault();
				return editor.chain().focus().insertContent(content).run();
			}
		},
		onUpdate: ({ editor }) => options.onUpdate(editor),
		onSelectionUpdate: ({ editor }) => {
			const { from, to } = editor.state.selection;
			options.onSelection(from, to);
		},
		onFocus: ({ editor }) => options.onFocusChange(true, editor),
		onBlur: ({ editor }) => options.onFocusChange(false, editor)
	});

	return editor;
}

export function getEmptyDoc(): JSONContent {
	return {
		type: 'doc',
		content: [{ type: 'paragraph' }]
	};
}

export function getEmptyFormattingState(): EditorFormattingState {
	return {
		bold: false,
		italic: false,
		underline: false,
		strike: false,
		code: false,
		heading1: false,
		heading2: false,
		heading3: false,
		blockquote: false,
		bulletList: false,
		orderedList: false,
		taskList: false,
		codeBlock: false
	};
}

export function getEditorFormattingState(editor: Editor): EditorFormattingState {
	return {
		bold: editor.isActive('bold'),
		italic: editor.isActive('italic'),
		underline: editor.isActive('underline'),
		strike: editor.isActive('strike'),
		code: editor.isActive('code'),
		heading1: editor.isActive('heading', { level: 1 }),
		heading2: editor.isActive('heading', { level: 2 }),
		heading3: editor.isActive('heading', { level: 3 }),
		blockquote: editor.isActive('blockquote'),
		bulletList: editor.isActive('bulletList'),
		orderedList: editor.isActive('orderedList'),
		taskList: editor.isActive('taskList'),
		codeBlock: editor.isActive('codeBlock')
	};
}

export function toggleEditorFormatting(editor: Editor, key: FormattingBadgeKey): boolean {
	const chain = editor.chain().focus();

	switch (key) {
		case 'bold':
			return toggleInlineMark(editor, 'bold', () => chain.toggleBold().run(), () => chain.extendMarkRange('bold').unsetBold().run());
		case 'italic':
			return toggleInlineMark(editor, 'italic', () => chain.toggleItalic().run(), () => chain.extendMarkRange('italic').unsetItalic().run());
		case 'underline':
			return toggleInlineMark(editor, 'underline', () => chain.toggleUnderline().run(), () => chain.extendMarkRange('underline').unsetUnderline().run());
		case 'strike':
			return toggleInlineMark(editor, 'strike', () => chain.toggleStrike().run(), () => chain.extendMarkRange('strike').unsetStrike().run());
		case 'code':
			return toggleInlineMark(editor, 'code', () => chain.toggleCode().run(), () => chain.extendMarkRange('code').unsetCode().run());
		case 'heading1':
			return chain.toggleHeading({ level: 1 }).run();
		case 'heading2':
			return chain.toggleHeading({ level: 2 }).run();
		case 'heading3':
			return chain.toggleHeading({ level: 3 }).run();
		case 'blockquote':
			return chain.toggleBlockquote().run();
		case 'bulletList':
			return chain.toggleBulletList().run();
		case 'orderedList':
			return chain.toggleOrderedList().run();
		case 'taskList':
			return chain.toggleTaskList().run();
		case 'codeBlock':
			return chain.toggleCodeBlock().run();
	}
}

function toggleInlineMark(
	editor: Editor,
	markName: 'bold' | 'italic' | 'underline' | 'strike' | 'code',
	toggle: () => boolean,
	unsetExpandedRange: () => boolean
): boolean {
	if (editor.state.selection.empty && editor.isActive(markName)) {
		return unsetExpandedRange();
	}

	return toggle();
}

function applyMarkdownShortcut(editor: Editor): boolean {
	const { state } = editor;
	const { selection } = state;
	if (!selection.empty) {
		return false;
	}

	const { $from } = selection;
	if ($from.parent.type.name !== 'paragraph') {
		return false;
	}

	const textBefore = $from.parent.textBetween(0, $from.parentOffset, '', '');
	if (!textBefore.trim()) {
		return false;
	}

	const blockStart = $from.pos - $from.parentOffset;
	const clearTrigger = () => editor.chain().focus().deleteRange({ from: blockStart, to: $from.pos });

	if (textBefore === '[ ]') {
		return clearTrigger().toggleTaskList().updateAttributes('taskItem', { checked: false }).run();
	}

	if (textBefore.toLowerCase() === '[x]') {
		return clearTrigger().toggleTaskList().updateAttributes('taskItem', { checked: true }).run();
	}

	if (textBefore === '```') {
		return clearTrigger().setCodeBlock().run();
	}

	if (textBefore === '---') {
		return clearTrigger().setHorizontalRule().run();
	}

	return false;
}

function shouldParseMarkdownPaste(text: string): boolean {
	const trimmed = text.trim();
	if (!trimmed) {
		return false;
	}

	if (isSingleLineBlockMarkdown(trimmed)) {
		return true;
	}

	if (!trimmed.includes('\n')) {
		return false;
	}

	const markdownSignals = [
		/^#{1,6}\s/m,
		/^```[\w-]*\s*$/m,
		/^>\s/m,
		/^[-+*]\s/m,
		/^\d+\.\s/m,
		/^\[(?: |x|X)\]\s/m,
		/^---$/m,
		/`[^`]+`/,
		/\*\*[^*]+\*\*/,
		/\[[^\]]+\]\([^\)]+\)/
	];

	return markdownSignals.some((pattern) => pattern.test(trimmed));
}

function normalizeMarkdownPaste(text: string): string {
	const normalized = text.replace(/\r\n/g, '\n').trim();
	if (!normalized.includes('\n')) {
		const trimmedStart = normalized.trimStart();
		if (isSingleLineBlockMarkdown(trimmedStart)) {
			return trimmedStart;
		}
	}

	return normalized;
}

function isSingleLineBlockMarkdown(text: string): boolean {
	return [
		/^#{1,6}\s+\S/,
		/^>\s+\S/,
		/^[-+*]\s+\S/,
		/^\d+\.\s+\S/,
		/^\[(?: |x|X)\]\s+\S/,
		/^```[\w-]*\s*$/,
		/^---$/
	].some((pattern) => pattern.test(text));
}

function normalizeCodeBlockLanguage(language: string | null | undefined) {
	const normalized = language?.trim().toLowerCase() ?? '';
	return normalized || 'plaintext';
}

function getCodeBlockClassName(language: string) {
	return `hljs language-${language}`;
}