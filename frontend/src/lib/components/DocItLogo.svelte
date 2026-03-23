<script lang="ts">
    let { className = "" }: { className?: string } = $props();
</script>

<span class={`doc-it-logo ${className}`.trim()} aria-hidden="true">DOC-IT</span>

<style>
    .doc-it-logo {
        display: inline-block;
        font-size: 1.08rem;
        font-weight: 800;
        line-height: 1;
        letter-spacing: 0.18em;
        white-space: nowrap;
        color: currentColor;
    }
</style>
