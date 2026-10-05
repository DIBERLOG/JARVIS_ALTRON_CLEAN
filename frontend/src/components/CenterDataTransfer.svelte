<script lang="ts">
    import { open, save } from '@tauri-apps/plugin-dialog'
    import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
    import { createCenterBackup, parseCenterBackup, restoreCenterBackup, type CenterBackup } from '@/lib/centerBackup'
    import { centerRevision, resetCenterVoice } from '@/lib/centerVoice'
    import VaultDataTransfer from './VaultDataTransfer.svelte'
    let busy=false, error='',notice='',candidate:CenterBackup|null=null,source=''
    const filters=[{name:'Данные JARVIS',extensions:['json']}]
    async function exportData() {
        busy=true;error='';notice=''
        try {
            const file=await save({defaultPath:`jarvis-data-${new Date().toISOString().slice(0,10)}.json`,filters})
            if(!file)return
            await writeTextFile(file,JSON.stringify(await createCenterBackup(),null,2))
            notice='Данные экспортированы. Скопируйте этот файл на другой компьютер.'
        } catch(e){error=String(e)}finally{busy=false}
    }
    async function chooseImport() {
        busy=true;error='';notice='';candidate=null
        try {
            const file=await open({multiple:false,filters})
            if(typeof file!=='string')return
            candidate=parseCenterBackup(await readTextFile(file));source=file
        }catch(e){error=String(e)}finally{busy=false}
    }
    async function confirmImport() {
        if(!candidate||busy)return
        busy=true;error=''
        try {
            const rollback=await save({title:'Сохраните резервную копию текущих данных перед импортом',defaultPath:`jarvis-before-import-${Date.now()}.json`,filters})
            if(!rollback){notice='Импорт отменён: резервная копия не сохранена.';return}
            await writeTextFile(rollback,JSON.stringify(await createCenterBackup(),null,2))
            await restoreCenterBackup(candidate)
            resetCenterVoice();centerRevision.update(n=>n+1);candidate=null
            notice='Данные восстановлены. Для обновления города в открытом прогнозе переоткройте раздел погоды.'
        }catch(e){error=String(e)}finally{busy=false}
    }
</script>

<section class="transfer" aria-label="Перенос личных данных">
    <p class="eyebrow">РЕЗЕРВНАЯ КОПИЯ</p><h2>Ваши данные — с вами</h2>
    <p>Заметки, календарь и напоминания, дни рождения, город, привычки и тренировки — в одном файле для переноса на другой компьютер.</p>
    <div class="actions"><button disabled={busy} on:click={exportData}>↓ Экспортировать данные</button><button disabled={busy} on:click={chooseImport}>↑ Импортировать файл</button></div>
    <small>Основной файл не зашифрован: храните его безопасно. Пароли переносите отдельным зашифрованным файлом ниже. Почта, токены и ключи API не включаются. Секреты, записанные в заметках вручную, останутся в заметках.</small>
    {#if candidate}<div class="confirm" role="region" aria-label="Подтверждение импорта"><h3>Заменить текущие личные данные?</h3><p>{source}</p><p>{candidate.data.notes.length} заметок · {candidate.data.reminders.length} напоминаний · {candidate.data.birthdays.length} дней рождения · город: {candidate.city}</p><p>Импорт заменяет данные, а не объединяет их. Сначала потребуется сохранить резервную копию. Голос и остальные настройки останутся прежними.</p><div class="actions"><button disabled={busy} on:click={confirmImport}>Сохранить копию и заменить</button><button disabled={busy} on:click={()=>candidate=null}>Отмена</button></div></div>{/if}
    {#if error}<p role="alert" class="error">{error}</p>{/if}
    {#if notice}<p role="status">{notice}</p>{/if}
    <VaultDataTransfer />
</section>

<style>
    .transfer{margin:24px 0;padding:24px;border:1px solid #31575b;border-radius:16px;background:linear-gradient(130deg,#143236,#0d1c22);color:#e5f8f5;font-family:'Manrope Variable',sans-serif}.eyebrow{color:#65e5d8;font-size:10px;letter-spacing:.16em;font-weight:800}.transfer h2{font-size:23px;margin:8px 0 12px;text-transform:none}.transfer p{font-size:13px;line-height:1.7;overflow-wrap:anywhere}.transfer small{display:block;color:#94b1b4;line-height:1.7}.actions{display:flex;flex-wrap:wrap;gap:10px;margin:16px 0}.actions button{padding:11px 15px;border:1px solid #4da99e;border-radius:10px;background:#174940;color:#caffef;font:700 12px 'Manrope Variable',sans-serif;cursor:pointer}.actions button:hover{background:#236356}.actions button:disabled{opacity:.5;cursor:wait}.confirm{padding:16px;border:1px solid #beac68;border-radius:12px;margin-top:16px;background:#332e1730}.error{color:#ffbbbb}button:focus-visible{outline:2px solid #77efdf;outline-offset:3px}
</style>
