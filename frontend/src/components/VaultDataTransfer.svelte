<script lang="ts">
    import { invoke } from '@tauri-apps/api/core'
    import { open, save } from '@tauri-apps/plugin-dialog'
    import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
    import { readVault, unlockVault } from '@/lib/passwordVault'
    let busy=false,error='',notice='',candidate='',master='',exportPending=false,exportMaster=''
    const filters=[{name:'Зашифрованное хранилище JARVIS',extensions:['json']}]
    function requestExport(){candidate='';master='';error='';notice='';exportMaster='';exportPending=true}
    async function exportVault(){if(busy||!exportPending||!exportMaster)return;busy=true;error='';notice='';try{
        const raw=await invoke<string|null>('password_vault_load')
        if(!raw)throw Error('Хранилище ещё не создано. Создайте его в разделе «Пароли».')
        await unlockVault(raw,exportMaster);exportMaster=''
        const data=JSON.stringify(readVault(raw)),file=await save({defaultPath:`jarvis-vault-${new Date().toISOString().slice(0,10)}.json`,filters})
        if(!file)return
        await writeTextFile(file,data);exportPending=false;notice='Зашифрованная копия сохранена. Для открытия на другом устройстве нужен прежний мастер-пароль или ключ восстановления.'
    }catch(e){error=e instanceof Error?e.message:String(e)}finally{exportMaster='';busy=false}}
    async function choose(){busy=true;error='';notice='';candidate='';master='';exportPending=false;exportMaster='';try{
        const file=await open({multiple:false,filters});if(typeof file!=='string')return
        candidate=JSON.stringify(readVault(await readTextFile(file)))
    }catch(e){error=e instanceof Error?e.message:String(e)}finally{busy=false}}
    async function restore(){if(!candidate||busy)return;busy=true;error='';try{
        // Authenticate the encrypted file before touching the existing vault. Never export plaintext.
        await unlockVault(candidate,master);master=''
        const existing=await invoke<string|null>('password_vault_load')
        if(existing){const file=await save({title:'Сохраните текущее хранилище перед заменой',defaultPath:`jarvis-vault-before-import-${Date.now()}.json`,filters});if(!file){notice='Импорт отменён: текущее хранилище не изменено.';return}await writeTextFile(file,existing)}
        await invoke('password_vault_save',{data:candidate});candidate=''
        notice='Хранилище перенесено и остаётся заблокированным. Откройте «Пароли» с мастер-паролем импортированного файла. Ключ восстановления этого файла остаётся действующим.'
    }catch(e){error=e instanceof Error?e.message:String(e)}finally{master='';busy=false}}
</script>
<section class="vault-transfer" aria-label="Перенос зашифрованных паролей">
    <h3>◇ Пароли — отдельно и под защитой</h3><p>Карточки, пароли и картинки переносятся без расшифровки в файле. Мастер-пароль и ключ восстановления храните отдельно от копии.</p>
    <div class="buttons"><button disabled={busy} on:click={requestExport}>Экспортировать пароли</button><button disabled={busy} on:click={choose}>Импортировать пароли</button></div>
    {#if exportPending}<form class="confirm" aria-label="Подтверждение экспорта паролей" on:submit|preventDefault={exportVault}><strong>Подтвердите экспорт паролей</strong><p>Введите тот же мастер-пароль, которым открываете текущее хранилище. Файл останется зашифрованным.</p><label>Мастер-пароль текущего хранилища<input type="password" bind:value={exportMaster} autocomplete="off" required disabled={busy}/></label><div class="buttons"><button disabled={busy||!exportMaster}>Проверить и экспортировать</button><button type="button" disabled={busy} on:click={()=>{exportPending=false;exportMaster='';error=''}}>Отмена</button></div></form>{/if}
    {#if candidate}<form class="confirm" aria-label="Подтверждение переноса паролей" on:submit|preventDefault={restore}><strong>Заменить хранилище паролей?</strong><p>Записи заменяются, не объединяются. Перед заменой существующего хранилища потребуется сохранить его резервную копию.</p><label>Мастер-пароль импортированного файла<input type="password" bind:value={master} autocomplete="off" required disabled={busy}/></label><div class="buttons"><button disabled={busy||!master}>Проверить и перенести пароли</button><button type="button" disabled={busy} on:click={()=>{candidate='';master=''}}>Отмена</button></div></form>{/if}
    {#if error}<p role="alert" class="error">{error}</p>{/if}{#if notice}<p role="status">{notice}</p>{/if}
</section>
<style>
    .vault-transfer{margin-top:24px;padding-top:20px;border-top:1px solid #31575b}h3{font-size:15px;color:#8de8d9;margin:0 0 10px}p{font-size:12px;line-height:1.7;color:#b1cccf}.buttons{display:flex;flex-wrap:wrap;gap:10px;margin:14px 0}button{padding:10px 14px;border:1px solid #3a726c;border-radius:10px;background:#183c37;color:#d1ffee;font:700 12px 'Manrope Variable',sans-serif;cursor:pointer}button:hover{background:#235348}button:disabled{opacity:.5;cursor:default}.confirm{padding:16px;border:1px solid #b5a66b;border-radius:12px;background:#39301730}label{display:grid;gap:7px;font-size:12px}input{padding:11px;border:1px solid #4a6d70;border-radius:8px;background:#09191d;color:#eaffff}.error{color:#ffbebe}input:focus-visible,button:focus-visible{outline:2px solid #60f3e9;outline-offset:2px}
</style>
