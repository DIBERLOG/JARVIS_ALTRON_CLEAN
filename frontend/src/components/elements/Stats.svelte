<script lang="ts">
    import { invoke } from "@tauri-apps/api/core"
    import { onMount } from "svelte"
    import { ipcConnected, jarvisRamUsage } from "@/stores"

    type Health = {
        microphone_ready: boolean
        microphone_name: string
        neural_ready: boolean
        tts_mode: string
        tts_ready: boolean
        voicemod_running: boolean
        internet_available: boolean
    }

    let health: Health = {
        microphone_ready: false, microphone_name: "Проверка…",
        neural_ready: false, tts_mode: "—", tts_ready: false,
        voicemod_running: false, internet_available: false
    }
    let checked = false

    async function refresh() {
        try {
            health = await invoke<Health>("get_health_status")
            checked = true
        } catch (error) {
            console.error("Failed to check Jarvis status:", error)
            checked = false
        }
    }

    onMount(() => {
        refresh()
        const timer = window.setInterval(refresh, 10000)
        return () => window.clearInterval(timer)
    })

    function short(value: string): string {
        return value.length > 25 ? value.slice(0, 24) + "…" : value
    }
</script>

<div class="stats-bar" aria-label="Состояние Jarvis">
    <div class="stat-item" title={health.microphone_name}>
        <span class="stat-dot" class:active={checked && health.microphone_ready}></span>
        <div><b>МИКРОФОН</b><small>{short(health.microphone_name)}</small></div>
    </div>
    <div class="stat-item">
        <span class="stat-dot" class:active={checked && health.neural_ready && $ipcConnected}></span>
        <div><b>НЕЙРОСЕТЬ</b><small>{checked && health.neural_ready && $ipcConnected ? "Связь с Jarvis" : "Нет связи"}</small></div>
    </div>
    <div class="stat-item">
        <span class="stat-dot" class:active={checked && health.tts_ready}></span>
        <div><b>ОЗВУЧКА</b><small>{health.tts_mode === "xtts" ? "XTTS" : "Silero"} · {checked && health.tts_ready ? "готова" : "недоступна"}</small></div>
    </div>
    <div class="stat-item">
        <span class="stat-dot" class:active={checked && health.voicemod_running}></span>
        <div><b>VOICEMOD</b><small>{checked && health.voicemod_running ? "Запущен" : "Не запущен"}</small></div>
    </div>
    <div class="stat-item">
        <span class="stat-dot" class:active={checked && health.internet_available}></span>
        <div><b>ИНТЕРНЕТ</b><small>{checked && health.internet_available ? "Доступен" : "Нет доступа"}</small></div>
    </div>
    <div class="stat-item resources">
        <span class="stat-dot" class:active={$jarvisRamUsage > 0}></span>
        <div><b>РЕСУРСЫ</b><small>RAM {$jarvisRamUsage || "—"} МБ</small></div>
    </div>
</div>

<style lang="scss">
    .stats-bar { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); justify-items: center; gap: 1rem 1.6rem; width: min(100%, 900px); margin: 0 auto; padding: 1.25rem 1.5rem; }
    .stat-item { display: flex; width: min(100%, 210px); min-width: 0; align-items: flex-start; gap: .6rem; }
    .stat-item div { display: flex; min-width: 0; flex-direction: column; gap: .18rem; }
    .stat-item b { color: #edfafa; font-size: .73rem; letter-spacing: .05em; }
    .stat-item small { color: rgba(220, 245, 247, .58); font-size: .7rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .stat-dot { flex: none; width: 9px; height: 9px; margin-top: .3rem; border-radius: 50%; background: #4f5b60; }
    .stat-dot.active { background: #2adbb8; box-shadow: 0 0 10px #2adbb8; }
    @media (max-width: 680px) { .stats-bar { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
