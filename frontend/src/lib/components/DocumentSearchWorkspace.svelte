<script lang="ts">
    import type { Snippet } from "svelte";
    import { onMount, tick } from "svelte";

    import FolderPathBadge from "$lib/components/FolderPathBadge.svelte";
    import type { FolderPathSegment } from "$lib/folders/path";
    import type { DocumentSearchResult } from "$lib/search/documents";

    let {
        query,
        results,
        selectedIndex,
        loading,
        errorMessage,
        emptyMessage = "No documents match that search.",
        placeholder = "Find another document",
        autoFocus = true,
        inputLeading,
        getFolderPath,
        onQueryChange,
        onKeyDown,
        onOpenResult,
        onHoverResult,
    } = $props<{
        query: string;
        results: DocumentSearchResult[];
        selectedIndex: number;
        loading: boolean;
        errorMessage: string;
        emptyMessage?: string;
        placeholder?: string;
        autoFocus?: boolean;
        inputLeading?: Snippet;
        getFolderPath?: (
            document: DocumentSearchResult["document"],
        ) => FolderPathSegment[];
        onQueryChange: (value: string) => void;
        onKeyDown: (event: KeyboardEvent) => void;
        onOpenResult: (result: DocumentSearchResult["document"]) => void;
        onHoverResult?: (index: number) => void;
    }>();

    let inputElement = $state<HTMLInputElement | null>(null);

    onMount(() => {
        if (!autoFocus) {
            return;
        }

        void tick().then(() => {
            inputElement?.focus();
            inputElement?.select();
        });
    });

    function formatUpdatedAt(value: string) {
        return new Date(value).toLocaleString();
    }
</script>

<section class="search-workspace" aria-label="Search documents">
    <div class="search-workspace__inner">
        <div class="search-workspace__input-wrap">
            {#if inputLeading}
                {@render inputLeading()}
            {/if}
            <input
                bind:this={inputElement}
                class="search-workspace__input"
                type="text"
                value={query}
                {placeholder}
                spellcheck="false"
                autocomplete="off"
                oninput={(event) =>
                    onQueryChange(
                        (event.currentTarget as HTMLInputElement).value,
                    )}
                onkeydown={onKeyDown}
            />
        </div>

        {#if loading}
            <p class="search-workspace__state">Loading documents…</p>
        {:else if errorMessage}
            <p class="search-workspace__state search-workspace__state--error">
                {errorMessage}
            </p>
        {:else if results.length === 0}
            <p class="search-workspace__state">{emptyMessage}</p>
        {:else}
            <div
                class:search-workspace__results--compact={!query.trim()}
                class="search-workspace__results"
            >
                {#if query.trim()}
                    <p class="search-workspace__label">Matching documents</p>
                {/if}
                {#each results as result, index (result.document.id)}
                    {@const folderPath = getFolderPath?.(result.document) ?? []}
                    <button
                        type="button"
                        class:selected={index === selectedIndex}
                        class="search-result"
                        onclick={() => onOpenResult(result.document)}
                        onmousemove={() => onHoverResult?.(index)}
                    >
                        <span class="search-result__content">
                            <span class="search-result__title"
                                >{result.document.title || "Untitled"}</span
                            >
                            <span class="search-result__meta">
                                Updated {formatUpdatedAt(
                                    result.document.updatedAt,
                                )}
                            </span>
                        </span>

                        {#if folderPath.length > 0}
                            <span class="search-result__folder-path">
                                <FolderPathBadge segments={folderPath} />
                            </span>
                        {/if}
                    </button>
                {/each}
            </div>
        {/if}
    </div>
</section>

<style>
    .search-workspace {
        padding: 24px clamp(18px, 4vw, 56px) 40px;
        min-height: 70vh;
    }

    .search-workspace__inner {
        max-width: 840px;
        margin: 0 auto;
        display: grid;
        gap: 18px;
    }

    .search-workspace__input-wrap {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        align-items: center;
        gap: 14px;
        padding: 0 0 18px;
        border-bottom: 1px solid var(--line);
        color: var(--muted);
    }

    .search-workspace__input {
        width: 100%;
        padding: 0;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--text);
        font-size: clamp(2rem, 4vw, 3.4rem);
        font-weight: 700;
        line-height: 1.02;
        letter-spacing: -0.05em;
    }

    .search-workspace__input::placeholder {
        color: var(--text-placeholder);
    }

    .search-workspace__results {
        display: grid;
        gap: 0;
    }

    .search-workspace__results--compact {
        margin-top: -18px;
    }

    .search-workspace__label {
        margin: 0;
        padding: 0 0 10px;
        font-size: 0.72rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--text-soft);
    }

    .search-workspace__state {
        margin: 0;
        padding: 4px 0 0;
        color: var(--muted);
        font-size: 0.98rem;
    }

    .search-workspace__state--error {
        color: var(--accent-strong);
    }

    .search-result {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        gap: 12px;
        width: 100%;
        padding: 16px 0;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: var(--text);
        text-align: left;
        cursor: pointer;
        transition: color 120ms ease;
    }

    .search-result:hover,
    .search-result.selected {
        color: var(--text);
    }

    .search-result__content {
        display: grid;
        gap: 4px;
        min-width: 0;
    }

    .search-result__title {
        font-weight: 500;
        font-size: 1.02rem;
    }

    .search-result__meta {
        font-size: 0.82rem;
        color: var(--muted);
    }

    .search-result__folder-path {
        display: inline-flex;
        justify-self: end;
        min-width: 0;
        max-width: min(100%, 22rem);
    }

    @media (max-width: 1180px) {
        .search-workspace {
            padding-left: 0;
            padding-right: 0;
        }

        .search-workspace__inner {
            gap: 18px;
        }

        .search-workspace__input {
            font-size: clamp(1.6rem, 8vw, 2.4rem);
        }

        .search-result {
            grid-template-columns: 1fr;
            justify-items: start;
        }

        .search-result__folder-path {
            justify-self: start;
            max-width: 100%;
        }
    }
</style>
