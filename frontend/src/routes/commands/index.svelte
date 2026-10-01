<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"
    import "@fontsource-variable/manrope/wght.css"

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
    type View = "list" | "tiles" | "categories"
    let view: View = "list"
    const views: { id: View; label: string; icon: string }[] = [
        { id: "list", label: "Список", icon: "≡" },
        { id: "tiles", label: "Плитки", icon: "▦" },
        { id: "categories", label: "Категории", icon: "◫" },
    ]
    const categories = [
        { id: "apps", label: "Приложения", icon: "▦" },
        { id: "web", label: "Интернет и сайты", icon: "↗" },
        { id: "weather", label: "Погода и город", icon: "☁" },
        { id: "counter", label: "Счётчик", icon: "+" },
        { id: "dialogue", label: "Общение с Jarvis", icon: "◌" },
        { id: "system", label: "Система и управление", icon: "⚙" },
        { id: "other", label: "Другие команды", icon: "◇" },
    ]
    let collapsed = new Set<string>()
    function categoryFor(command: JarvisCommand) {
        const id = command.id
        if (/^(weather|set_city)$/.test(id)) return categories[2]
        if (id.startsWith("counter")) return categories[3]
        if (/^(jarvis_restart|computer_restart|repeat_command)$/.test(id)) return categories[5]
        if (/^(jarvis_|dialogue_|test_greet)/.test(id)) return categories[4]
        if (/^(browser_|open_)/.test(id)) return categories[1]
        if (/^(discord_|telegram_|steam_|game_mode|vscode_|calculator_|twitch_)/.test(id)) return categories[0]
        return categories[6]
    }
    function chooseView(next: View) {
        view = next
        try { localStorage.setItem("jarvis-command-view", next) } catch { /* The view still works without storage. */ }
    }
    function toggleCategory(id: string) {
        const next = new Set(collapsed)
        if (next.has(id)) next.delete(id); else next.add(id)
        collapsed = next
    }

    const commandPhrases = (command: JarvisCommand) =>
        command.phrases[$currentLanguage]
        || command.phrases.ru
        || command.phrases.en
        || Object.values(command.phrases)[0]
        || []

    $: normalizedQuery = query.trim().toLocaleLowerCase()
    $: filteredCommands = commands.filter(command => {
        if (!normalizedQuery) return true
        return [command.id, command.type, command.description, categoryFor(command).label, ...commandPhrases(command)]
            .join(" ")
            .toLocaleLowerCase()
            .includes(normalizedQuery)
    })
    $: groups = view === "categories"
        ? categories.map(category => ({ ...category, commands: filteredCommands.filter(command => categoryFor(command).id === category.id) })).filter(group => group.commands.length)
        : [{ id: "all", label: "Все команды", icon: "", commands: filteredCommands }]

    async function refreshCommands() {
        try {
            commands = await invoke<JarvisCommand[]>("get_commands_list")
            loadError = ""
        } catch (error) {
            loadError = `Не удалось загрузить команды: ${String(error)}`
        } finally {
            loading = false
        }
    }

    onMount(() => {
        try {
            const saved = localStorage.getItem("jarvis-command-view")
            if (saved === "list" || saved === "tiles" || saved === "categories") view = saved
        } catch { /* Keep the default list view. */ }
        refreshCommands()
    })
</script>

<svelte:window on:focus={refreshCommands} />

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

    <div class="view-toolbar">
        <div class="view-switch" role="group" aria-label="Вид команд">
            {#each views as option}<button class:active={view === option.id} aria-pressed={view === option.id} on:click={() => chooseView(option.id)}><span aria-hidden="true">{option.icon}</span>{option.label}</button>{/each}
        </div>
        <span class="result-count">{normalizedQuery ? `Найдено: ${filteredCommands.length}` : `${commands.length} команд`}</span>
    </div>

    {#if loading}
        <p class="status">Сканирую подключённые пакеты команд…</p>
    {:else if loadError}
        <p class="status error">{loadError}</p>
    {:else if filteredCommands.length === 0}
        <p class="status">По запросу «{query}» ничего не найдено.</p>
    {:else}
        <div class="command-groups">
        {#each groups as group (group.id)}
            <section class="command-group" class:categorized={view === "categories"} aria-label={group.label}>
                {#if view === "categories"}<button class="category-heading" aria-expanded={normalizedQuery ? true : !collapsed.has(group.id)} on:click={() => toggleCategory(group.id)}><span class="category-icon">{group.icon}</span><h2>{group.label}</h2><span class="category-count">{group.commands.length}</span><span class="category-chevron" aria-hidden="true">{collapsed.has(group.id) && !normalizedQuery ? "+" : "−"}</span></button>{/if}
                {#if view !== "categories" || !collapsed.has(group.id) || normalizedQuery}
        <div class="command-list" class:tiles={view === "tiles"}>
            {#each group.commands as command, index (command.id)}
                <article class="command-row" style={`--row-index: ${index}`}>
                    <div class="command-number" aria-hidden="true">{view === "tiles" ? categoryFor(command).icon : String(index + 1).padStart(2, "0")}</div>
                    <div class="command-main">
                        <div class="command-meta">
                            <h2>{command.id.replaceAll("_", " ")}</h2>
                            <span class="command-type">{categoryFor(command).label}</span>
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

    .command-registry{font-family:"Manrope Variable",sans-serif}.summary,.search input,.description,.phrases span,.status,.command-type{font-family:"Manrope Variable",sans-serif}
    .view-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px;margin:18px 0}.view-switch{display:flex;gap:4px;padding:4px;border:1px solid #28464d;border-radius:10px;background:#0e1c22}.view-switch button{display:flex;align-items:center;gap:7px;padding:9px 13px;border:1px solid transparent;border-radius:7px;background:transparent;color:#92adb4;font:700 12px "Manrope Variable",sans-serif;cursor:pointer;transition:background .18s,color .18s,border-color .18s}.view-switch button span{font-size:19px;line-height:1}.view-switch button:hover{background:#1b343b;color:#fff}.view-switch button.active{background:linear-gradient(120deg,#16565b,#17363f);border-color:#58d5d0;color:#dcfffb}.view-switch button:focus-visible,.category-heading:focus-visible{outline:2px solid #a1fff4;outline-offset:3px}.result-count{color:#809da6;font-size:11px;white-space:nowrap}.command-groups{display:grid;gap:16px}.command-row{animation-delay:calc(min(var(--row-index),8) * 20ms)}.command-meta{flex-wrap:wrap}.command-main{min-width:0}.command-main h2{overflow-wrap:anywhere}.phrases span{overflow-wrap:anywhere;border-radius:5px}.command-type{font-size:9px;text-transform:none;letter-spacing:0;border-radius:4px;color:#9ccfd2;border-color:#31565c}.command-list.tiles{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;border-top:0}.tiles .command-row{display:flex;flex-direction:column;gap:12px;padding:18px;border:1px solid #28484f;border-radius:12px;background:radial-gradient(ellipse at 0 0,#1b565b35,transparent 65%),#0e1c22;box-shadow:0 8px 24px #0002;transition:border-color .18s,background .18s}.tiles .command-row:hover{border-color:#57b5b3;background:#133039}.tiles .command-number{display:grid;place-items:center;width:36px;height:36px;padding:0;border:1px solid #35626a;border-radius:10px;background:#18414a;color:#70e9df;font-size:22px}.tiles .command-meta{align-items:flex-start;flex-direction:column;gap:7px}.tiles .command-main h2{font-size:15px}.tiles .description{line-height:1.55}.tiles .phrases span{font-size:11px;line-height:1.5}.categorized{overflow:hidden;border:1px solid #28484f;border-radius:12px;background:#0e1c2280}.category-heading{display:flex;align-items:center;gap:11px;width:100%;padding:15px 17px;border:0;background:linear-gradient(110deg,#1b424c,#0e222a);color:#daf6f7;text-align:left;cursor:pointer}.category-heading h2{font-size:14px;text-transform:none;letter-spacing:0}.category-icon{display:grid;place-items:center;width:29px;height:29px;border:1px solid #39656b;border-radius:8px;background:#19434b;color:#7df1e5;font-size:18px}.category-count{margin-left:auto;padding:3px 8px;border:1px solid #37636b;border-radius:12px;color:#8ae5df;font-size:11px}.category-chevron{width:14px;text-align:center;color:#7cbfc4;font-size:20px}.categorized .command-list{padding:0 14px;border-top:1px solid #28484f}.categorized .command-row:last-child{border-bottom:0}
    @media(max-width:560px){.view-toolbar{align-items:flex-start;flex-direction:column}.view-switch{width:100%}.view-switch button{flex:1;justify-content:center;padding:9px 6px;font-size:11px}.command-list.tiles{grid-template-columns:1fr}.result-count{align-self:flex-end}}
    @media(prefers-reduced-motion:reduce){.command-row{animation:none}.view-switch button,.tiles .command-row{transition:none}}
</style>
