import {
	Editor,
	Extension,
	textblockTypeInputRule,
	type InputRule,
	type JSONContent,
	wrappingInputRule
} from '@tiptap/core';
import Collaboration from '@tiptap/extension-collaboration';
import HorizontalRule from '@tiptap/extension-horizontal-rule';
import Link from '@tiptap/extension-link';
import TaskItem from '@tiptap/extension-task-item';
import TaskList from '@tiptap/extension-task-list';
import Underline from '@tiptap/extension-underline';
import StarterKit from '@tiptap/starter-kit';
import type * as Y from 'yjs';

import type { EditorFormattingState, FormattingBadgeKey } from '$lib/types';

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
	return new Editor({
		element: options.element,
		extensions: [
			MarkdownShortcuts,
			StarterKit.configure({
				heading: { levels: [1, 2, 3] }
			}),
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