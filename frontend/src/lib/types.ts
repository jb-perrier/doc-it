export type DocumentSummary = {
	id: string;
	folderId: string;
	title: string;
	updatedAt: string;
	createdAt: string;
};

export type DocumentRecord = DocumentSummary;

export type FolderSummary = {
	id: string;
	parentFolderId: string | null;
	name: string;
	createdAt: string;
	updatedAt: string;
};

export type SessionProfile = {
	clientId: string;
	name: string;
	color: string;
};

export type PeerPresence = {
	clientId: string;
	name: string;
	color: string;
	anchor?: number;
	head?: number;
};

export type EditorFormattingState = {
	bold: boolean;
	italic: boolean;
	underline: boolean;
	strike: boolean;
	code: boolean;
	heading1: boolean;
	heading2: boolean;
	heading3: boolean;
	blockquote: boolean;
	bulletList: boolean;
	orderedList: boolean;
	taskList: boolean;
	codeBlock: boolean;
};

export type FormattingBadgeKey = keyof EditorFormattingState;