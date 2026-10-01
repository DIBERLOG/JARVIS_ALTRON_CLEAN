<script lang="ts">
    import { onDestroy } from "svelte"
    import { listeningShortcut, shortcutError, recordingShortcut, applyListeningShortcut } from "@/lib/listeningShortcut"
    let busy = false
    let hint = ""
    onDestroy(() => recordingShortcut.set(false))
    async function save(shortcut: string) {
        busy = true
        recordingShortcut.set(false)
        try { await applyListeningShortcut(shortcut); hint = "Сохранено" } catch { hint = "" }
        finally { busy = false }
    }
    function capture(event: KeyboardEvent) {
        if (!$recordingShortcut) return
        event.preventDefault()
        event.stopPropagation()
        if (event.code === "Escape") { recordingShortcut.set(false); return }
        if (event.repeat || /^(Control|Shift|Alt|Meta)/.test(event.code)) return
        if (!(event.ctrlKey || event.altKey || event.metaKey)) { hint = "Добавьте Ctrl или Alt к клавише"; return }
        const key = /^Key[A-Z]$/.test(event.code) ? event.code.slice(3) : /^Digit[0-9]$/.test(event.code) ? event.code.slice(5) : event.code
        if (!/^[A-Z0-9]$|^F\d{1,2}$|^Space$|^Arrow(Up|Down|Left|Right)$/.test(key)) { hint = "Используйте букву, цифру, F-клавишу, пробел или стрелку"; return }
        const parts = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super", key].filter(Boolean)
        void save(parts.join("+"))
    }
</script>

<svelte:window on:keydown={capture} />
<section class="shortcut-card" aria-label="Горячая клавиша прослушивания">
    <div><p class="eyebrow">БЫСТРОЕ УПРАВЛЕНИЕ</p><h3>Слушать / не слушать</h3><p>Одно сочетание включает или выключает микрофон, даже поверх других приложений. Работает, пока окно JARVIS открыто или свёрнуто.</p></div>
    <div class="controls">
        <button class="capture" disabled={busy} on:click={() => { hint = ""; recordingShortcut.set(!$recordingShortcut) }} aria-pressed={$recordingShortcut}>{$recordingShortcut ? "Нажмите сочетание…" : $listeningShortcut || "Не назначено"}</button>
        <button disabled={busy || !$listeningShortcut} on:click={() => save("")}>Отключить</button>
    </div>
    <small aria-live="polite">{$recordingShortcut ? "Ctrl / Alt + клавиша. Esc — отмена." : hint || "Нажмите на сочетание, чтобы изменить. Сохраняется автоматически."}</small>
    {#if $shortcutError}<p class="error" role="alert">{$shortcutError}</p>{/if}
</section>

<style lang="scss">
    .shortcut-card {--accent:#52fefe;margin-top:.7rem;padding:1rem;border:1px solid #29555a;border-radius:9px;background:linear-gradient(110deg,#10292c,#0c1519);font-family:"Manrope Variable",sans-serif;color:#e7fafa}
    .eyebrow {margin:0;color:var(--accent);font-size:.6rem;font-weight:800;letter-spacing:.14em}
    h3 {margin:.35rem 0;font-size:1rem}p:not(.eyebrow) {color:#a8bdc2;font-size:.75rem;line-height:1.5;margin:.3rem 0}
    .controls {display:flex;flex-wrap:wrap;gap:.5rem;margin:.8rem 0}
    button {padding:.55rem .8rem;border:1px solid #35595c;border-radius:7px;background:#102d31;color:#ccece9;font:600 .75rem "Manrope Variable",sans-serif;cursor:pointer}
    .capture {border-color:var(--accent);color:var(--accent);min-width:160px}.capture[aria-pressed="true"] {background:#165156}
    button:disabled {opacity:.5;cursor:default}button:focus-visible {outline:2px solid var(--accent);outline-offset:3px}
    small {color:#91afaf;font-size:.65rem}.error {color:#ffaaaa!important}
</style>
