<script lang="ts">
	import type { PeerPresence } from '$lib/types';

	let { peers } = $props<{ peers: PeerPresence[] }>();
</script>

<div class="presence-bar">
	{#if peers.length > 0}
		{#each peers as peer (peer.clientId)}
			<div class="chip">
				<span class="swatch" style={`background:${peer.color}`}></span>
				<span>{peer.name}</span>
			</div>
		{/each}
	{/if}
</div>

<style>
	.presence-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: nowrap;
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--presence-chip-gap, 8px);
		white-space: nowrap;
		line-height: 1;
		padding: var(--presence-chip-padding-y, 4px) var(--presence-chip-padding-x, 10px);
		border-radius: 8px;
		background: var(--surface-overlay);
		border: 1px solid var(--line);
		font-size: var(--presence-chip-font-size, 0.84rem);
		font-weight: 500;
		color: var(--text);
	}

	.swatch {
		width: var(--presence-swatch-size, 10px);
		height: var(--presence-swatch-size, 10px);
		border-radius: 999px;
		transform: translateY(0.5px);
	}
</style>