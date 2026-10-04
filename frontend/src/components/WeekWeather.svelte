<script lang="ts">
    import { onMount } from "svelte"
    import WeatherIcon from "./WeatherIcon.svelte"
    import JarvisSelect from "./JarvisSelect.svelte"
    import { weatherPeriod } from '@/lib/weather'
    import { chartGeometry, clothingFor, getWeekWeather, getWeatherCity, temperature, weatherLabel, type WeekWeather } from "@/lib/weather"

    let forecast: WeekWeather | null = null
    let period = 7, page = 0
    $: period = $weatherPeriod
    let loading = false, error = "", cityInput = "", selected = 0
    const date = (value: string) => new Date(`${value}T12:00:00`)
    const weekday = (value: string) => date(value).toLocaleDateString("ru-RU", { weekday: "short" })
    const shortDate = (value: string) => date(value).toLocaleDateString("ru-RU", { day: "numeric", month: "short" }).replace(".", "")
    const maximum = (values: (number | null)[]) => { const available = values.filter((v): v is number => v !== null); return available.length ? Math.max(...available) : null }
    $: viewDays = forecast?.days.slice(page*7,page*7+7) || []
    $: chart = viewDays.length ? chartGeometry(viewDays) : null
    $: chosenDay = viewDays[selected]
    $: tips = chosenDay ? clothingFor(chosenDay) : []
    $: average = forecast ? forecast.days.reduce((sum, day) => sum + day.high, 0) / forecast.days.length : 0
    $: rain = forecast ? maximum(forecast.days.map(day => day.rain)) : null
    $: wind = forecast ? maximum(forecast.days.map(day => day.wind)) : null
    $: uv = forecast ? maximum(forecast.days.map(day => day.uv)) : null

    async function refresh(force = true, city?: string) {
        if (loading) return
        loading = true; error = ""
        const requestPeriod=period
        try {
            const next = await getWeekWeather(city, force,requestPeriod)
            if(requestPeriod!==period)return
            const selectedDate = chosenDay?.date
            selected = Math.max(0, next.days.slice(page*7,page*7+7).findIndex(day => day.date === selectedDate))
            forecast = next; cityInput = next.requested_city; error = next.warning || ""
        } catch (e) { error = String(e) }
        finally { loading = false; if(requestPeriod!==period)void refresh(true) }
    }
    async function syncCity() {
        if (loading) return
        try {
            const city = await getWeatherCity()
            if (forecast && city.toLowerCase() !== forecast.requested_city.toLowerCase()) await refresh(true)
        } catch (e) { console.error("Не удалось проверить город погоды", e) }
    }
    onMount(() => {
        const unsubscribe = weatherPeriod.subscribe(value=>{period=value;page=0;selected=0;if(!loading){forecast=null;refresh(true)}})
        const timer = setInterval(() => refresh(true), 10 * 60_000)
        const cityTimer = setInterval(syncCity, 3000)
        window.addEventListener("focus", syncCity)
        return () => { unsubscribe(); clearInterval(timer); clearInterval(cityTimer); window.removeEventListener("focus", syncCity) }
    })
</script>

<section class="weather-card" aria-label="Прогноз погоды на семь дней" aria-busy={loading}>
    <header class="weather-heading">
        <div><p class="kicker"><span class="live-dot"></span> ПОГОДА · {period} ДНЕЙ</p><h2>{forecast?.city || "Прогноз на неделю"}</h2><p class="region">{forecast ? `${shortDate(forecast.days[0].date)} — ${shortDate(forecast.days[forecast.days.length-1].date)} · ${forecast.region}` : "Планы на неделю начинаются с погоды"}</p></div>
        <button class="refresh" on:click={() => refresh()} disabled={loading} title="Обновить прогноз" aria-label="Обновить прогноз"><span class:spinning={loading}>↻</span></button>
    </header>
    <div class="period-controls"><JarvisSelect label="Период" value={$weatherPeriod} disabled={loading} options={[{value:7,label:'Неделя',detail:'Прогноз на 7 дней',icon:'☀'},{value:30,label:'Месяц',detail:'Приблизительная сезонная оценка',icon:'◷'},{value:60,label:'Два месяца',detail:'Приблизительная сезонная оценка',icon:'◈'}]} on:change={event=>weatherPeriod.set(Number(event.detail))}/>{#if forecast && forecast.days.length>7}<button disabled={page===0} on:click={()=>{page--;selected=0}}>←</button><span>{page+1} / {Math.ceil(forecast.days.length/7)}</span><button disabled={(page+1)*7>=forecast.days.length} on:click={()=>{page++;selected=0}}>→</button>{/if}</div>
    {#if period>7}<p class="season-warning">Сезонная оценка ECMWF: средние значения ансамбля, а не точная погода по дням. Высокая неопределённость, сетка около 36 км. Не используйте для выбора одежды или планирования конкретного дня.</p>{/if}
    <form class="city-form" on:submit|preventDefault={() => refresh(true, cityInput.trim())}><svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M19 10c0 5-7 11-7 11S5 15 5 10a7 7 0 1 1 14 0Z" stroke="currentColor" stroke-width="1.7"/><circle cx="12" cy="10" r="2.5" stroke="currentColor" stroke-width="1.7"/></svg><input aria-label="Город для прогноза" bind:value={cityInput} maxlength="100" placeholder="Город, например Котельники" required/><button disabled={loading}>Показать</button></form>
    {#if error}<div class="weather-error" role="alert"><strong>Погода временно недоступна</strong><span>{error}</span>{#if forecast}<small>Ниже — последний полученный прогноз для города {forecast.city}. Время обновления указано под карточкой.</small>{/if}<button on:click={() => refresh(true, cityInput.trim() || undefined)} disabled={loading}>Повторить</button></div>{/if}
    {#if forecast && chart && chosenDay}
        <div class="forecast-board">
            <div class="day-row">{#each viewDays as day, index}<button class:selected={selected === index} on:click={() => selected = index} aria-pressed={selected === index} aria-label={`${shortDate(day.date)}: ${weatherLabel(day.code)}, днём ${temperature(day.high)}, ночью ${temperature(day.low)}`}><strong>{index === 0 ? "Сегодня" : weekday(day.date)}</strong><small>{shortDate(day.date)}</small>{#if !forecast.seasonal}<div class="day-icon"><WeatherIcon code={day.code} id={`week-${index}`}/></div><span class="condition">{weatherLabel(day.code)}</span>{:else}<span class="condition">Модельная оценка</span>{/if}</button>{/each}</div>
            <div class="chart-legend"><span><i class="high-key"></i>Днём</span><span><i class="low-key"></i>Ночью</span><small>°C</small></div>
            <svg class="temperature-chart" viewBox="0 0 700 218" role="img" aria-label={`График дневных и ночных температур в городе ${forecast.city}`}>
                <title>Дневная и ночная температура на семь дней</title>
                <desc>{forecast.days.map(day => `${shortDate(day.date)}: максимум ${temperature(day.high)}, минимум ${temperature(day.low)}`).join("; ")}</desc>
                <defs><linearGradient id="week-area" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="#39ddd8" stop-opacity=".34"/><stop offset="100%" stop-color="#39ddd8" stop-opacity=".015"/></linearGradient><linearGradient id="week-line"><stop stop-color="#64ece2"/><stop offset="100%" stop-color="#63b6ff"/></linearGradient></defs>
                {#each chart.ticks as tick}<line x1="42" x2="688" y1={tick.y} y2={tick.y} stroke="#29494f" stroke-dasharray="4 6"/><text x="31" y={tick.y + 4} text-anchor="end" fill="#78969e" font-size="13">{tick.value}°</text>{/each}
                {#each chart.points as point, index}<line x1={point.x} x2={point.x} y1="20" y2="194" stroke={selected === index ? "#5ce7e2" : "#25444c"} stroke-opacity={selected === index ? .38 : .5} stroke-dasharray="3 6"/>{/each}
                <path d={chart.areaPath} fill="url(#week-area)"/>
                <path d={chart.lowPath} stroke="#7b98bc" stroke-width="2" stroke-dasharray="5 6" fill="none"/>
                <path d={chart.highPath} stroke="url(#week-line)" stroke-width="4" fill="none" stroke-linecap="round"/>
                {#each chart.points as point, index}<circle cx={point.x} cy={point.high} r={selected === index ? 12 : 0} fill="#6ef5e0" fill-opacity=".14"/><circle cx={point.x} cy={point.high} r={selected === index ? 6.5 : 5} fill={selected === index ? "#84fff0" : "#14383f"} stroke="#b2fff7" stroke-width="2.5"/><text x={point.x} y={point.high - 17} fill="#ebffff" text-anchor="middle" font-size="19" font-weight="800">{temperature(viewDays[index].high)}</text><circle cx={point.x} cy={point.low} r="3" fill="#a0badd"/><text x={point.x} y={point.low + 19} fill="#90a9c4" text-anchor="middle" font-size="13">{temperature(viewDays[index].low)}</text>{/each}
            </svg>
        </div>
        <div class="week-stats"><div><span>Средний максимум</span><strong>{temperature(average)}<small>C</small></strong></div><div><span>Осадки · максимум</span><strong>{rain === null ? "—" : Math.round(rain)}<small>{rain === null ? "" : "%"}</small></strong></div><div><span>Ветер · максимум</span><strong>{wind === null ? "—" : wind.toFixed(1)}<small>{wind === null ? "" : "м/с"}</small></strong></div><div><span>УФ · максимум</span><strong>{uv === null ? "—" : uv.toFixed(1)}</strong></div></div>
        {#if !forecast.seasonal}<div class="clothing"><div class="clothing-heading"><div><span class="shirt-mark"><svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="m8 4-6 4 3 5 3-2v9h8v-9l3 2 3-5-6-4c-1 3-7 3-8 0Z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg></span><h3>Что надеть</h3></div><span>{shortDate(chosenDay.date)}</span></div><div class="clothing-tips">{#each tips as tip}<div class="tip"><svg viewBox="0 0 24 24" fill="none" aria-hidden="true">{#if tip.icon === "umbrella"}<path d="M3 12a9 9 0 0 1 18 0H3Zm9-9v18c0 2-4 2-4-1"/>{:else if tip.icon === "shoe"}<path d="M3 14V9h5l3 4 9 3v4H3v-6Zm1 2h15M9 12l3-2m0 5 3-2"/>{:else if tip.icon === "sun"}<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l2 2m10 10 2 2M5 19l2-2M17 7l2-2"/>{:else if tip.icon === "wind"}<path d="M2 8h15a3 3 0 1 0-3-3M2 12h18M2 16h12a3 3 0 1 1-3 3"/>{:else if tip.icon === "coat"}<path d="m8 3-5 5-1 11h4l2-9v12h8V10l2 9h4L21 8l-5-5-4 4-4-4Zm4 4v15M8 3l1 6 3-2 3 2 1-6"/>{:else}<path d="m8 4-6 4 3 5 3-2v9h8v-9l3 2 3-5-6-4c-1 3-7 3-8 0Z"/>{/if}</svg><span>{tip.text}</span></div>{/each}</div></div>{/if}
        <footer class="weather-footer"><a href="https://open-meteo.com/" target="_blank" rel="noreferrer">Open-Meteo ↗</a><span>Обновлено {new Date(forecast.updated_at).toLocaleTimeString("ru-RU", { hour: "2-digit", minute: "2-digit" })} · {forecast.timezone}</span></footer>
    {:else if loading}
        <div class="weather-loading" role="status"><div class="loading-orbit"></div><strong>Собираю прогноз на неделю</strong><span>Температура, осадки и рекомендации по одежде</span></div>
    {:else}
        <div class="weather-empty"><span>☁</span><p>Укажите город или повторите загрузку прогноза.</p></div>
    {/if}
</section>

<style>
    .period-controls{display:flex;align-items:center;gap:.6rem;margin:.8rem 0;font-size:.7rem;color:#c2e7e4}.period-controls select,.period-controls button{background:#102b31;border:1px solid #36585f;border-radius:8px;padding:.5rem;color:#eaffff}.season-warning{padding:.8rem;border:1px solid #817445;border-radius:9px;background:#302a19;color:#efdfa3;font-size:.7rem;line-height:1.7}
    .weather-card{--weather-accent:#6cecdf;--weather-blue:#79bcff;--weather-line:#28474f;color:#e9f7fb;border:1px solid #36585f;border-radius:16px;padding:1.15rem;background:radial-gradient(ellipse at 100% 0%,#277b842c,transparent 55%),linear-gradient(150deg,#12292f,#0b1b22 65%);box-shadow:0 18px 40px #0003;font-family:"Manrope Variable",sans-serif;overflow:hidden}.weather-heading{display:flex;justify-content:space-between;gap:.8rem;align-items:center}.kicker{display:flex;align-items:center;gap:.4rem;margin:0;color:var(--weather-accent);font-size:.59rem;font-weight:800;letter-spacing:.15em}.live-dot{width:6px;height:6px;border-radius:50%;background:var(--weather-accent);box-shadow:0 0 9px #5fe7d9}.weather-heading h2{margin:.5rem 0 .3rem;font-size:1.65rem;line-height:1.1;letter-spacing:-.045em}.region{margin:0;color:#91acb5;font-size:.66rem;line-height:1.5}.refresh{display:grid;place-items:center;flex:none;width:34px;height:34px;border:1px solid #416169;border-radius:9px;background:#203f46;color:#bdfaf0;cursor:pointer}.refresh span{display:block;font-size:1.4rem;line-height:1}.refresh:hover{background:#2d5258}.refresh:disabled{opacity:.55;cursor:wait}.city-form{display:flex;align-items:center;gap:.5rem;margin:1rem 0;border:1px solid #2c4b53;border-radius:8px;padding:.3rem .3rem .3rem .6rem;background:#091a20}.city-form svg{width:16px;height:16px;flex:none;color:#70b9c5}.city-form input{flex:1;min-width:0;outline:none;border:0;background:transparent;padding:.2rem;color:#d6eff5;font:600 .7rem "Manrope Variable",sans-serif}.city-form:focus-within{border-color:var(--weather-accent)}.city-form button,.weather-error button{border:1px solid #48777c;border-radius:5px;background:#20494f;color:#cafff3;padding:.4rem .55rem;font:700 .63rem "Manrope Variable",sans-serif;cursor:pointer}.city-form button:disabled{opacity:.5}.forecast-board{border:1px solid #294b53;border-radius:12px;background:linear-gradient(160deg,#1a3b433f,#10262e66);overflow:hidden}.day-row{display:grid;grid-template-columns:repeat(7,minmax(0,1fr));padding:0 1.714% 0 6%}.day-row button{min-width:0;padding:.7rem .05rem .55rem;border:0;border-right:1px solid #33505a70;border-bottom:2px solid transparent;background:transparent;color:#d3eaf0;text-align:center;cursor:pointer;transition:background .18s,border-color .18s}.day-row button:last-child{border-right:0}.day-row button:hover{background:#2a555c55}.day-row button.selected{background:linear-gradient(#60cbd51f,#60cbd50a);border-bottom-color:var(--weather-accent)}.day-row strong{display:block;font-size:.66rem;font-weight:800;text-transform:capitalize}.day-row small{display:block;margin-top:.18rem;color:#8fa9b6;font-size:.56rem;white-space:nowrap}.day-icon{width:39px;max-width:90%;height:43px;margin:.35rem auto .25rem;filter:drop-shadow(0 5px 9px #0003)}.condition{display:block;min-height:2.5em;padding:0 .08rem;font-size:.55rem;line-height:1.25;overflow-wrap:normal;color:#adcad2}.chart-legend{display:flex;align-items:center;gap:.9rem;padding:.6rem .85rem 0;font-size:.6rem;color:#a8c1ca}.chart-legend span{display:flex;gap:.35rem;align-items:center}.chart-legend i{width:15px;height:2px;display:inline-block}.high-key{background:var(--weather-accent)}.low-key{border-top:2px dashed #7b98bc}.chart-legend small{margin-left:auto;font-size:.6rem;color:#789aa4}.temperature-chart{display:block;width:100%;height:auto;overflow:visible;font-family:"Manrope Variable",sans-serif}.week-stats{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:.4rem;margin:.7rem 0}.week-stats>div{padding:.7rem .6rem;border:1px solid #294a52;border-radius:9px;background:linear-gradient(130deg,#183b434f,#12252d)}.week-stats span{display:block;color:#8daeb8;font-size:.55rem;line-height:1.4;min-height:2.8em}.week-stats strong{display:block;margin-top:.3rem;color:#e4f9fb;font-size:1.15rem;letter-spacing:-.04em;white-space:nowrap}.week-stats strong small{margin-left:.2rem;font-size:.6rem;font-weight:500;letter-spacing:0;color:#95c5cc}.clothing{margin-top:.85rem;padding:.8rem;border:1px solid #31555c;border-radius:11px;background:linear-gradient(120deg,#1c414652,#10282f)}.clothing-heading{display:flex;justify-content:space-between;align-items:center;gap:.5rem}.clothing-heading>div{display:flex;gap:.45rem;align-items:center}.shirt-mark{display:grid;place-items:center;width:25px;height:25px;border-radius:7px;background:#24575b;color:#89f1e2}.shirt-mark svg{width:17px;height:17px}.clothing-heading h3{margin:0;font-size:.83rem;letter-spacing:-.02em}.clothing-heading>span{color:#89afb6;font-size:.62rem}.clothing-tips{display:flex;flex-wrap:wrap;gap:.45rem;margin-top:.65rem}.tip{display:flex;align-items:center;gap:.4rem;flex:1 1 120px;padding:.55rem .5rem;border:1px solid #2f535c;border-radius:8px;background:#16353d;color:#bedde0;min-width:0}.tip svg{flex:none;width:22px;height:22px;stroke:#75d4d0;stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round}.tip span{font-size:.63rem;line-height:1.45}.weather-footer{display:flex;justify-content:space-between;gap:.5rem;margin:.85rem .1rem 0;color:#698d99;font-size:.55rem}.weather-footer a{color:#82b7c5;text-decoration:none}.weather-footer a:hover{color:#c5fffa}.weather-error{display:flex;flex-direction:column;gap:.35rem;margin-bottom:.8rem;padding:.7rem;border:1px solid #92584b;border-radius:8px;background:#412a245c;color:#efc1b5;font-size:.65rem;line-height:1.45;overflow-wrap:anywhere}.weather-error small{color:#c5a69f;font-size:.6rem}.weather-error button{align-self:flex-start}.weather-loading,.weather-empty{display:flex;min-height:340px;flex-direction:column;align-items:center;justify-content:center;gap:.6rem;text-align:center}.weather-loading strong{font-size:.9rem}.weather-loading span,.weather-empty p{font-size:.7rem;color:#85a6b1}.loading-orbit{width:42px;height:42px;margin-bottom:.4rem;border:2px solid #284c57;border-top-color:var(--weather-accent);border-radius:50%;animation:spin 1s linear infinite}.spinning{animation:spin 1s linear infinite}.weather-empty>span{font-size:3rem;color:#6f9fae}@keyframes spin{to{transform:rotate(360deg)}}button:focus-visible,a:focus-visible{outline:2px solid #a6fff3;outline-offset:3px}@media(prefers-reduced-motion:reduce){.loading-orbit,.spinning{animation:none}.day-row button{transition:none}}@media(max-width:520px){.weather-card{padding:.8rem}.weather-heading h2{font-size:1.4rem}.week-stats{grid-template-columns:repeat(2,minmax(0,1fr))}.week-stats span{min-height:0}.day-row strong{font-size:.6rem}.day-row small{font-size:.5rem}.condition{font-size:.48rem}.day-icon{height:38px}.weather-footer{flex-wrap:wrap}}
</style>
