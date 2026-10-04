<script lang="ts">
    import { onMount } from "svelte"
    import { dayKey, makeId, type Habit } from "@/lib/center"
    import { activeOn, habitValue, habitStats, habitSeries } from "@/lib/habits"
    export let habits: Habit[] = []
    export let disabled = false
    export let onSave: (habits: Habit[]) => Promise<boolean>
    let today = dayKey(new Date()), day = today, period = 7, filter = "all"
    let chart: "ring" | "line" | "bars" = "ring"
    let title = "", target = 1, unit = "раз", showForm = false
    let busy = false, message = ""
    let selectedPoint: string | null = null
    onMount(() => {
        const timer = setInterval(() => {
            const next = dayKey(new Date())
            if (next !== today) { if (day === today) day = next; today = next }
        }, 30000)
        return () => clearInterval(timer)
    })
    $: visible = habits.filter(habit => activeOn(habit, day))
    $: daily = habitStats(habits, day)
    $: tracked = filter === "all" ? habits : habits.filter(habit => habit.id === filter)
    $: series = habitSeries(tracked, day, Number(period))
    $: opportunities = series.reduce((sum, point) => sum + point.total, 0)
    $: completed = series.reduce((sum, point) => sum + point.done, 0)
    $: percentage = opportunities ? Math.round(completed / opportunities * 100) : 0
    $: points = series.map((point, index) => ({ ...point, x: 32 + index * 476 / (series.length - 1), y: 160 - (point.percent || 0) * 1.3 }))
    $: segments = points.reduce<string[]>((lines, point, index) => {
        if (point.percent === null) return lines
        if (!index || points[index - 1].percent === null) lines.push(`M ${point.x} ${point.y}`)
        else lines[lines.length - 1] += ` L ${point.x} ${point.y}`
        return lines
    }, [])
    function shortDate(value: string) { return new Date(`${value}T12:00`).toLocaleDateString("ru-RU", { day: "numeric", month: "short" }) }
    async function save(next: Habit[]) {
        if (disabled || busy) return false
        busy = true; message = ""
        try { return await onSave(next) }
        catch { message = "Не удалось сохранить отметку. Повторите попытку."; return false }
        finally { busy = false }
    }
    async function update(habit: Habit, value: number) {
        if (day > today) return
        const entries = { ...habit.entries, [day]: Math.min(habit.target, Math.max(0, value)) }
        await save(habits.map(item => item.id === habit.id ? { ...item, entries } : item))
    }
    async function add() {
        if (!title.trim() || !Number.isInteger(Number(target)) || target < 1 || target > 1000) return
        const next: Habit = { id: makeId(), title: title.trim(), caption: "Ваша ежедневная привычка", icon: "✦", target: Number(target), unit: unit.trim() || "раз", createdOn: today, entries: {} }
        if (await save([...habits, next])) { title = ""; target = 1; showForm = false; day = today }
    }
    async function archive(habit: Habit) {
        if (!confirm(`Убрать привычку «${habit.title}»? История предыдущих дней останется в статистике. Сегодняшняя цель будет исключена.`)) return
        await save(habits.map(item => item.id === habit.id ? { ...item, archivedOn: today } : item))
    }
</script>

<section class="habits" aria-label="Привычки и статистика">
    <header><div><p class="eyebrow">КАЖДЫЙ ДЕНЬ · МАЛЕНЬКИЕ ШАГИ</p><h2>Привычки на сегодня</h2><p class="muted">Большие перемены начинаются с одной отметки.</p></div><input type="date" aria-label="День отметок" bind:value={day} max={today} required on:change={() => { if (!day || day > today) day = today }} /></header>
    <div class="overview"><div class="mini-ring" style={`--progress:${daily.percent || 0}%`}><strong>{daily.done}<small>/{daily.total}</small></strong></div><div><strong>{daily.done} из {daily.total} выполнено</strong><p>{daily.total && daily.done === daily.total ? "Все цели дня выполнены. Отличная работа!" : "В своём темпе — шаг за шагом."}</p></div><span class="day-label">{shortDate(day)}</span></div>
    <div class="layout"><div class="list">
        {#each visible as habit (habit.id)}
            <article class:complete={habitValue(habit, day) >= habit.target}>
                <span class="habit-icon" aria-hidden="true">{habit.icon}</span><div class="copy"><h3>{habit.title}</h3><p>{habit.caption}</p>{#if habit.target > 1}<div class="meter"><span style={`width:${habitValue(habit, day) / habit.target * 100}%`}></span></div><small>{habitValue(habit, day)} / {habit.target} {habit.unit}</small>{/if}</div>
                {#if habit.target === 1}<button class="check" class:checked={habitValue(habit, day) === 1} title={habitValue(habit, day) === 1 ? "Снять отметку" : "Отметить выполнение"} aria-label={`Отметить: ${habit.title}`} aria-pressed={habitValue(habit, day) === 1} disabled={disabled || busy} on:click={() => update(habit, habitValue(habit, day) === 1 ? 0 : 1)}><span class="check-core" aria-hidden="true"></span><svg class="check-glyph" viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 4.5 4.5L19 7"/></svg></button>
                {:else}<div class="stepper"><button title="Уменьшить на один" aria-label={`Уменьшить: ${habit.title}`} disabled={disabled || busy || habitValue(habit, day) === 0} on:click={() => update(habit, habitValue(habit, day) - 1)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12h12"/></svg></button><button title="Добавить один" aria-label={`Добавить: ${habit.title}`} disabled={disabled || busy || habitValue(habit, day) >= habit.target} on:click={() => update(habit, habitValue(habit, day) + 1)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12h12M12 6v12"/></svg></button></div>{/if}
                {#if day === today}<button class="archive" title="Убрать привычку, сохранив прошлую историю" aria-label={`Убрать привычку: ${habit.title}`} disabled={disabled || busy} on:click={() => archive(habit)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="m7 7 10 10M17 7 7 17"/></svg></button>{/if}
            </article>
        {/each}
        {#if !visible.length}<p class="empty">На этот день привычек ещё нет. Отметки до создания привычки не учитываются.</p>{/if}
        <button class="add" on:click={() => showForm = !showForm} aria-expanded={showForm}>+ Своя привычка</button>
        {#if showForm}<form on:submit|preventDefault={add}><input bind:value={title} maxlength="100" placeholder="Например: выучить новые слова" aria-label="Название привычки" required/><div class="form-row"><label>Цель в день<input type="number" min="1" max="1000" step="1" bind:value={target} required /></label><label>Единица<input bind:value={unit} maxlength="20" placeholder="раз / минут / страниц" /></label></div><button class="add" disabled={disabled || busy}>Создать привычку</button></form>{/if}
        {#if message}<p role="alert">{message}</p>{/if}
    </div><aside class="stats" aria-label="Статистика привычек">
        <p class="eyebrow">ВАШ РИТМ</p><h3>Статистика</h3><div class="filters"><select aria-label="Привычка для статистики" bind:value={filter}><option value="all">Все привычки</option>{#each habits as habit}<option value={habit.id}>{habit.title}{habit.archivedOn ? " · архив" : ""}</option>{/each}</select><select aria-label="Период статистики" bind:value={period}><option value={7}>7 дней</option><option value={30}>30 дней</option></select></div>
        <div class="tabs" aria-label="Вид графика"><button class:active={chart === "ring"} aria-pressed={chart === "ring"} on:click={() => chart = "ring"}>◉ Круг</button><button class:active={chart === "line"} aria-pressed={chart === "line"} on:click={() => chart = "line"}>⌁ Линия</button><button class:active={chart === "bars"} aria-pressed={chart === "bars"} on:click={() => chart = "bars"}>▥ Столбцы</button></div>
        <div class="chart">
        {#if !opportunities}<p class="empty">За этот период пока нет целей. График появится после добавления привычек.</p>
        {:else if chart === "ring"}<svg viewBox="0 0 240 210" role="img" aria-label={`Выполнено ${completed} из ${opportunities} целей: ${percentage}%`}><circle cx="120" cy="100" r="73" fill="none" stroke="#20383e" stroke-width="17"/><circle cx="120" cy="100" r="73" fill="none" stroke="#60f3e9" stroke-width="17" stroke-linecap={completed ? "round" : "butt"} pathLength="100" stroke-dasharray={`${completed / opportunities * 100} 100`} transform="rotate(-90 120 100)"/><text x="120" y="104" text-anchor="middle" fill="#eaffff" font-size="36" font-weight="800">{percentage}%</text><text x="120" y="131" text-anchor="middle" fill="#96b4b8" font-size="12">выполнено</text></svg>
        {:else}<svg viewBox="0 0 540 210" role="img" aria-label={`${chart === "line" ? "Линейный" : "Столбчатый"} график выполнения привычек за ${period} дней`}><title>Доля выполненных целей за каждый день</title>{#each [0, 50, 100] as tick}<line x1="32" x2="520" y1={160 - tick * 1.3} y2={160 - tick * 1.3} stroke="#29474d" stroke-dasharray="3 5"/><text x="25" y={164 - tick * 1.3} fill="#86a5ac" text-anchor="end" font-size="10">{tick}%</text>{/each}
        {#if chart === "line"}{#each segments as path}<path d={path} fill="none" stroke="#60f3e9" stroke-width="3" stroke-linejoin="round"/>{/each}{/if}
        {#each points as point, index}{#if point.percent !== null}{#if chart === "line"}<circle class="point-dot" cx={point.x} cy={point.y} r="4.5" fill="#60f3e9" stroke="#0d1b20" stroke-width="2"/>{:else}<rect x={point.x - (period === 7 ? 16 : 5)} y={point.y} width={period === 7 ? 32 : 10} height={Math.max(2,160 - point.y)} rx="3" fill={point.percent ? "#60f3e9" : "#49686c"}/>{/if}{/if}{#if period === 7 || index === 0 || index === 14 || index === 29}<text x={point.x} y="187" fill="#91abb1" text-anchor="middle" font-size="10">{shortDate(point.day)}</text>{/if}{/each}
        {#each points.filter(point => point.percent !== null) as point}
            <g class="chart-point" class:selected={selectedPoint === point.day} role="button" tabindex="0" aria-label={`${shortDate(point.day)}: выполнено ${point.done} из ${point.total} целей, ${point.percent}%`} aria-pressed={selectedPoint === point.day} on:click={()=>selectedPoint = selectedPoint === point.day ? null : point.day} on:keydown={(event)=>{if(event.key === 'Enter' || event.key === ' '){event.preventDefault();selectedPoint = selectedPoint === point.day ? null : point.day}else if(event.key === 'Escape'){selectedPoint=null}}}>
                <circle class="point-hit" cx={point.x} cy={point.y} r={period === 7 ? 17 : 8} fill="transparent"/>
                <g class="point-detail" pointer-events="none">
                    <line x1={point.x} x2={point.x} y1="30" y2="160" stroke="#60f3e9" stroke-opacity=".25" stroke-dasharray="3 5"/>
                    <circle cx={point.x} cy={point.y} r="7" fill="#60f3e9" stroke="#d4fffa" stroke-width="2"/>
                    <g class="point-tooltip" role="tooltip" transform={`translate(${Math.max(35, Math.min(345, point.x - 90))}, ${point.y < 90 ? point.y + 20 : point.y - 82})`}>
                        <rect width="180" height="72" rx="12" fill="#102a30" stroke="#428d8d"/>
                        <text x="14" y="21" fill="#a4c8cd" font-size="12">{shortDate(point.day)}</text>
                        <text x="14" y="44" fill="#efffff" font-size="17" font-weight="800">{point.done} / {point.total}<tspan x="164" text-anchor="end" fill="#60f3e9">{point.percent}%</tspan></text>
                        <text x="14" y="61" fill="#a4c8cd" font-size="11">целей выполнено</text>
                    </g>
                </g>
            </g>
        {/each}</svg>{/if}
        </div><div class="numbers"><div><strong>{completed}</strong><span>выполнено</span></div><div><strong>{opportunities - completed}</strong><span>не выполнено</span></div><div><strong>{series.filter(point => point.total > 0 && point.done === point.total).length}</strong><span>идеальных дней</span></div></div>
        <p class="footnote">{shortDate(series[0].day)} — {shortDate(day)}. Только ваши сохранённые отметки. Считаются ежедневные цели с даты создания; неполная цель не считается выполненной. Архивные привычки учитываются до дня удаления.</p>
        <details><summary>Данные по дням</summary><div class="history">{#each [...series].reverse() as point}<div><span>{shortDate(point.day)}</span><span>{point.total ? `${point.done} / ${point.total} · ${point.percent}%` : "Нет целей"}</span></div>{/each}</div></details>
    </aside></div>
</section>

<style>
    .habits article{border-radius:16px;padding:1rem .85rem;gap:.8rem;transition:border-color .22s ease,background .22s ease,box-shadow .22s ease}.habits article:hover{border-color:#427176}.habits article.complete{box-shadow:inset 3px 0 0 #60f3e9}
    .habits .check,.habits .stepper button,.habits .archive{display:grid;place-items:center;flex:none;padding:0;border-radius:50%;transition:background .22s ease,border-color .22s ease,box-shadow .22s ease,transform .22s ease;color:var(--accent)}
    .habits .check{width:36px;height:36px;position:relative;border:1px solid #4b777b;background:#0c2025;box-shadow:inset 0 1px 2px #0004}.check-core{width:8px;height:8px;background:#45686c;border-radius:50%;transition:opacity .2s ease,transform .2s ease}.habits .check.checked{background:#60f3e9;border-color:#a0fff5;box-shadow:0 0 16px #60f3e922;color:#092426}.checked .check-core{opacity:0;transform:scale(.25)}.check-glyph{position:absolute;width:21px;height:21px;opacity:0;transform:scale(.5);transition:opacity .2s ease,transform .25s ease}.checked .check-glyph{opacity:1;transform:scale(1)}
    .habits .stepper{gap:.4rem;flex:none}.habits .stepper button{width:34px;height:34px;border:1px solid #386368;background:#15353a;box-shadow:inset 0 1px 0 #ffffff08}.habits .stepper button+button{background:#17464a;border-color:#408183}.stepper svg{width:17px;height:17px}.habits .archive{width:28px;height:28px;border:1px solid transparent;background:transparent;color:#77969c}.archive svg{width:14px;height:14px}.habits .archive:not(:disabled):hover{color:#ffb8ad;background:#492a2c;border-color:#8d5353}
    .check-glyph path,.stepper path,.archive path{fill:none;stroke:currentColor;stroke-width:2.3;stroke-linecap:round;stroke-linejoin:round}.habits .check:not(:disabled):hover,.habits .stepper button:not(:disabled):hover{border-color:var(--accent);box-shadow:0 0 0 4px #60f3e90c;transform:translateY(-1px)}.habits .check:not(:disabled):active,.habits .stepper button:not(:disabled):active,.habits .archive:not(:disabled):active{transform:scale(.92)}
    .point-detail{display:none}.chart-point{cursor:pointer;outline:none}.chart-point:hover .point-detail,.chart-point:focus-visible .point-detail,.chart-point.selected .point-detail{display:block}.point-tooltip{filter:drop-shadow(0 5px 8px #0005)}
    @media(prefers-reduced-motion:reduce){.habits article,.habits .check,.habits .stepper button,.habits .archive,.check-core,.check-glyph{transition:none}}
    .habits{--accent:#60f3e9;--line:#29494f;--muted:#91abb1;color:#ecffff;font-family:"Manrope Variable",sans-serif}header{display:flex;justify-content:space-between;gap:1rem;align-items:center;margin-bottom:1rem}.eyebrow{font-size:.61rem;letter-spacing:.13em;color:var(--accent);font-weight:800;margin:0}h2{font-size:1.5rem;letter-spacing:-.035em;margin:.4rem 0}h3{margin:0;font-size:.86rem}.muted,.copy p{color:var(--muted);font-size:.7rem;margin:.3rem 0}.overview{display:flex;align-items:center;gap:1rem;padding:1rem;border:1px solid #357174;border-radius:15px;background:radial-gradient(ellipse at right,#17505466,transparent 70%),#10292e;margin-bottom:1rem}.overview strong{font-size:1rem}.overview p{font-size:.72rem;color:#a2cdcb;margin:.35rem 0}.mini-ring{display:grid;place-items:center;flex:none;width:70px;height:70px;border-radius:50%;background:conic-gradient(var(--accent) var(--progress),#28434a 0);position:relative}.mini-ring:before{content:"";position:absolute;inset:7px;background:#10292e;border-radius:50%}.mini-ring strong{position:relative;font-size:1.2rem}.mini-ring small{color:#91abb1;font-size:.8rem}.day-label{margin-left:auto;color:var(--accent);font-size:.7rem;white-space:nowrap}.layout{display:grid;grid-template-columns:minmax(0,1.1fr) minmax(0,1fr);gap:1rem;align-items:start}.list{display:flex;flex-direction:column;gap:.55rem}article{display:flex;align-items:center;gap:.65rem;padding:.8rem .65rem;border:1px solid var(--line);border-radius:11px;background:linear-gradient(110deg,#122b30,#0d1b20)}article.complete{border-color:#38786f;background:linear-gradient(110deg,#153b39,#0d1b20)}.habit-icon{display:grid;place-items:center;width:38px;height:42px;border-radius:10px;background:#203e42;color:var(--accent);font-size:1.35rem;flex:none}.copy{flex:1;min-width:0}.copy h3{overflow-wrap:anywhere;font-size:.75rem}.copy p,.copy small{font-size:.6rem;color:#91abb1}.check{width:30px;height:30px;border:1px solid #54787e;border-radius:9px;background:transparent;font-size:1.2rem;flex:none}.checked{background:#60f3e9;color:#092426;border-color:#60f3e9}.stepper{display:flex;gap:.25rem}.stepper button{width:26px;height:30px}.archive{border:0;background:none;color:#698b91;padding:0;font-size:1rem}.meter{height:4px;background:#28434a;border-radius:8px;margin:.5rem 0 .25rem;overflow:hidden}.meter span{display:block;height:100%;background:var(--accent);transition:width .2s}.stats{padding:1rem;border:1px solid var(--line);border-radius:14px;background:linear-gradient(145deg,#18383c66,transparent),#0d1b20}.stats h3{font-size:1.2rem;margin:.4rem 0 .9rem}.filters{display:flex;gap:.4rem}.filters select:first-child{flex:1;min-width:0}.tabs{display:flex;gap:.25rem;margin-top:.8rem}.tabs button{flex:1;font-size:.62rem;padding:.5rem .25rem}.tabs .active{border-color:#60f3e9;background:#174f53;color:#d8fffa}.chart{min-height:185px;display:grid;place-items:center;margin:.5rem 0}.chart svg{display:block;width:100%;max-height:220px}.numbers{display:grid;grid-template-columns:repeat(3,1fr);gap:.4rem;border-top:1px solid var(--line);padding-top:.8rem}.numbers div{display:flex;flex-direction:column;gap:.3rem}.numbers strong{font-size:1.3rem;color:var(--accent)}.numbers span{font-size:.57rem;color:var(--muted)}.footnote{font-size:.6rem;color:var(--muted);line-height:1.6;margin:1rem 0}.add{padding:.7rem;border-color:#398487;color:var(--accent);text-align:center}.form-row{display:flex;gap:.5rem}.form-row label{flex:1;min-width:0;color:#91abb1;font-size:.65rem}form{display:flex;flex-direction:column;gap:.6rem;border:1px solid var(--line);padding:.7rem;border-radius:10px}input,select,button{font:inherit;border:1px solid var(--line);border-radius:6px;background:#10262b;color:#e3ffff;padding:.5rem;font-size:.7rem}input{color-scheme:dark;min-width:0}form input{width:100%;box-sizing:border-box}button{cursor:pointer}button:hover{border-color:var(--accent)}button:disabled{opacity:.45;cursor:default}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid var(--accent);outline-offset:2px}.empty{font-size:.75rem;color:var(--muted);line-height:1.6}summary{cursor:pointer;font-size:.65rem;color:var(--accent)}.history{max-height:220px;overflow:auto;margin-top:.6rem}.history div{display:flex;justify-content:space-between;padding:.4rem 0;border-bottom:1px solid var(--line);font-size:.64rem;color:#aec9cc}@media(max-width:1000px){.layout{grid-template-columns:1fr}.stats{order:1}}@media(max-width:520px){header{align-items:flex-start;flex-direction:column}.day-label{display:none}.overview{gap:.6rem;padding:.8rem}h2{font-size:1.25rem}}@media(prefers-reduced-motion:reduce){.meter span{transition:none}}
</style>
