import type { JSONContent } from '@tiptap/core';
import { fromMarkdown } from 'mdast-util-from-markdown';
import { gfmFromMarkdown, gfmToMarkdown } from 'mdast-util-gfm';
import { toMarkdown } from 'mdast-util-to-markdown';
import { gfm } from 'micromark-extension-gfm';

type MdNode = {
	type: string;
	value?: string;
	depth?: number;
	lang?: string | null;
	url?: string;
	ordered?: boolean;
	start?: number;
	checked?: boolean | null;
	children?: MdNode[];
};

export function markdownToDoc(markdown: string): JSONContent {
	const tree = fromMarkdown(markdown, {
		extensions: [gfm()],
		mdastExtensions: [gfmFromMarkdown()]
	}) as MdNode;

	return {
		type: 'doc',
		content: (tree.children ?? [])
			.map((child) => mdBlockToTiptap(child))
			.filter(Boolean) as JSONContent[]
	};
}

export function docToMarkdown(doc: JSONContent): string {
	const root: MdNode = {
		type: 'root',
		children: (doc.content ?? []).flatMap((node) => tiptapBlockToMd(node))
	};

	return toMarkdown(root as never, {
		extensions: [gfmToMarkdown()]
	}).trimEnd() + '\n';
}

function mdBlockToTiptap(node: MdNode): JSONContent | null {
	switch (node.type) {
		case 'paragraph':
			return withInline('paragraph', node.children);
		case 'heading':
			return {
				type: 'heading',
				attrs: { level: Math.min(Math.max(node.depth ?? 1, 1), 3) },
				content: mdInlineChildren(node.children)
			};
		case 'blockquote':
			return { type: 'blockquote', content: mapBlocks(node.children) };
		case 'code':
			return {
				type: 'codeBlock',
				attrs: { language: node.lang ?? null },
				content: node.value ? [{ type: 'text', text: node.value }] : []
			};
		case 'thematicBreak':
			return { type: 'horizontalRule' };
		case 'list':
			return mdListToTiptap(node);
		default:
			return null;
	}
}

function mdListToTiptap(node: MdNode): JSONContent | null {
	const items = node.children ?? [];
	const isTaskList = items.some((item) => item.checked === true || item.checked === false);

	if (isTaskList) {
		return {
			type: 'taskList',
			content: items.map((item) => ({
				type: 'taskItem',
				attrs: { checked: Boolean(item.checked) },
				content: mapBlocks(item.children)
			}))
		};
	}

	return {
		type: node.ordered ? 'orderedList' : 'bulletList',
		attrs: node.ordered ? { start: node.start ?? 1 } : undefined,
		content: items.map((item) => ({ type: 'listItem', content: mapBlocks(item.children) }))
	};
}

function mdInlineChildren(children: MdNode[] | undefined, marks: JSONContent['marks'] = []): JSONContent[] {
	return (children ?? []).flatMap((child) => mdInlineToTiptap(child, marks));
}

function mdInlineToTiptap(node: MdNode, marks: JSONContent['marks'] = []): JSONContent[] {
	switch (node.type) {
		case 'text':
			return node.value ? [{ type: 'text', text: node.value, marks }] : [];
		case 'strong':
			return mdInlineChildren(node.children, [...marks, { type: 'bold' }]);
		case 'emphasis':
			return mdInlineChildren(node.children, [...marks, { type: 'italic' }]);
		case 'delete':
			return mdInlineChildren(node.children, [...marks, { type: 'strike' }]);
		case 'inlineCode':
			return node.value ? [{ type: 'text', text: node.value, marks: [...marks, { type: 'code' }] }] : [];
		case 'link':
			return mdInlineChildren(node.children, [...marks, { type: 'link', attrs: { href: node.url } }]);
		case 'break':
			return [{ type: 'hardBreak' }];
		default:
			return [];
	}
}

function tiptapBlockToMd(node: JSONContent): MdNode[] {
	switch (node.type) {
		case 'paragraph':
			return [{ type: 'paragraph', children: tiptapInlineChildren(node.content) }];
		case 'heading':
			return [
				{
					type: 'heading',
					depth: Number(node.attrs?.level ?? 1),
					children: tiptapInlineChildren(node.content)
				}
			];
		case 'blockquote':
			return [{ type: 'blockquote', children: (node.content ?? []).flatMap((child) => tiptapBlockToMd(child)) }];
		case 'codeBlock':
			return [
				{
					type: 'code',
					lang: (node.attrs?.language as string | undefined) ?? null,
					value: extractText(node.content)
				}
			];
		case 'horizontalRule':
			return [{ type: 'thematicBreak' }];
		case 'bulletList':
			return [{ type: 'list', ordered: false, children: (node.content ?? []).map(listItemToMd) }];
		case 'orderedList':
			return [
				{
					type: 'list',
					ordered: true,
					start: Number(node.attrs?.start ?? 1),
					children: (node.content ?? []).map(listItemToMd)
				}
			];
		case 'taskList':
			return [
				{
					type: 'list',
					ordered: false,
					children: (node.content ?? []).map(taskItemToMd)
				}
			];
		default:
			return [];
	}
}

function tiptapInlineChildren(content: JSONContent[] | undefined): MdNode[] {
	return (content ?? []).flatMap((node) => tiptapInlineToMd(node));
}

function tiptapInlineToMd(node: JSONContent): MdNode[] {
	if (node.type === 'hardBreak') {
		return [{ type: 'break' }];
	}

	if (node.type !== 'text') {
		return [];
	}

	let current: MdNode = { type: 'text', value: node.text ?? '' };
	for (const mark of node.marks ?? []) {
		if (mark.type === 'bold') {
			current = { type: 'strong', children: [current] };
		} else if (mark.type === 'italic') {
			current = { type: 'emphasis', children: [current] };
		} else if (mark.type === 'strike') {
			current = { type: 'delete', children: [current] };
		} else if (mark.type === 'code') {
			current = { type: 'inlineCode', value: extractMdText(current) };
		} else if (mark.type === 'link') {
			current = {
				type: 'link',
				url: String(mark.attrs?.href ?? ''),
				children: current.type === 'inlineCode' ? [{ type: 'text', value: current.value ?? '' }] : [current]
			};
		}
	}

	return [current];
}

function listItemToMd(node: JSONContent): MdNode {
	return {
		type: 'listItem',
		children: (node.content ?? []).flatMap((child) => tiptapBlockToMd(child))
	};
}

function taskItemToMd(node: JSONContent): MdNode {
	return {
		type: 'listItem',
		checked: Boolean(node.attrs?.checked),
		children: (node.content ?? []).flatMap((child) => tiptapBlockToMd(child))
	};
}

function withInline(type: string, children: MdNode[] | undefined): JSONContent {
	return { type, content: mdInlineChildren(children) };
}

function mapBlocks(children: MdNode[] | undefined): JSONContent[] {
	return (children ?? []).map((child) => mdBlockToTiptap(child)).filter(Boolean) as JSONContent[];
}

function extractText(content: JSONContent[] | undefined): string {
	return (content ?? []).map((node) => node.text ?? '').join('');
}

function extractMdText(node: MdNode): string {
	if (node.type === 'text' || node.type === 'inlineCode') {
		return node.value ?? '';
	}

	return (node.children ?? []).map((child) => extractMdText(child)).join('');
}