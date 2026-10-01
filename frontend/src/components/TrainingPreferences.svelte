<script lang="ts">
    import { dayKey } from "@/lib/center"
    import { trainingId, type TrainingData, type ChargeStep } from "@/lib/training"
    export let data:TrainingData
    export let disabled=false
    export let mode="profile"
    export let onSave:(data:TrainingData)=>Promise<boolean>
    export let defaults:ChargeStep[]=[]
    const levels=["Очень лёгкий","Лёгкий","Умеренно лёгкий","Средний","Выше среднего","Интенсивный","Продвинутый"]
    let height=data.profile?.heightCm??undefined, weight:number|undefined=undefined, fat=data.profile?.bodyFatPercent??undefined
    let method=data.profile?.bodyFatMethod||"Не указан", experience=data.profile?.experience||"Начинающий", limitations=data.profile?.limitations||""
    let level=data.chargeSettings?.level??data.profile?.preferredLevel??1
    let steps:ChargeStep[]=JSON.parse(JSON.stringify(data.chargeSettings?.steps||defaults)).map((item:ChargeStep)=>({...item,enabled:item.enabled!==false}))
    let newName="", newHint="", error="", notice="", busy=false
    $: latest=[...(data.profile?.measurements||[])].sort((a,b)=>a.date.localeCompare(b.date)).filter(item=>item.date<=dayKey(new Date())).at(-1)
    async function save() {
        error="";notice=""
        if(disabled||busy)return
        if(height!==undefined&&(!Number.isFinite(height)||height<50||height>250)){error="Рост: от 50 до 250 см.";return}
        if(weight!==undefined&&(!Number.isFinite(weight)||weight<1||weight>500)){error="Вес: от 1 до 500 кг.";return}
        if(fat!==undefined&&(!Number.isFinite(fat)||fat<=0||fat>=100)){error="Процент жира: больше 0 и меньше 100. Если неизвестен, оставьте пустым.";return}
        if(mode==="charge"&&!steps.some(item=>item.enabled!==false)){error="Оставьте хотя бы одно движение.";return}
        const date=dayKey(new Date())
        const measurements=weight===undefined?data.profile?.measurements||[]:[...(data.profile?.measurements||[]).filter(item=>item.date!==date),{id:trainingId(),date,weight}]
        const next:TrainingData=mode==="profile"?{...data,profile:{...data.profile,heightCm:height??null,measurements,bodyFatPercent:fat??null,bodyFatMethod:method,experience,preferredLevel:level,limitations:limitations.trim()}}:{...data,chargeSettings:{level,steps:steps.map(item=>({...item,name:item.name.trim(),hint:item.hint.trim()}))}}
        if(mode==="charge"&&steps.some(item=>!item.name.trim())){error="Укажите названия движений.";return}
        busy=true
        try{if(await onSave(next)){notice="Настройки сохранены";weight=undefined}else error="Не удалось сохранить. Повторите попытку."}catch{error="Не удалось сохранить. Повторите попытку."}finally{busy=false}
    }
    function preset(value:number){
        level=value
        const seconds=[30,45,50,60,70,80,90][value], reps=[4,6,8,10,12,14,16][value]
        steps=defaults.map(item=>({...item,enabled:true,hint:["walk","arms","finish"].includes(item.id)?`${seconds} секунд · комфортный темп`:`${reps} повторов · комфортная амплитуда`}))
    }
</script>

<section class="preferences" aria-label={mode==="profile"?"Мои параметры":"Настройки зарядки"}>
    <p class="eyebrow">JARVIS / PERSONAL SETTINGS</p><h3>{mode==="profile"?"Мои параметры":"Настрой зарядку под себя"}</h3>
    <form on:submit|preventDefault={save}>
        {#if mode==="profile"}<div class="fields"><label>Рост, см<input type="number" min="50" max="250" step="0.1" bind:value={height}/></label><label>Новое измерение веса, кг<input type="number" min="1" max="500" step="0.1" bind:value={weight} placeholder={latest?`Последнее: ${latest.weight} кг`:"Ещё нет измерений"}/></label><label>Процент жира, %<input type="number" min="0.1" max="99.9" step="0.1" bind:value={fat} placeholder="Необязательно"/></label><label>Способ оценки<select bind:value={method}><option>Не указан</option><option>Биоимпедансные весы</option><option>Калипер / специалист</option><option>DXA</option><option>Другое</option></select></label><label>Опыт занятий<select bind:value={experience}><option>Начинающий</option><option>После перерыва</option><option>Регулярно занимаюсь</option><option>Опытный</option></select></label></div><label>Что учитывать при выборе упражнений<textarea bind:value={limitations} maxlength="1000" placeholder="Например: без прыжков, нужен вариант со стулом"/></label><p class="hint">Это ваша заметка, не медицинская оценка. JARVIS не проверяет противопоказания автоматически.</p>{/if}
        <label>{mode==="profile"?"Предпочитаемый уровень нагрузки":"Уровень зарядки"}<select bind:value={level}>{#each levels as name,index}<option value={index}>{index+1} · {name}</option>{/each}</select></label>
        <p class="hint">Уровень — ваш выбор, а не назначение по весу или проценту жира. Начинайте с комфортной нагрузки; увеличивайте её постепенно.</p>
        {#if mode==="charge"}<p class="hint">Профиль: {data.profile?.heightCm??"—"} см · {latest?.weight??"—"} кг · жир {data.profile?.bodyFatPercent??"—"}% · {data.profile?.experience||"опыт не указан"}. {data.profile?.limitations||""}</p><button type="button" disabled={busy||disabled} on:click={()=>preset(data.profile?.preferredLevel??1)}>Взять предпочитаемый уровень из профиля</button><button type="button" on:click={()=>preset(level)} disabled={busy||disabled}>Применить шаблон уровня</button><small>Заменит редактируемый список базовым комплексом с длительностью и повторами этого уровня. Сохранённые отметки остаются.</small><div class="steps">{#each steps as step,index}<div class="step"><label class="enabled"><input type="checkbox" bind:checked={step.enabled}/>Включить движение</label><label>Название движения<input bind:value={step.name} maxlength="100" required/></label><label>Длительность / повторы<input bind:value={step.hint} maxlength="200"/></label><button type="button" on:click={()=>steps=steps.filter((_,i)=>i!==index)}>Убрать</button></div>{/each}</div><div class="fields"><label>Своё движение<input bind:value={newName} maxlength="100"/></label><label>Цель<input bind:value={newHint} maxlength="200" placeholder="Например: 30 секунд"/></label></div><button type="button" disabled={!newName.trim()} on:click={()=>{steps=[...steps,{id:trainingId(),name:newName.trim(),hint:newHint.trim(),enabled:true}];newName="";newHint=""}}>Добавить движение</button>{/if}
        <button class="primary" disabled={disabled||busy}>{busy?"Сохраняю…":"Сохранить настройки"}</button>{#if error}<p role="alert" class="error">{error}</p>{/if}{#if notice}<p role="status">{notice}</p>{/if}
    </form>
    {#if mode==="profile"}<details><summary>Как определить процент жира?</summary><p>Рост и вес не позволяют точно определить процент жира. ИМТ — не процент жира и не прямое измерение состава тела.</p><p>Биоимпедансные весы и калипер дают оценку; результат зависит от метода и условий измерения. DXA и другие обследования состава тела проводятся специалистами. Сравнивайте измерения одним методом в похожих условиях; если значения нет, поле можно оставить пустым.</p><p><a href="https://www.niddk.nih.gov/health-information/weight-management/adult-overweight-obesity/am-i-healthy-weight" target="_blank" rel="noopener noreferrer">NIDDK: ограничения ИМТ</a> · <a href="https://www.ncbi.nlm.nih.gov/books/NBK603308/" target="_blank" rel="noopener noreferrer">Обзор методов оценки состава тела</a></p></details>{/if}
</section>

<style>
    .preferences{padding:1rem;border:1px solid #28494f;border-radius:12px;background:#0c1b20;color:#eaffff;font-family:"Manrope Variable",sans-serif}.eyebrow{font-size:.6rem;color:#60f3e9;letter-spacing:.12em}h3{font-size:1.15rem}form{display:flex;flex-direction:column;gap:.8rem}.fields{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,220px),1fr));gap:.7rem}label{display:flex;flex-direction:column;gap:.35rem;font-size:.72rem;color:#b3d0d3}input,select,textarea,button{font:600 .75rem "Manrope Variable",sans-serif;padding:.65rem;border:1px solid #28494f;border-radius:8px;background:#10262c;color:#eaffff;box-sizing:border-box;min-width:0}input,select,textarea{width:100%;color-scheme:dark}button{cursor:pointer}.primary{background:#19b7b0;color:#042427;border-color:#60f3e9}button:disabled{opacity:.5;cursor:default}.hint,small,details p{font-size:.68rem;line-height:1.7;color:#91abb1}.steps{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,250px),1fr));gap:.7rem}.step{display:flex;flex-direction:column;gap:.6rem;padding:.8rem;border:1px solid #28494f;border-radius:8px}.enabled{flex-direction:row;align-items:center}.enabled input{width:18px;height:18px;accent-color:#60f3e9}summary,a{color:#60f3e9;font-size:.72rem}details{margin-top:1rem}.error{color:#ffb7b7}button:focus-visible,input:focus-visible,select:focus-visible,textarea:focus-visible,summary:focus-visible{outline:2px solid #60f3e9;outline-offset:2px}
</style>
