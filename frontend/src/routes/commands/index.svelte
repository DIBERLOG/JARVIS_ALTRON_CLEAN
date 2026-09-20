<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"

    import HDivider from "@/components/elements/HDivider.svelte"
    import Footer from "@/components/Footer.svelte"
    import { currentLanguage } from "@/stores"

    type JarvisCommand = {
        id: string
        type: string
        description: string
        phrases: Record<string, string[]>
    }

    let commands: JarvisCommand[] = []
    let query = ""
    let loading = true
    let loadError = ""

    const commandPhrases = (command: JarvisCommand) =>
        command.phrases[$currentLanguage]
        || command.phrases.ru
        || command.phrases.en
        || Object.values(command.phrases)[0]
        || []

    $: normalizedQuery = query.trim().toLocaleLowerCase()
    $: filteredCommands = commands.filter(command => {
        if (!normalizedQuery) return true
        return [command.id, command.type, command.description, ...commandPhrases(command)]
            .join(" ")
            .toLocaleLowerCase()
            .includes(normalizedQuery)
    })

    onMount(async () => {
        try {
            commands = await invoke<JarvisCommand[]>("get_commands_list")
        } catch (error) {
            loadError = `Не удалось загрузить команды: ${String(error)}`
        } finally {
            loading = false
        }
    })
</script>

<section class="command-registry" aria-labelledby="commands-title">
    <div class="registry-heading">
        <div>
            <p class="eyebrow">ГОЛОСОВОЙ РЕЕСТР</p>
            <h1 id="commands-title">Команды Jarvis</h1>
            <p class="summary">Произнеси любую из фраз — помощник выполнит соответствующее действие.</p>
        </div>
        <div class="counter" aria-label="Количество доступных команд">
            <strong>{commands.length}</strong>
            <span>доступно</span>
        </div>
    </div>

    <label class="search" for="command-search">
        <span>ПОИСК</span>
        <input id="command-search" bind:value={query} placeholder="Например: браузер, погода, как дела" />
    </label>

    {#if loading}
        <p class="status">Сканирую подключённые пакеты команд…</p>
    {:else if loadError}
        <p class="status error">{loadError}</p>
    {:else if filteredCommands.length === 0}
        <p class="status">По запросу «{query}» ничего не найдено.</p>
    {:else}
        <div class="command-list" aria-live="polite">
            {#each filteredCommands as command, index (command.id)}
                <article class="command-row" style={`--row-index: ${index}`}>
                    <div class="command-number">{String(index + 1).padStart(2, "0")}</div>
                    <div class="command-main">
                        <div class="command-meta">
                            <h2>{command.id.replaceAll("_", " ")}</h2>
                            <span class="command-type">{command.type}</span>
                        </div>
                        {#if command.description}
                            <p class="description">{command.description}</p>
                        {/if}
                        <div class="phrases" aria-label="Фразы для команды">
                            {#each commandPhrases(command) as phrase}
                                <span>«{phrase}»</span>
                            {/each}
                        </div>
                    </div>
                </article>
            {/each}
        </div>
    {/if}
</section>

<HDivider />
<Footer />

<style>
    :global(:root) {
        --registry-cyan: #52fefe;
        --registry-dim: rgba(82, 254, 254, 0.12);
        --registry-line: rgba(255, 255, 255, 0.10);
    }

    .command-registry {
        width: min(940px, 100%);
        margin: 44px auto 28px;
        padding: 0 22px;
        font-family: "Roboto Condensed", sans-serif;
    }

    .registry-heading {
        display: flex;
        justify-content: space-between;
        gap: 28px;
        align-items: end;
        padding: 0 0 18px 18px;
        border-left: 2px solid var(--registry-cyan);
        background: linear-gradient(90deg, var(--registry-dim), transparent 62%);
    }

    .eyebrow {
        color: var(--registry-cyan);
        font-size: 11px;
        font-weight: 700;
        letter-spacing: 0.16em;
        margin-bottom: 5px;
    }

    h1, h2, p { margin: 0; }

    h1 {
        color: #fff;
        font-size: clamp(25px, 4vw, 34px);
        line-height: 1;
        letter-spacing: 0.03em;
        text-transform: uppercase;
    }

    .summary {
        color: #89979a;
        margin-top: 8px;
        font-family: "Roboto", sans-serif;
        font-size: 13px;
    }

    .counter {
        display: grid;
        text-align: right;
        padding-right: 20px;
        color: #8f9da0;
        text-transform: uppercase;
        letter-spacing: 0.09em;
        font-size: 10px;
    }

    .counter strong {
        color: var(--registry-cyan);
        font-size: 30px;
        line-height: 0.9;
        letter-spacing: 0;
    }

    .search {
        display: grid;
        grid-template-columns: 70px 1fr;
        align-items: center;
        gap: 12px;
        margin: 24px 0 12px;
        padding: 9px 14px;
        border: 1px solid var(--registry-line);
        background: rgba(0, 0, 0, 0.18);
    }

    .search span {
        color: #8f9da0;
        font-size: 11px;
        font-weight: 700;
        letter-spacing: 0.1em;
    }

    .search input {
        min-width: 0;
        border: 0;
        outline: 0;
        background: transparent;
        color: #f6ffff;
        font: 15px "Roboto", sans-serif;
    }

    .search input::placeholder { color: #5c696c; }
    .search:focus-within { border-color: rgba(82, 254, 254, 0.55); }

    .command-list { border-top: 1px solid var(--registry-line); }

    .command-row {
        display: grid;
        grid-template-columns: 52px 1fr;
        gap: 14px;
        padding: 17px 14px 17px 0;
        border-bottom: 1px solid var(--registry-line);
        animation: reveal 280ms ease-out both;
        animation-delay: calc(var(--row-index) * 20ms);
    }

    .command-row:hover { background: linear-gradient(90deg, var(--registry-dim), transparent 70%); }

    .command-number {
        color: #526467;
        padding: 2px 0 0 14px;
        font-size: 12px;
        font-weight: 700;
        letter-spacing: 0.08em;
    }

    .command-meta { display: flex; align-items: center; gap: 10px; }

    h2 {
        color: #e8f5f6;
        font-size: 17px;
        font-weight: 700;
        letter-spacing: 0.035em;
        text-transform: uppercase;
    }

    .command-type {
        color: var(--registry-cyan);
        border: 1px solid rgba(82, 254, 254, 0.32);
        padding: 2px 6px;
        font: 700 10px "Roboto", sans-serif;
        letter-spacing: 0.09em;
        text-transform: uppercase;
    }

    .description { color: #9eacae; font: 13px "Roboto", sans-serif; margin-top: 5px; }
    .phrases { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
    .phrases span { color: #b7c4c6; background: rgba(255,255,255,0.045); padding: 4px 7px; font: 13px "Roboto", sans-serif; }
    .status { color: #9eacae; padding: 36px 0; text-align: center; font: 14px "Roboto", sans-serif; }
    .error { color: #ee9a8e; }

    @keyframes reveal { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: translateY(0); } }

    @media (max-width: 560px) {
        .command-registry { margin-top: 28px; padding: 0 14px; }
        .registry-heading { align-items: start; padding-left: 13px; }
        .counter { padding-right: 12px; }
        .summary { max-width: 230px; }
        .search { grid-template-columns: 1fr; gap: 4px; }
        .command-row { grid-template-columns: 35px 1fr; gap: 8px; }
        .command-number { padding-left: 8px; }
    }
</style>
