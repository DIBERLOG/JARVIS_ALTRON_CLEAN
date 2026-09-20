<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"
    type Message = { role: string, content: string }
    type Config = { provider: string, local_model: string, deepseek_model: string, deepseek_configured: boolean, speak_responses: boolean, personality: string }
    let config: Config = { provider: "local", local_model: "qwen3:8b", deepseek_model: "deepseek-flash", deepseek_configured: false, speak_responses: true, personality: "jarvis" }
    let key = "", prompt = "", loading = false, error = "", webSearch = false
    let messages: Message[] = []
    $: visible = messages.filter(m => m.role !== "system")
    onMount(async () => { config = await invoke<Config>("chat_get_config") })
    async function save() {
        await Promise.all([
            invoke("db_write", { key: "chat_provider", val: config.provider }),
            invoke("db_write", { key: "local_chat_model", val: config.local_model }),
            invoke("db_write", { key: "deepseek_chat_model", val: config.deepseek_model }),
            invoke("db_write", { key: "chat_speak_responses", val: String(config.speak_responses) }),
            invoke("db_write", { key: "assistant_personality", val: config.personality }),
            key ? invoke("db_write", { key: "api_key__deepseek", val: key }) : Promise.resolve(true)
        ])
        if (key) { config.deepseek_configured = true; key = "" }
    }
    async function send() {
        const text = prompt.trim(); if (!text || loading) return
        error = ""; prompt = ""; messages = [...messages, { role: "user", content: text }]; loading = true
        try {
            let requestMessages = messages
            if (webSearch) {
                const facts = await invoke<string>("chat_search_web", { query: text })
                requestMessages = [...messages, { role: "system", content: `Свежие данные веб-поиска. Используй только если они отвечают на вопрос, укажи, что это результаты поиска:\n${facts}` }]
            }
            const reply = await invoke<{content:string}>("chat_send", { messages: requestMessages }); messages = [...messages, { role: "assistant", content: reply.content }]
        }
        catch (e) { error = String(e) } finally { loading = false }
    }
</script>

<section class="chat-shell">
    <header><p>НЕЙРОННЫЙ МОДУЛЬ</p><h1>Чат с Jarvis</h1><span>По умолчанию — локальная модель. Облачный DeepSeek включается только с твоим ключом.</span></header>
    <div class="settings" class:altron={config.personality === 'altron'}>
        <div class="persona-switch" aria-label="Личность ассистента"><button class:active={config.personality === 'jarvis'} on:click={() => config.personality = 'jarvis'}><b>JARVIS</b><span>точный, спокойный</span></button><button class:active={config.personality === 'altron'} on:click={() => config.personality = 'altron'}><b>ALTRON</b><span>холодный, прямой</span></button></div>
        <label>Источник <select bind:value={config.provider}><option value="local">Локально — Ollama</option><option value="deepseek">DeepSeek API</option></select></label>
        {#if config.provider === "local"}<label>Модель <input bind:value={config.local_model} /></label><small>На компьютере уже есть <b>qwen3:8b</b> (≈5.2 ГБ), поэтому она выбрана по умолчанию. Ollama работает локально и не требует API-ключа.</small>{:else}<label>Модель DeepSeek <input bind:value={config.deepseek_model} /></label><label>API-ключ <input type="password" bind:value={key} placeholder={config.deepseek_configured ? "Ключ сохранён — введи новый для замены" : "sk-..."} /></label>{/if}
        <label class="voice-toggle"><input type="checkbox" bind:checked={config.speak_responses} /> Озвучивать ответы Jarvis</label><button on:click={save}>Сохранить настройки</button>
    </div>
    <div class="dialog">{#each visible as message}<article class:me={message.role === "user"}><b>{message.role === "user" ? "ТЫ" : "JARVIS"}</b><p>{message.content}</p></article>{/each}{#if loading}<article><b>JARVIS</b><p>Думаю…</p></article>{/if}{#if error}<p class="error">{error}</p>{/if}</div>
    <form on:submit|preventDefault={send}><textarea bind:value={prompt} placeholder="Напиши вопрос ассистенту…" disabled={loading}></textarea><label class="web-search"><input type="checkbox" bind:checked={webSearch} /><span class="pulse"></span><span><b>WEB INTEL</b><small>Искать в интернете перед ответом</small></span></label><button disabled={loading}>Отправить</button></form>
</section>

<style lang="scss">
.chat-shell{max-width:940px;margin:2.5rem auto;color:#eaf8fa;padding:0 1.4rem}.chat-shell header p{color:#52fefe;letter-spacing:.16em;font-size:.75rem}.chat-shell h1{margin:.25rem 0}.chat-shell header span,small{color:#90a7ad}.settings,.dialog,form{background:#0d1417;border:1px solid #1c353a;border-radius:10px;padding:1rem;margin-top:1rem}.settings{display:grid;gap:.7rem}.settings label{display:grid;gap:.3rem;color:#b9d2d8}.persona-switch{display:grid;grid-template-columns:1fr 1fr;gap:.55rem}.persona-switch button{width:100%;text-align:left;background:#102126;border:1px solid #27454c;color:#b8d4d9}.persona-switch button span{display:block;font-size:.72rem;font-weight:400;opacity:.7;margin-top:.2rem}.persona-switch button.active{background:linear-gradient(135deg,#0a6e78,#132c37);border-color:#52fefe;color:#fff}.persona-switch button:last-child.active{background:linear-gradient(135deg,#7b2318,#2a1113);border-color:#ff735a}input,select,textarea{background:#071012;border:1px solid #315057;border-radius:6px;color:#ecffff;padding:.65rem;font:inherit}button{background:#16a8ae;color:#041011;border:0;border-radius:6px;padding:.65rem 1rem;font-weight:700;cursor:pointer;width:max-content}.dialog{min-height:260px;max-height:430px;overflow:auto}article{padding:.65rem .8rem;margin:.6rem 0;background:#101d20;border-left:3px solid #52fefe;border-radius:4px}article.me{border-left-color:#9b63ff}article p{white-space:pre-wrap;margin:.3rem 0 0}form{display:grid;gap:.7rem}textarea{min-height:90px;resize:vertical}.web-search{display:flex!important;align-items:center;gap:.65rem;padding:.65rem .8rem;border:1px solid #27535a;border-radius:7px;background:linear-gradient(90deg,#0b2025,#0d1417);cursor:pointer}.web-search input{accent-color:#52fefe}.web-search b{color:#52fefe;letter-spacing:.1em;font-size:.73rem}.web-search small{display:block}.pulse{height:.5rem;width:.5rem;border-radius:99px;background:#52fefe;box-shadow:0 0 12px #52fefe}.error{color:#ff8e8e}code{color:#52fefe}
</style>
