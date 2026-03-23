import type { DocumentSummary } from "$lib/types";

export type DocumentSearchResult = {
    document: DocumentSummary;
    score: number;
};

type SearchOptions = {
    excludeDocumentId?: string;
    limit?: number;
};

export function getDocumentSearchResults(
    documents: DocumentSummary[],
    query: string,
    options: SearchOptions = {},
): DocumentSearchResult[] {
    const normalizedQuery = query.trim();
    if (!normalizedQuery) {
        return getAlphabeticalDocumentResults(documents, options);
    }

    const limit = options.limit ?? 8;

    return getSearchableDocuments(documents, options.excludeDocumentId)
        .map((item) => ({
            document: item,
            score: getFuzzyMatchScore(item.title, normalizedQuery),
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
        .slice(0, limit);
}

export function getAlphabeticalDocumentResults(
    documents: DocumentSummary[],
    options: SearchOptions = {},
): DocumentSearchResult[] {
    return getSearchableDocuments(documents, options.excludeDocumentId)
        .slice()
        .sort(compareDocumentsAlphabetically)
        .map((item) => ({ document: item, score: 0 }));
}

function getSearchableDocuments(
    documents: DocumentSummary[],
    excludeDocumentId?: string,
) {
    if (!excludeDocumentId) {
        return documents;
    }

    return documents.filter((item) => item.id !== excludeDocumentId);
}

function compareDocumentsAlphabetically(
    left: DocumentSummary,
    right: DocumentSummary,
) {
    const titleComparison = getDocumentSortLabel(left).localeCompare(
        getDocumentSortLabel(right),
        undefined,
        { sensitivity: "base" },
    );

    if (titleComparison !== 0) {
        return titleComparison;
    }

    return left.id.localeCompare(right.id, undefined, {
        sensitivity: "base",
    });
}

function getDocumentSortLabel(document: DocumentSummary) {
    return document.title.trim() || "Untitled";
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