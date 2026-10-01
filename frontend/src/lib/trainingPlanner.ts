import { workingSets, type TrainingData, type TrainingPlan, type ChargeStep } from './training'
import { dayKey } from './center'
export const plannerLevels=['Очень лёгкий','Лёгкий','Умеренно лёгкий','Средний','Выше среднего','Интенсивный','Продвинутый']
export type Readiness={fatigue:number; pain:boolean; mood:number|null}
export type Proposal={level:number; reasons:string[]; plan?:TrainingPlan; rest:boolean}
export function dailyProposal(data:TrainingData, readiness:Readiness, random=false, now=Date.now(), rng= Math.random):Proposal {
    const reasons:string[]=[]
    let level=Math.min(6,Math.max(0,Math.trunc(data.profile?.preferredLevel??1)))
    const experience=data.profile?.experience
    if(!experience||experience==='Начинающий'||experience==='После перерыва'){level=Math.min(level,2);reasons.push('Начало занятий или перерыв: объём ограничен лёгким уровнем.')}
    if(readiness.pain)return{level:0,reasons:['Вы отметили боль или плохое самочувствие: тренировку не подбираем. При необходимости обратитесь к специалисту.'],rest:true}
    if(readiness.fatigue>=4)return{level:0,reasons:['Высокая усталость: сегодня предложен отдых, а не интенсивная тренировка.'],rest:true}
    if(readiness.fatigue>=3||(readiness.mood!==null&&readiness.mood<=2)){level=Math.min(level,1);reasons.push('Самочувствие сегодня: сниженный объём.')}
    const sessions=data.sessions.filter(s=>workingSets(s).length&&Date.parse(s.finishedAt||s.startedAt)<=now)
    if(sessions.some(s=>dayKey(new Date(s.finishedAt||s.startedAt))===dayKey(new Date(now))))return{level:0,reasons:['Сегодня уже есть записанные силовые подходы. Повторную силовую нагрузку автоматически не предлагаем.'],rest:true}
    const recent=sessions.filter(s=>now-Date.parse(s.finishedAt||s.startedAt)<48*3600000)
    const heavy=(s:typeof sessions[number])=>(s.plannedLevel??0)>=4||workingSets(s).some(set=>(set.rpe??0)>=9)||workingSets(s).length>=16
    const recentCharge=Object.entries(data.chargeLevels||{}).some(([date,value])=>value>=4&&(data.morningExercise?.[date]?.length||0)>0&&now-Date.parse(date+'T12:00')>=-12*3600000&&now-Date.parse(date+'T12:00')<48*3600000)
    if(recent.some(heavy)||recentCharge){level=Math.min(level,2);reasons.push('В последние 48 часов отмечена интенсивная нагрузка: усиленный день исключён.')}
    const weekly=sessions.filter(s=>now-Date.parse(s.finishedAt||s.startedAt)<7*86400000)
    if(weekly.filter(heavy).length>=2){level=Math.min(level,3);reasons.push('Уже два интенсивных дня за неделю: выбираем умеренный объём.')}
    const worked=new Set(recent.flatMap(s=>workingSets(s).map(set=>s.items.find(item=>item.exerciseId===set.exerciseId)?.muscle).filter(Boolean)))
    const candidates=data.plans.filter(plan=>plan.items.length&&plan.items.every(item=>{const exercise=data.exercises.find(e=>e.id===item.exerciseId);return exercise&&!worked.has(exercise.muscle)}))
    if(!candidates.length)return{level:0,reasons:[...reasons,'Нет программы без повторной нагрузки на недавно работавшие группы. Предлагаем отдых или лёгкую зарядку.'],rest:true}
    const last=[...sessions].sort((a,b)=>b.startedAt.localeCompare(a.startedAt))[0]
    const fresh=candidates.filter(plan=>plan.id!==last?.planId)
    const pool=fresh.length?fresh:candidates
    const source=random?pool[Math.min(pool.length-1,Math.max(0,Math.floor(rng()*pool.length)))]:[...pool].sort((a,b)=>a.items.length-b.items.length)[0]
    if(random) level=Math.max(0,level-Math.floor(rng()*2))
    const plan={...source,name:`${source.name} · ${plannerLevels[level]}`,items:source.items.map(item=>({...item,sets:Math.min(item.sets,[1,1,2,3,3,4,4][level]),rest:Math.max(item.rest,90)}))}
    reasons.push('Выбрана существующая программа без основных групп, нагруженных за последние 48 часов. Вес снарядов не увеличивается автоматически.')
    if(data.profile?.limitations?.trim())reasons.push('В профиле есть ограничения: проверьте упражнения вручную. Текст ограничений не считается медицинской проверкой.')
    reasons.push('Рост, вес и процент жира показываются как контекст, но не используются для угадывания безопасного рабочего веса.')
    return{level,reasons,plan,rest:false}
}
export function chargeProposal(steps:ChargeStep[],data:TrainingData,readiness:Readiness,random=false,now=Date.now(),rng=Math.random){
    const proposal=dailyProposal(data,readiness,false,now,rng)
    let level=proposal.rest?0:proposal.level
    // Any recorded strength activity in the last 48h keeps the morning routine light.
    if(data.sessions.some(s=>workingSets(s).length&&now-Date.parse(s.finishedAt||s.startedAt)>=0&&now-Date.parse(s.finishedAt||s.startedAt)<48*3600000))level=Math.min(level,1)
    if(random)level=Math.max(0,level-Math.floor(rng()*2))
    const blocked=readiness.pain||readiness.fatigue>=4
    const seconds=[30,40,45,50,60,70,80][level],reps=[4,6,8,10,12,14,16][level]
    const enabled=steps.filter(s=>s.enabled!==false)
    const generated=enabled.map(step=>({...step,hint:['walk','arms','finish'].includes(step.id)?`${seconds} секунд · спокойный темп`:['shoulders','squat','heels'].includes(step.id)?`${reps} повторов · без рывков`:step.hint}))
    if(random){for(let i=generated.length-2;i>1;i--){const j=1+Math.floor(rng()*i);[generated[i],generated[j]]=[generated[j],generated[i]]}}
    return{level,steps:blocked?[]:generated,reasons:[...proposal.reasons,'Зарядка не содержит прыжков и не повышает вес отягощений; свои движения сохраняют заданные вами цели.'],blocked}
}
