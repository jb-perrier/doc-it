<script lang="ts">
    import type { Snippet } from "svelte";

    let { leftRail, stage, rightRail } = $props<{
        leftRail?: Snippet;
        stage: Snippet;
        rightRail?: Snippet;
    }>();
</script>

<div class="workspace-page">
    <section class="workspace-layout">
        <aside class="side-rail side-rail--left">
            <div class="side-rail__inner">
                {#if leftRail}
                    {@render leftRail()}
                {/if}
            </div>
        </aside>

        <section class="workspace-pane">
            <div class="workspace-content">
                {@render stage()}
            </div>
        </section>

        <aside class="side-rail side-rail--right">
            <div class="side-rail__inner side-rail__inner--right">
                {#if rightRail}
                    {@render rightRail()}
                {/if}
            </div>
        </aside>
    </section>
</div>

<style>
    .workspace-page {
        padding: 0 0 28px;
        --editor-column-width: 960px;
        --editor-column-min-width: 840px;
        --side-rail-min-width: 0px;
        --side-rail-top-offset: 22px;
        --menu-badge-height: calc(
            1em + (var(--presence-chip-padding-y, 4px) * 2) + 2px
        );
        --menu-badge-radius: 8px;
        --presence-chip-padding-y: 4px;
        --presence-chip-padding-x: 10px;
        --presence-chip-font-size: 0.84rem;
        --presence-chip-gap: 8px;
        --presence-swatch-size: 10px;
    }

    .workspace-layout {
        display: grid;
        grid-template-columns:
            minmax(var(--side-rail-min-width), 1fr)
            minmax(var(--editor-column-min-width), var(--editor-column-width))
            minmax(var(--side-rail-min-width), 1fr);
        gap: clamp(18px, 2.4vw, 36px);
        align-items: start;
        min-height: 100vh;
        padding: 0 clamp(16px, 2vw, 28px);
        box-sizing: border-box;
    }

    .side-rail {
        min-width: 0;
        align-self: stretch;
    }

    .side-rail__inner {
        position: sticky;
        top: var(--side-rail-top-offset);
        display: grid;
        gap: 14px;
        align-content: start;
        padding-top: 0;
    }

    .side-rail__inner--right {
        justify-items: end;
    }

    .workspace-pane {
        width: 100%;
        min-width: 0;
    }

    .workspace-content {
        min-height: 70vh;
    }

    :global(.side-rail__actions) {
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
    }

    :global(.side-rail__actions--right) {
        justify-content: flex-end;
    }

    :global(.topbar-left) {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        align-items: flex-start;
        gap: 10px;
        min-width: 0;
    }

    :global(.topbar-meta) {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        align-items: flex-end;
        justify-content: flex-end;
        gap: 12px;
        min-width: 0;
    }

    :global(.dropdown-badge) {
        position: relative;
    }

    :global(.dropdown-badge summary),
    :global(.menu-badge-button) {
        list-style: none;
        display: inline-flex;
        align-items: center;
        gap: var(--presence-chip-gap, 8px);
        white-space: nowrap;
        block-size: var(--menu-badge-height);
        line-height: 1;
        padding: 0 var(--presence-chip-padding-x, 10px);
        border: 1px solid var(--line);
        border-radius: var(--menu-badge-radius);
        background: var(--surface-overlay);
        color: var(--text);
        font-size: var(--presence-chip-font-size, 0.84rem);
        font-weight: 500;
        cursor: pointer;
        transition:
            background 120ms ease,
            border-color 120ms ease,
            color 120ms ease;
    }

    :global(.menu-badge-button) {
        appearance: none;
        -webkit-appearance: none;
        margin: 0;
        font-family: inherit;
    }

    :global(.menu-badge-button--icon),
    :global(.dropdown-badge--icon summary) {
        justify-content: center;
        inline-size: var(--menu-badge-height);
        padding: 0;
        gap: 0;
    }

    :global(.menu-badge-button:disabled),
    :global(.menu-badge-button--disabled) {
        cursor: default;
        opacity: 0.72;
    }

    :global(.dropdown-badge summary::-webkit-details-marker) {
        display: none;
    }

    :global(.dropdown-badge summary span),
    :global(.dropdown-badge summary svg),
    :global(.menu-badge-button span),
    :global(.menu-badge-button svg) {
        display: block;
        flex-shrink: 0;
    }

    :global(.dropdown-badge summary svg) {
        transform: translateY(1px);
    }

    :global(.dropdown-badge[open] summary),
    :global(.dropdown-badge summary:hover),
    :global(.menu-badge-button:hover) {
        background: var(--surface-overlay-medium);
        border-color: var(--surface-overlay-border);
    }

    :global(.dropdown-panel) {
        position: absolute;
        top: calc(100% + 10px);
        left: 0;
        min-width: 220px;
        padding: 0;
        border: 1px solid var(--line);
        border-radius: var(--menu-badge-radius);
        background: var(--surface-overlay);
        box-shadow: var(--shadow);
    }

    :global(.dropdown-badge--right .dropdown-panel) {
        left: auto;
        right: 0;
    }

    :global(.dropdown-label) {
        margin: 0 0 8px;
        font-size: 0.72rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--text-soft);
    }

    :global(.dropdown-items) {
        display: grid;
        gap: 0;
    }

    :global(.dropdown-item) {
        width: 100%;
        display: flex;
        align-items: center;
        block-size: var(--menu-badge-height);
        padding: 0 10px;
        border: 0;
        border-radius: var(--menu-badge-radius);
        background: transparent;
        color: var(--text);
        font-size: var(--presence-chip-font-size, 0.84rem);
        text-align: left;
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    :global(.dropdown-item:hover),
    :global(.dropdown-item:focus-visible) {
        background: var(--surface-overlay-medium);
        border-radius: 0;
    }

    :global(.dropdown-item--danger) {
        border-top: 1px solid var(--line);
        border-radius: 0;
        color: var(--danger);
    }

    @media (max-width: 1320px) {
        .workspace-page {
            --editor-column-width: 920px;
            --editor-column-min-width: 760px;
        }
    }

    @media (max-width: 1180px) {
        .workspace-page {
            --side-rail-top-offset: 16px;
        }

        .workspace-layout {
            grid-template-columns: minmax(0, 1fr);
            gap: 18px;
            padding-left: 16px;
            padding-right: 16px;
        }

        .side-rail--left {
            order: 1;
        }

        .side-rail--right {
            order: 2;
        }

        .workspace-pane {
            order: 3;
        }

        .side-rail__inner {
            position: static;
            min-height: auto;
            padding-top: 0;
        }

        .side-rail__inner--right {
            justify-items: start;
        }

        :global(.topbar-left) {
            flex-direction: row;
            flex-wrap: wrap;
        }

        :global(.topbar-meta) {
            align-items: flex-start;
        }

        :global(.side-rail__actions--right) {
            justify-content: flex-start;
        }

        :global(.dropdown-panel) {
            left: 0;
            right: auto;
        }

        :global(.dropdown-badge--right .dropdown-panel) {
            left: auto;
            right: 0;
        }
    }

    @media (min-resolution: 1.5dppx) {
        .workspace-page {
            --presence-chip-padding-y: 5px;
            --presence-chip-padding-x: 11px;
            --presence-chip-font-size: 0.87rem;
            --presence-swatch-size: 11px;
        }
    }

    @media (min-resolution: 2dppx) {
        .workspace-page {
            --presence-chip-padding-y: 6px;
            --presence-chip-padding-x: 12px;
            --presence-chip-font-size: 0.9rem;
            --presence-swatch-size: 12px;
        }
    }
</style>
