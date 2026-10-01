<script lang="ts">
    import type { Exercise } from "@/lib/training"
    export let selected = ""
    export let counts: { group:string; sets:number }[] = []
    export let exercises: Exercise[] = []
    export let metric = "Рабочие подходы"
    export let caption = "Записанные подходы по основной мышечной группе. Не измерение восстановления."
    export let onSelect: (group:string)=>void = ()=>{}
    export let onExercise: ((exercise:Exercise)=>void) | undefined = undefined
    let view: "front" | "back" | "both" = "both"
    let collapsed = false
    const markers = [
        {group:"Грудь",x:27,y:22,side:"front",about:"Основная группа для жимов и сведений рук."},
        {group:"Спина",x:73,y:29,side:"back",about:"Основная группа для вертикальных и горизонтальных тяг."},
        {group:"Плечи",x:13,y:20,side:"front",about:"Основная группа для жимов над головой и упражнений на дельты."},
        {group:"Бицепс",x:12,y:29,side:"front",about:"Основная группа для сгибаний рук."},
        {group:"Трицепс",x:88,y:29,side:"back",about:"Основная группа для разгибаний рук."},
        {group:"Пресс",x:27,y:33,side:"front",about:"Основная группа для упражнений на мышцы живота."},
        {group:"Ноги",x:20,y:57,side:"front",about:"Объединяет упражнения на бёдра, ягодицы и икры. Упражнения выбираются ниже."},
    ]
    $: groups = [...markers.map(marker=>marker.group),...new Set([...counts.map(item=>item.group),...exercises.map(exercise=>exercise.muscle)].filter(group=>!markers.some(marker=>marker.group===group)))]
    $: chosen = markers.find(marker=>marker.group===selected)
    $: matching = exercises.filter(exercise=>exercise.muscle===selected)
    $: maximum = Math.max(1,...counts.map(item=>item.sets))
    function count(group:string) {return counts.find(item=>item.group===group)?.sets || 0}
    function choose(group:string) {selected=selected===group?"":group;onSelect(selected)}
    function left(x:number) {return view==="both"?x:view==="front"?x*2:(x-50)*2}
</script>

<section class="body-map" aria-label="Интерактивная карта мышечных групп">
    <button class="map-toggle" aria-expanded={!collapsed} on:click={()=>collapsed=!collapsed}>{collapsed ? "▸ Показать карту тела" : "▾ Скрыть карту тела"}</button>
    {#if !collapsed}
    <div class="heading"><div><p>КАРТА ТЕЛА</p><h4>Выбери мышечную группу</h4></div>{#if selected}<button class="clear" on:click={()=>{selected="";onSelect("")}}>Сбросить выбор</button>{/if}</div>
    <div class="views" aria-label="Вид тела"><button class:active={view==="front"} aria-pressed={view==="front"} on:click={()=>view="front"}>Спереди</button><button class:active={view==="back"} aria-pressed={view==="back"} on:click={()=>view="back"}>Сзади</button><button class:active={view==="both"} aria-pressed={view==="both"} on:click={()=>view="both"}>Общее</button></div>
    <div class="map-layout"><div class="visual"><div class="canvas" class:single={view!=="both"}><img src="/images/training-body-hud-v1.png" alt="Условная фигура человека спереди и сзади в стиле JARVIS" style={`width:${view==="both"?100:200}%;left:${view==="back"?-100:0}%`}/>{#each markers as marker,index}{#if view==="both"||marker.side===view}<button class="marker" class:recorded={count(marker.group)>0} class:selected={selected===marker.group} style={`left:${left(marker.x)}%;top:${marker.y}%`} aria-label={`Выбрать группу ${marker.group}, ${count(marker.group)} подходов`} aria-pressed={selected===marker.group} title={`${index+1}. ${marker.group} · ${metric}: ${count(marker.group)}`} on:click={()=>choose(marker.group)}>{index+1}</button>{/if}{/each}</div><small class="hint">Цифры 1–7 — номера групп, не количество подходов.</small></div>
    <div class="legend" aria-label="Мышечные группы">{#each groups as group,index}<button class:selected={selected===group} aria-pressed={selected===group} aria-label={`Группа ${group}`} on:click={()=>choose(group)}><span>{index<7?index+1:"•"}</span><strong>{group}</strong><small>{count(group)} подх.</small></button>{/each}</div></div>
    {#if selected}<div class="detail" aria-live="polite"><div class="detail-title"><h4>{selected}</h4><strong>{count(selected)}<small>подходов</small></strong></div><p>{chosen?.about||"Ваша собственная мышечная группа. На условной фигуре её область не размечена."}</p><small>{metric}. {caption}</small><h5>Упражнения этой группы</h5>{#if !matching.length}<p>В текущем списке пока нет упражнений этой группы.</p>{:else}<div class="exercise-links">{#each matching as exercise}{#if onExercise}<button on:click={()=>onExercise?.(exercise)}><strong>{exercise.name}</strong><small>{exercise.equipment} →</small></button>{:else}<div><strong>{exercise.name}</strong><small>{exercise.equipment}</small></div>{/if}{/each}</div>{/if}</div>{:else}<p class="hint">Нажми номер на фигуре или название в списке. Откроются пояснение, подходы и упражнения.</p>{/if}
    <details><summary>Как читать карту?</summary><p>Бирюзовый маркер означает, что для группы есть подходы в текущем контексте. Число справа в списке — количество подходов, а цифра на теле — только номер группы. Рамка отмечает ваш выбор. Передняя и задняя фигуры показывают условное расположение групп; это не персональная анатомическая модель и не карта боли или восстановления.</p><p>{metric}: {caption}</p></details>
    {/if}
</section>

<style>
    .map-toggle{display:block;width:100%;text-align:left;margin-bottom:.7rem;padding:.65rem .8rem;color:#60f3e9;font-size:.72rem}.map-toggle[aria-expanded="false"]{margin-bottom:0}
    .visual{display:flex;flex-direction:column;align-items:center;width:100%}.visual .hint{text-align:center;max-width:230px}.map-layout{justify-items:center}.legend{width:100%}
    .body-map{--accent:#60f3e9;--line:#28494f;color:#eaffff;font-family:"Manrope Variable",sans-serif}.heading{display:flex;justify-content:space-between;gap:.5rem;align-items:start;margin-bottom:.7rem}.heading p{margin:0;color:var(--accent);font-size:.57rem;letter-spacing:.14em;font-weight:800}h4{font-size:.85rem;margin:.4rem 0}button{font:600 .64rem "Manrope Variable",sans-serif;border:1px solid var(--line);border-radius:7px;background:#102b31;color:#cceff0;padding:.5rem;cursor:pointer}.clear{font-size:.55rem;flex:none}.views{display:flex;gap:.3rem;margin-bottom:.8rem}.views button{flex:1;font-size:.62rem}.active{background:#17545a;border-color:var(--accent);color:white}.map-layout{display:grid;grid-template-columns:minmax(100px,1fr) minmax(110px,.8fr);gap:.7rem;align-items:center}.visual{min-width:0}.canvas{position:relative;overflow:hidden;aspect-ratio:2/3;width:100%;max-width:230px;margin:auto;border-radius:10px;background:#071a22}.canvas.single{aspect-ratio:1/3;max-width:115px}.canvas img{position:absolute;top:0;height:auto;max-width:none}.marker{position:absolute;transform:translate(-50%,-50%);display:grid;place-items:center;width:26px;height:26px;border-radius:50%;padding:0;background:#09252ee6;border-color:#47878e;color:#b0d2d7;font-size:.66rem;box-shadow:0 0 0 4px #09252e44}.marker.recorded{background:#124c51;border-color:#60f3e9;color:#bffff5;box-shadow:0 0 14px #60f3e944}.marker.selected{background:#60f3e9;border-color:#fff;color:#031e23;box-shadow:0 0 0 4px #60f3e942,0 0 20px #60f3e950}.legend{display:flex;flex-direction:column;gap:.35rem}.legend button{display:flex;align-items:center;gap:.4rem;padding:.55rem .4rem;text-align:left}.legend button>span{display:grid;place-items:center;border:1px solid #356a70;border-radius:50%;width:20px;height:20px;flex:none;color:var(--accent);font-size:.58rem}.legend strong{font-size:.64rem;flex:1}.legend small{font-size:.55rem;color:#a4c1c5}.legend .selected{border-color:var(--accent);background:#174b50}.hint{display:block;color:#91abb1;font-size:.58rem;line-height:1.6;margin:.7rem 0}.detail{border-top:1px solid var(--line);margin-top:.8rem;padding-top:.6rem}.detail-title{display:flex;justify-content:space-between;align-items:center}.detail-title>strong{color:var(--accent);font-size:1.1rem}.detail-title small{font-size:.55rem;margin-left:.35rem;font-weight:500}.detail p,details p{font-size:.62rem;line-height:1.65;color:#afcdd0}.detail>small{font-size:.56rem;color:#91abb1;line-height:1.6}h5{font-size:.63rem;margin:.8rem 0 .4rem}.exercise-links{display:flex;flex-direction:column;gap:.35rem}.exercise-links button,.exercise-links>div{display:flex;flex-direction:column;gap:.2rem;padding:.55rem;border:1px solid var(--line);border-radius:6px;background:#102b31;text-align:left}.exercise-links strong{font-size:.64rem}.exercise-links small{font-size:.56rem;color:#91abb1}details{margin-top:.8rem}summary{font-size:.59rem;color:var(--accent);cursor:pointer}button:hover{border-color:var(--accent)}button:focus-visible,summary:focus-visible{outline:2px solid var(--accent);outline-offset:2px}@media(max-width:420px){.map-layout{grid-template-columns:1fr}.canvas{max-width:200px}.canvas.single{max-width:100px}.legend{display:grid;grid-template-columns:repeat(2,minmax(0,1fr))}.legend button{min-width:0}.legend small{font-size:.5rem}}
    .canvas{max-width:360px}.canvas.single{width:50%;max-width:180px}.visual .hint{max-width:360px}.legend button{min-height:42px;padding:.7rem .6rem}.exercise-links{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,230px),1fr))}
    @media(max-width:600px){.canvas{max-width:230px}.canvas.single{max-width:115px}.legend button{min-height:36px;padding:.55rem .4rem}}
    @media(max-width:420px){.canvas{max-width:200px}.canvas.single{max-width:100px}}
</style>
