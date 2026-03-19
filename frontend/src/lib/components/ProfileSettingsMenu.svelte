<script lang="ts">
    import { Settings2 } from "lucide-svelte";

    let {
        open,
        onToggle,
        username,
        saving,
        errorMessage,
        onUsernameInput,
        onSubmit,
    } = $props<{
        open: boolean;
        onToggle: (isOpen: boolean) => void;
        username: string;
        saving: boolean;
        errorMessage: string;
        onUsernameInput: (value: string) => void;
        onSubmit: () => void;
    }>();
</script>

<details
    class="dropdown-badge dropdown-badge--right dropdown-badge--icon"
    {open}
    ontoggle={(event) =>
        onToggle((event.currentTarget as HTMLDetailsElement).open)}
>
    <summary aria-label="Open settings" title="Settings">
        <Settings2 size={14} strokeWidth={2.2} />
    </summary>
    <div class="dropdown-panel settings-panel">
        <p class="dropdown-label">Profile</p>
        <form
            class="settings-panel__form"
            onsubmit={(event) => {
                event.preventDefault();
                onSubmit();
            }}
        >
            <label class="settings-panel__field">
                <span class="settings-panel__label">Username</span>
                <input
                    value={username}
                    class="settings-panel__input"
                    type="text"
                    maxlength="32"
                    autocomplete="nickname"
                    spellcheck="false"
                    placeholder="Guest"
                    oninput={(event) =>
                        onUsernameInput(
                            (event.currentTarget as HTMLInputElement).value,
                        )}
                />
            </label>
            {#if errorMessage}
                <p class="settings-panel__error">{errorMessage}</p>
            {/if}
            <div class="settings-panel__actions">
                <button
                    type="submit"
                    class="settings-panel__submit"
                    disabled={saving}
                >
                    {saving ? "Saving..." : "Save"}
                </button>
            </div>
        </form>
    </div>
</details>

<style>
    .settings-panel {
        min-width: 260px;
    }

    .settings-panel__form {
        display: grid;
        gap: 10px;
    }

    .settings-panel__field {
        display: grid;
        gap: 6px;
    }

    .settings-panel__label {
        font-size: 0.8rem;
        color: var(--muted);
    }

    .settings-panel__input {
        width: 100%;
        padding: 10px 12px;
        border: 1px solid var(--line);
        border-radius: 12px;
        background: var(--surface-overlay);
        color: var(--text);
        outline: 0;
        transition:
            border-color 120ms ease,
            background 120ms ease;
    }

    .settings-panel__input:focus {
        border-color: var(--surface-overlay-border);
        background: var(--surface-overlay-medium);
    }

    .settings-panel__actions {
        display: flex;
        justify-content: flex-end;
    }

    .settings-panel__submit {
        padding: 9px 12px;
        border: 1px solid var(--line);
        border-radius: 12px;
        background: var(--surface-overlay);
        color: var(--text);
        font-size: 0.82rem;
        font-weight: 500;
        cursor: pointer;
        transition:
            background 120ms ease,
            border-color 120ms ease;
    }

    .settings-panel__submit:hover:not(:disabled),
    .settings-panel__submit:focus-visible {
        background: var(--surface-overlay-medium);
        border-color: var(--surface-overlay-border);
    }

    .settings-panel__submit:disabled {
        cursor: wait;
        opacity: 0.72;
    }

    .settings-panel__error {
        margin: 0;
        font-size: 0.78rem;
        color: var(--accent-strong);
    }

    :global(.settings-panel .dropdown-label) {
        padding: 12px 12px 0;
    }

    @media (max-width: 1180px) {
        .settings-panel {
            min-width: min(260px, calc(100vw - 32px));
        }
    }
</style>
