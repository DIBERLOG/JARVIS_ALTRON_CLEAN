export type Exercise = { id: string; name: string; muscle: string; equipment: string }
export type PlanItem = { exerciseId: string; sets: number; minReps: number; maxReps: number; rest: number }
export type TrainingPlan = { id: string; name: string; items: PlanItem[] }
export type TrainingSet = { id: string; exerciseId: string; weight: number; reps: number; rpe: number | null; warmup: boolean; at: string }
export type BodyMeasurement = { id: string; date: string; weight: number }
export type TrainingProfile = { heightCm: number | null; measurements: BodyMeasurement[]; bodyFatPercent?: number | null; bodyFatMethod?: string; experience?: string; preferredLevel?: number; limitations?: string }
export type TrainingContext = { mood?: number | null; heightCm?: number | null; bodyWeight?: number | null; plannedLevel?: number }
export type TrainingSession = TrainingContext & { id: string; planId: string; name: string; startedAt: string; finishedAt?: string; status: "active" | "paused" | "completed" | "cancelled"; elapsed: number; resumedAt: number | null; items: (PlanItem & Exercise)[]; sets: TrainingSet[]; warmup: boolean[]; note: string }
export type ChargeStep = { id:string; name:string; hint:string; enabled?:boolean }
export type TrainingData = { exercises: Exercise[]; plans: TrainingPlan[]; sessions: TrainingSession[]; profile?: TrainingProfile; morningExercise?: Record<string, string[]>; chargeSettings?: {level:number; steps:ChargeStep[]}; chargeLevels?: Record<string,number> }
export const trainingId = () => crypto.randomUUID()
export function defaultTraining(): TrainingData {
    const definitions = [
        ["bench", "Жим лёжа", "Грудь", "Штанга"], ["incline", "Жим гантелей под углом", "Грудь", "Гантели"],
        ["fly", "Сведение рук в тренажёре", "Грудь", "Тренажёр"], ["press", "Жим над головой", "Плечи", "Гантели"],
        ["triceps", "Разгибание рук на блоке", "Трицепс", "Блок"], ["pulldown", "Тяга верхнего блока", "Спина", "Блок"],
        ["row", "Горизонтальная тяга", "Спина", "Блок"], ["curl", "Сгибание рук", "Бицепс", "Гантели"],
        ["squat", "Приседания", "Ноги", "Штанга"], ["legpress", "Жим ногами", "Ноги", "Тренажёр"],
        ["legcurl", "Сгибание ног", "Ноги", "Тренажёр"], ["calf", "Подъёмы на носки", "Ноги", "Тренажёр"],
        ["crunch", "Скручивания", "Пресс", "Собственный вес"],
    ]
    const item = (exerciseId: string, minReps = 8, maxReps = 12, rest = 120): PlanItem => ({ exerciseId, sets: 3, minReps, maxReps, rest })
    return {
        exercises: definitions.map(([id, name, muscle, equipment]) => ({ id, name, muscle, equipment })),
        plans: [
            { id: "push", name: "PUSH · Грудь, плечи, трицепс", items: [item("bench", 6, 8, 180), item("incline", 8, 10), item("fly", 10, 15, 90), item("press"), item("triceps", 10, 15, 90)] },
            { id: "pull", name: "PULL · Спина и бицепс", items: [item("pulldown"), item("row"), item("curl", 10, 15, 90)] },
            { id: "legs", name: "LEGS · Ноги", items: [item("squat", 6, 8, 180), item("legpress"), item("legcurl"), item("calf", 12, 15, 90)] },
            { id: "full", name: "FULL BODY · Всё тело", items: [item("squat", 6, 8, 180), item("bench", 6, 8, 180), item("row"), item("press"), item("crunch", 12, 15, 90)] },
        ], sessions: [],
    }
}
export function newTraining(plan: TrainingPlan, exercises: Exercise[], now = Date.now(), context: TrainingContext = {}): TrainingSession {
    const items = plan.items.map(item => {
        const exercise = exercises.find(exercise => exercise.id === item.exerciseId)
        if (!exercise) throw new Error("Упражнение не найдено в библиотеке")
        return { ...exercise, ...item }
    })
    if (!items.length) throw new Error("Добавьте упражнения в программу")
    return { id: trainingId(), planId: plan.id, name: plan.name, startedAt: new Date(now).toISOString(), status: "active", elapsed: 0, resumedAt: now, items, sets: [], warmup: [false, false, false], note: "", ...context }
}
export function trainingDuration(session: TrainingSession, now = Date.now()) {
    return session.elapsed + (session.status === "active" && session.resumedAt !== null ? Math.max(0, now - session.resumedAt) : 0)
}
export function changeTrainingStatus(session: TrainingSession, status: TrainingSession["status"], now = Date.now()): TrainingSession {
    return { ...session, status, elapsed: trainingDuration(session, now), resumedAt: status === "active" ? now : null,
        ...(status === "completed" || status === "cancelled" ? { finishedAt: new Date(now).toISOString() } : {}) }
}
export function workingSets(session: TrainingSession, exerciseId?: string) {
    return session.sets.filter(set => !set.warmup && (!exerciseId || set.exerciseId === exerciseId))
}
export function trainingVolume(session: TrainingSession) { return workingSets(session).reduce((sum, set) => sum + set.weight * set.reps, 0) }
export function estimatedMax(set: TrainingSet) { return set.weight > 0 && set.reps >= 1 && set.reps <= 10 ? set.weight * (1 + set.reps / 30) : null }
export function previousTraining(data: TrainingData, exerciseId: string, excluding?: string) {
    return [...data.sessions].filter(session => session.id !== excluding && session.status === "completed" && workingSets(session, exerciseId).length).sort((a, b) => b.startedAt.localeCompare(a.startedAt))[0]
}
export function progression(item: PlanItem, sets: TrainingSet[]) {
    const baseline = sets[0]?.weight
    if (baseline === undefined) return "Нет прошлых рабочих подходов — укажите свой вес."
    if (sets.some(set => set.weight !== baseline)) return "Вес в подходах различался — сравните историю перед изменением нагрузки."
    if (sets.length >= item.sets && sets.every(set => set.reps >= item.maxReps && set.rpe !== null && set.rpe <= 8)) return "Верхняя граница повторов достигнута без предельного RPE. Можно рассмотреть небольшой шаг веса; изменение только вручную."
    if (sets.some(set => set.reps < item.minReps || (set.rpe !== null && set.rpe >= 9))) return "Есть недобор повторов или высокий RPE. Не повышайте вес автоматически; оцените отдых и технику."
    return "Сначала закрепите текущий вес и целевой диапазон повторений."
}
export function validateSet(weight: number, reps: number, rpe: number | null) {
    return Number.isFinite(weight) && weight >= 0 && weight <= 1000 && Number.isInteger(reps) && reps >= 1 && reps <= 500
        && (rpe === null || (Number.isInteger(rpe) && rpe >= 6 && rpe <= 10))
}
