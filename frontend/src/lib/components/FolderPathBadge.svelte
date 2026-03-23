<script lang="ts">
    import type { FolderPathSegment } from "$lib/folders/path";

    type DisplaySegment = {
        id: string;
        name: string;
        ellipsis?: boolean;
    };

    let {
        segments,
        maxVisible = 3,
        size = "xs",
        label,
    } = $props<{
        segments: FolderPathSegment[];
        maxVisible?: number;
        size?: "xs" | "sm";
        label?: string;
    }>();

    const fullPath = $derived(
        segments.map((segment: FolderPathSegment) => segment.name).join(" / "),
    );

    const displaySegments = $derived.by(() => {
        if (segments.length <= maxVisible) {
            return segments;
        }

        const trailingCount = Math.max(1, maxVisible - 2);
        const trailingSegments = segments.slice(-trailingCount);

        return [
            segments[0],
            { id: "folder-path-ellipsis", name: "...", ellipsis: true },
            ...trailingSegments,
        ] satisfies DisplaySegment[];
    });
</script>

{#if segments.length > 0}
    <span
        class:folder-path-badge--sm={size === "sm"}
        class="folder-path-badge"
        title={label ?? fullPath}
        aria-label={`Folder path: ${label ?? fullPath}`}
    >
        <span class="folder-path-badge__path">
            {#each displaySegments as segment, index (segment.id)}
                {#if index > 0}
                    <span class="folder-path-badge__divider">/</span>
                {/if}
                <span
                    class:folder-path-badge__segment--ellipsis={segment.ellipsis}
                    class="folder-path-badge__segment"
                >
                    {segment.name}
                </span>
            {/each}
        </span>
    </span>
{/if}

<style>
    .folder-path-badge {
        display: inline-flex;
        align-items: center;
        min-width: 0;
        max-width: min(100%, 24rem);
        padding: 6px 0 6px 0;
        border-radius: var(--menu-badge-radius, 8px);
        background: color-mix(
            in srgb,
            var(--surface-elevated) 72%,
            transparent
        );
        color: var(--muted);
        font-size: 0.7rem;
        line-height: 1;
        letter-spacing: 0.02em;
        white-space: nowrap;
    }

    .folder-path-badge--sm {
        padding: 7px 0 7px 0;
        font-size: 0.76rem;
    }

    .folder-path-badge__path {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        min-width: 0;
        overflow: hidden;
    }

    .folder-path-badge__segment {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .folder-path-badge__segment--ellipsis,
    .folder-path-badge__divider {
        color: var(--text-soft);
    }
</style>
