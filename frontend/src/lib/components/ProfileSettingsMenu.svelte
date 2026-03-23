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
        <form
            class="settings-panel__form"
            onsubmit={(event) => {
                event.preventDefault();
                onSubmit();
            }}
        >
            <div class="settings-panel__header">
                <p class="settings-panel__title">Settings</p>
                <p class="settings-panel__caption">Profile name</p>
            </div>
            <label class="settings-panel__field">
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
            <div class="settings-panel__actions">
                {#if errorMessage}
                    <p class="settings-panel__error">{errorMessage}</p>
                {/if}
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
        padding: 10px;
    }

    .settings-panel__form {
        display: grid;
        gap: 8px;
    }

    .settings-panel__header {
        display: grid;
        gap: 2px;
    }

    .settings-panel__title,
    .settings-panel__caption {
        margin: 0;
    }

    .settings-panel__title {
        font-size: 0.84rem;
        font-weight: 600;
        color: var(--text);
    }

    .settings-panel__caption {
        font-size: 0.74rem;
        color: var(--text-soft);
    }

    .settings-panel__field {
        min-width: 0;
    }

    .settings-panel__input {
        width: 100%;
        padding: 8px 0;
        border: 0;
        border-bottom: 1px solid
            color-mix(in srgb, var(--line) 82%, transparent);
        border-radius: 0;
        background: transparent;
        color: var(--text);
        outline: 0;
        font-size: 0.84rem;
        transition:
            border-color 120ms ease,
            background 120ms ease;
    }

    .settings-panel__input:focus {
        border-color: var(--surface-overlay-border);
        background: transparent;
    }

    .settings-panel__actions {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        min-height: 28px;
    }

    .settings-panel__submit {
        padding: 7px 10px;
        border: 1px solid var(--line);
        border-radius: 9px;
        background: var(--surface-overlay);
        color: var(--text);
        font-size: 0.8rem;
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
        font-size: 0.74rem;
        color: var(--accent-strong);
    }

    @media (max-width: 1180px) {
        .settings-panel {
            min-width: min(260px, calc(100vw - 32px));
        }
    }
</style>
