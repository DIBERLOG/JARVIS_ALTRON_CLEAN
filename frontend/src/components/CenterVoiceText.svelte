<script lang="ts">
    import {onDestroy,onMount} from 'svelte'
    import {makeId,type Note} from '@/lib/center'
    import {invoke} from '@tauri-apps/api/core'
    import {dictationDraft,recordVoice,audioFilePcm,transcribeAudio,pauseCommandListener,MAX_RECORDING_SECONDS,type VoiceRecording} from '@/lib/voiceToText'
    export let disabled=false
    export let onSave:(note:Note)=>Promise<boolean>
    let recording=false,preparing=false,processing=false,saving=false,error='',notice='',level=0,seconds=0,disposed=false
    let devices:MediaDeviceInfo[]=[],deviceId=localStorage.getItem('jarvis-dictation-mic')||''
    let wave:number[]=Array(41).fill(0)
    let autoPunctuation=localStorage.getItem('jarvis-dictation-punctuation')!=='off',formatting=false
    async function refreshDevices(){try{devices=(await navigator.mediaDevices.enumerateDevices()).filter(device=>device.kind==='audioinput')}catch{devices=[]}}
    onMount(()=>{void refreshDevices()})
    let session:VoiceRecording|undefined,restoreListener:(()=>void)|undefined,timer:ReturnType<typeof setInterval>|undefined,noticeTimer:ReturnType<typeof setTimeout>|undefined
    $: busy=recording||preparing||processing||saving
    $: words=$dictationDraft.text.trim()?$dictationDraft.text.trim().split(/\s+/).length:0
    $: clock=`${String(Math.floor(seconds/60)).padStart(2,'0')}:${String(seconds%60).padStart(2,'0')}`
    function release(){clearInterval(timer);timer=undefined;session?.cancel();session=undefined;restoreListener?.();restoreListener=undefined;level=0}
    onDestroy(()=>{disposed=true;release();clearTimeout(noticeTimer)})
    function notify(message:string){if(disposed)return;clearTimeout(noticeTimer);notice=message;noticeTimer=setTimeout(()=>notice='',2000)}
    function message(value:unknown){return value instanceof Error?value.message:typeof value==='string'?value:'Не удалось обработать запись. Попробуйте ещё раз.'}
    async function append(bytes:Uint8Array){
        if(disposed)return
        let text=await transcribeAudio(bytes)
        if(disposed)return
        if(!text.trim()){notify('Речь не распознана. Попробуйте говорить ближе к микрофону.');return}
        if(autoPunctuation){
            formatting=true
            try{text=await invoke<string>('center_punctuate_text',{text})}
            catch(e){if(!disposed)error=`${message(e)} Исходный текст сохранён.`}
            finally{formatting=false}
            if(disposed)return
        }
        dictationDraft.update(draft=>({...draft,text:[draft.text.trim(),text.trim()].filter(Boolean).join('\n\n')}))
        notify('Текст распознан')
    }
    async function start(){
        if(busy)return;preparing=true;error='';notice='';seconds=0
        try{
            restoreListener=await pauseCommandListener()
            if(disposed){release();return}
            wave=Array(41).fill(0)
            session=await recordVoice(value=>{if(!disposed){level=value;wave=[...wave.slice(1),value]}},deviceId)
            void refreshDevices()
            if(disposed){release();return}
            recording=true;const started=Date.now()
            timer=setInterval(()=>{seconds=Math.min(MAX_RECORDING_SECONDS,Math.floor((Date.now()-started)/1000));if(seconds>=MAX_RECORDING_SECONDS)void finish()},250)
        }catch(e){release();if(!disposed)error=message(e)}
        finally{preparing=false}
    }
    async function finish(){
        if(!recording||!session)return
        recording=false;processing=true;clearInterval(timer);error=''
        try{const bytes=await session.stop();release();await append(bytes)}
        catch(e){if(!disposed)error=message(e)}
        finally{release();processing=false}
    }
    async function upload(event:Event){
        const input=event.currentTarget as HTMLInputElement,file=input.files?.[0];input.value=''
        if(!file||busy)return
        processing=true;error='';notice=''
        try{await append(await audioFilePcm(file))}catch(e){if(!disposed)error=message(e)}finally{processing=false}
    }
    async function copy(){try{await navigator.clipboard.writeText($dictationDraft.text);notify('Текст скопирован')}catch{error='Не удалось скопировать. Выделите текст и нажмите Ctrl+C.'}}
    async function formatExisting(){
        if(busy||!$dictationDraft.text.trim())return
        processing=true;formatting=true;error=''
        try{const text=await invoke<string>('center_punctuate_text',{text:$dictationDraft.text});if(!disposed){dictationDraft.update(draft=>({...draft,text}));notify('Знаки препинания добавлены')}}
        catch(e){if(!disposed)error=message(e)}finally{processing=false;formatting=false}
    }
    async function save(){
        if(disabled||busy||!$dictationDraft.text.trim())return
        saving=true;error=''
        try{if(await onSave({id:makeId(),title:$dictationDraft.title.trim()||`Диктовка · ${new Date().toLocaleDateString('ru-RU')}`,text:$dictationDraft.text.trim(),type:'text',format:'plain',updatedAt:new Date().toISOString()}))notify('Заметка сохранена');else error='Не удалось сохранить заметку. Текст остался в редакторе.'}
        catch(e){error=message(e)}finally{saving=false}
    }
</script>

<section class="voice-text" aria-label="Голос в текст">
    <header><div><p class="eyebrow">JARVIS / VOICE TO TEXT</p><h2>Скажи. Сохрани.</h2><p class="subtitle">Мысли, идеи и голосовые записи — теперь в тексте.</p></div><span class="local">● Локально · Vosk</span></header>
    <div class="capture" class:recording>
        <div class="mic" aria-hidden="true">🎙</div>
        <div class="capture-copy"><h3>{recording?'Говорите — идёт запись':processing?'Распознаю речь…':preparing?'Подключаю микрофон…':'Ваш голос — ваши слова'}</h3><p>{recording?'Нажмите «Остановить и распознать», когда закончите.':'Микрофон или аудиофайл. Ничего не отправляется в интернет.'}</p></div>
        <strong class="clock">{clock}</strong>
        <div class="waveform" role="img" aria-label={recording?(level>.01?'Микрофон слышит звук':'Запись: ожидаю голос'):'Волны голоса'} class:active={recording}>
            {#each wave as amplitude}<span style:height={`${4+amplitude*60}px`}></span>{/each}
        </div>
        <label class="device">Микрофон<select aria-label="Микрофон для диктовки" disabled={busy} bind:value={deviceId} on:change={()=>localStorage.setItem('jarvis-dictation-mic',deviceId)}><option value="">Системный по умолчанию</option>{#each devices as device,i}<option value={device.deviceId}>{device.label||`Микрофон ${i+1}`}</option>{/each}</select></label>
        <div class="controls">{#if recording}<button class="primary stop" on:click={finish}>■ Остановить и распознать</button><button on:click={()=>{recording=false;release();notify('Запись отменена')}}>Отменить запись</button>{:else}<button class="primary" disabled={busy} on:click={start}>{preparing?'Подключаю…':processing?'Распознаю…':'● Начать запись'}</button>{/if}<label class="upload" class:disabled={busy}>↑ Загрузить аудио<input type="file" aria-label="Загрузить аудиофайл" accept="audio/*,.wav,.mp3,.m4a,.ogg,.webm" disabled={busy} on:change={upload}/></label></div>
        <p class="hint">До 2 минут · файл до 10 МБ. Волны реагируют на реальный звук. При записи слушание команд временно приостанавливается. Язык определяется моделью Vosk из настроек.</p>
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if notice}<p class="notice" role="status">{notice}</p>{/if}
    <div class="punctuation"><label><input type="checkbox" bind:checked={autoPunctuation} disabled={busy} on:change={()=>localStorage.setItem('jarvis-dictation-punctuation',autoPunctuation?'on':'off')}/> Автопунктуация</label><small>{formatting?'Расставляю знаки препинания…':'Запятые, точки и заглавные буквы · локально через Ollama'}</small><button disabled={busy||!$dictationDraft.text.trim()} on:click={formatExisting}>{formatting?'Обрабатываю…':'Оформить текущий текст'}</button></div>
    <div class="editor"><div class="editor-heading"><h3>Результат</h3><small>{words} слов · можно редактировать</small></div><input aria-label="Название заметки из диктовки" placeholder="Название заметки — необязательно" maxlength="140" bind:value={$dictationDraft.title} disabled={saving}/><textarea aria-label="Распознанный текст" placeholder="Здесь появится распознанная речь. Новые записи добавляются к тексту — ничего не теряется." bind:value={$dictationDraft.text} disabled={processing||saving} spellcheck="true"></textarea><div class="actions"><button class="primary" disabled={disabled||busy||!$dictationDraft.text.trim()} on:click={save}>{saving?'Сохраняю…':'✎ Сохранить в заметки'}</button><button disabled={!$dictationDraft.text.trim()||busy} on:click={copy}>Скопировать текст</button><button class="clear" disabled={!$dictationDraft.text||busy} on:click={()=>{if(confirm('Очистить весь текст диктовки?'))dictationDraft.set({title:'',text:''})}}>Очистить</button></div><p class="hint">Черновик остаётся при переходах между разделами до закрытия приложения. Аудиозапись не сохраняется на диск. Проверьте текст перед сохранением — распознавание может ошибаться.</p></div>
</section>

<style>
    .punctuation{display:flex;align-items:center;flex-wrap:wrap;gap:.8rem;padding:1rem;border:1px solid var(--line);border-radius:12px;margin-bottom:1rem;background:#102b31}.punctuation label{display:flex;align-items:center;gap:.5rem;font-size:.8rem;font-weight:700}.punctuation input{accent-color:var(--accent);width:18px;height:18px}.punctuation small{color:var(--muted);font-size:.65rem}.punctuation button{margin-left:auto}
    .waveform{grid-column:1/-1;height:72px;display:flex;align-items:center;justify-content:center;gap:5px;border-radius:12px;background:#081b20;overflow:hidden}.waveform span{width:5px;border-radius:8px;background:var(--accent);opacity:.25;transition:height .09s linear,opacity .2s}.waveform.active span{opacity:1;box-shadow:0 0 10px #60f3e950}.device{grid-column:1/-1;display:flex;gap:12px;align-items:center;font-size:.7rem;color:var(--muted)}.device select{min-width:0;flex:1;background:#09171b;color:#eaffff;border:1px solid var(--line);border-radius:9px;padding:.7rem;font:inherit}@media(prefers-reduced-motion:reduce){.waveform span{transition:none}}
    .voice-text{--accent:#60f3e9;--muted:#91abb1;--line:#28494f;color:#eaffff;font-family:'Manrope Variable',sans-serif}header{display:flex;justify-content:space-between;align-items:center;gap:1rem;margin-bottom:1.2rem}.eyebrow{color:var(--accent);font-size:.62rem;font-weight:800;letter-spacing:.16em;margin:0}h2{font-size:1.9rem;letter-spacing:-.045em;margin:.4rem 0}.subtitle{color:var(--muted);font-size:.75rem;margin:0}.local{padding:.5rem .7rem;border:1px solid var(--line);border-radius:30px;color:var(--accent);font-size:.62rem;white-space:nowrap}.capture,.editor{padding:1.3rem;border:1px solid var(--line);border-radius:14px;background:linear-gradient(135deg,#173a3d77,#0c1b20);margin-bottom:1rem}.capture{display:grid;grid-template-columns:56px minmax(0,1fr) auto;gap:1rem;align-items:center}.capture.recording{border-color:var(--accent);box-shadow:0 0 24px #60f3e915}.mic{width:56px;height:56px;display:grid;place-items:center;border-radius:16px;background:#164147;border:1px solid #3d7377;font-size:1.6rem}h3{font-size:1rem;margin:0}.capture-copy p{font-size:.7rem;color:var(--muted);margin:.45rem 0 0;line-height:1.7}.clock{font-size:1.4rem;font-variant-numeric:tabular-nums;color:var(--accent)}.meter{grid-column:1/-1;height:5px;border-radius:10px;background:#203c42;overflow:hidden}.meter span{display:block;height:100%;background:var(--accent);transition:width .1s}.controls,.actions{display:flex;flex-wrap:wrap;gap:.6rem}.controls,.capture>.hint{grid-column:1/-1}button,.upload{font:650 .75rem 'Manrope Variable',sans-serif;background:#102b31;color:#d1f1ef;border:1px solid var(--line);border-radius:9px;padding:.8rem 1rem;cursor:pointer}.primary{background:#1aa7a8;border-color:var(--accent);color:#061617;font-weight:800}.stop{background:#163f44;color:var(--accent)}button:hover,.upload:hover{border-color:var(--accent)}button:focus-visible,.upload:focus-within{outline:2px solid var(--accent);outline-offset:3px}button:disabled,.disabled{opacity:.5;cursor:not-allowed}.upload{position:relative;overflow:hidden}.upload input{position:absolute;inset:0;width:100%;opacity:0;cursor:pointer}.hint{color:var(--muted);font-size:.64rem;line-height:1.8;margin:.3rem 0 0}.editor-heading{display:flex;justify-content:space-between;gap:.5rem;margin-bottom:1rem}.editor-heading small{color:var(--muted);font-size:.65rem}.editor>input,textarea{width:100%;box-sizing:border-box;background:#09171b;border:1px solid var(--line);border-radius:9px;color:#eaffff;padding:.8rem;font:500 .78rem 'Manrope Variable',sans-serif;outline:none}.editor>input{margin-bottom:.6rem}textarea{min-height:250px;resize:vertical;line-height:1.85;margin-bottom:.8rem}.editor>input:focus,textarea:focus{border-color:var(--accent)}.clear{margin-left:auto;color:#d5a6a9}.error,.notice{padding:.7rem .9rem;border-radius:9px;font-size:.73rem;line-height:1.7}.error{border:1px solid #805157;background:#351c21;color:#ffc4c4}.notice{border:1px solid #3c8f87;background:#123d3b;color:#b6f7ea}@media(max-width:600px){header{align-items:flex-start;flex-direction:column}.capture{padding:1rem;grid-template-columns:44px minmax(0,1fr)}.mic{width:44px;height:44px}.clock{grid-column:2}.editor-heading{flex-wrap:wrap}.clear{margin-left:0}}@media(prefers-reduced-motion:reduce){.meter span{transition:none}}
</style>
