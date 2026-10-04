<script lang="ts">
    import { onDestroy, tick } from 'svelte'
    import { invoke } from '@tauri-apps/api/core'
    import { ipcConnected, sendTextCommand } from '@/stores'

    let opened=false, log='', error='', command='', sending=false, reading=false
    let follow=true, paused=false, destroyed=false
    let viewport: HTMLPreElement
    let timer: ReturnType<typeof setTimeout> | undefined

    async function refresh() {
        if (!opened || destroyed || reading) return
        reading=true
        try {
            if (!paused) {
                const text=await invoke<string>('get_jarvis_terminal_log')
                if (!opened || destroyed) return
                log=text; error=''
                await tick()
                if (follow && viewport) viewport.scrollTop=viewport.scrollHeight
            }
        } catch (e) { if (!destroyed && opened) error=String(e) }
        finally {
            reading=false
            if (opened && !destroyed) timer=setTimeout(refresh,1000)
        }
    }
    function toggle(show:boolean) {
        opened=show
        clearTimeout(timer)
        if (show) void refresh()
    }
    async function submit() {
        const text=command.trim()
        if (!text || sending) return
        if (!$ipcConnected) {error='Голосовой модуль не подключён. Запустите JARVIS и дождитесь подключения.';return}
        sending=true; error=''
        try {await sendTextCommand(text);command=''} catch(e) {error=String(e)} finally {sending=false}
    }
    onDestroy(()=>{destroyed=true;clearTimeout(timer)})
</script>

<div class="terminal-bar">
    <span>Терминал JARVIS</span>
    <button disabled={!opened} on:click={()=>toggle(false)}>Скрыть терминал</button>
    <button disabled={opened} aria-expanded={opened} aria-controls="jarvis-live-terminal" on:click={()=>toggle(true)}>Открыть терминал</button>
</div>
{#if opened}
    <section id="jarvis-live-terminal" class="live-terminal" aria-label="Терминал JARVIS">
        <header><div><strong>● Живые логи</strong><small>Последние 300 строк · обновление каждую секунду</small></div><span class:online={$ipcConnected}>{$ipcConnected?'Модуль подключён':'Модуль не подключён'}</span></header>
        <div class="options"><label><input type="checkbox" bind:checked={follow}/> Автопрокрутка</label><label><input type="checkbox" bind:checked={paused}/> Пауза просмотра</label></div>
        <pre bind:this={viewport} tabindex="0" aria-label="Логи голосового модуля">{log || 'Лог пока пуст. Новые команды и сообщения появятся здесь.'}</pre>
        <form on:submit|preventDefault={submit}><label for="terminal-command">Команда JARVIS</label><div><input id="terminal-command" bind:value={command} maxlength="200" autocomplete="off" placeholder="Например: какая погода"/><button disabled={sending || !command.trim()}>{sending?'Отправляю…':'Отправить'}</button></div></form>
        {#if error}<p role="alert">{error}</p>{/if}
        <p class="hint">Это просмотр логов, не системная консоль. Скрытие и пауза просмотра не останавливают JARVIS. Запросы чата отправляются во вкладке «Чат».</p>
    </section>
{/if}
<style>
    .terminal-bar{display:flex;align-items:center;gap:.5rem;flex-wrap:wrap;margin:1rem 0}.terminal-bar>span{margin-right:auto;color:#91abb1;font-size:.75rem}
    .live-terminal{--accent:#60f3e9;--edge:#31545a;background:#09191d;border:1px solid var(--edge);border-radius:14px;overflow:hidden;margin-bottom:1.5rem;color:#eaffff;font-family:'Manrope Variable',sans-serif}
    header{display:flex;justify-content:space-between;align-items:center;gap:1rem;flex-wrap:wrap;padding:1rem;border-bottom:1px solid var(--edge);background:linear-gradient(110deg,#15363b,#0a1c21)}header strong{display:block;color:var(--accent);font-size:.85rem}header small{display:block;color:#91abb1;margin-top:.3rem;font-size:.65rem}header>span{font-size:.65rem;color:#d5b48f}header>span.online{color:var(--accent)}
    .options{display:flex;gap:1rem;flex-wrap:wrap;padding:.8rem 1rem;font-size:.7rem;color:#b0d0d4}.options label{display:flex;gap:.4rem;align-items:center}.options input{accent-color:var(--accent)}
    pre{height:300px;resize:vertical;min-height:140px;max-height:650px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;margin:0;padding:1rem;font:500 .72rem/1.8 'Manrope Variable',sans-serif;font-variant-numeric:tabular-nums;background:#061115;color:#bfdedb}
    form{padding:1rem}form>label{display:block;font-size:.7rem;color:#91abb1;margin-bottom:.5rem}form>div{display:flex;gap:.5rem}form input{flex:1;min-width:0}
    button,form input{border:1px solid #31545a;border-radius:8px;background:#102b31;color:#eaffff;padding:.65rem .8rem;font:600 .72rem 'Manrope Variable',sans-serif}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:not(:disabled):hover{border-color:#60f3e9}form button{background:#14787c}
    .hint{margin:0;padding:0 1rem 1rem;color:#91abb1;font-size:.65rem;line-height:1.7}[role="alert"]{color:#ffb3b3;padding:0 1rem;font-size:.75rem}
    button:focus-visible,input:focus-visible,pre:focus-visible{outline:2px solid #60f3e9;outline-offset:2px}@media(max-width:550px){form>div{flex-direction:column}}
</style>
