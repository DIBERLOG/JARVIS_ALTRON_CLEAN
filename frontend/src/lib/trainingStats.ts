import { dayKey } from "./center"
import { workingSets, trainingVolume, estimatedMax, type TrainingData, type TrainingSession } from "./training"

export const moodLabels = ["", "Плохое", "Ниже обычного", "Обычное", "Хорошее", "Отличное"]
export const validMood = (value: unknown): value is number => typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 5
export function statsSessions(data: TrainingData, days: number, now = new Date()) {
    const end = dayKey(now), startDate = new Date(now.getFullYear(), now.getMonth(), now.getDate())
    startDate.setDate(startDate.getDate() - days + 1)
    const start = dayKey(startDate)
    return data.sessions.filter(session => {
        const day = dayKey(new Date(session.startedAt))
        return session.status === "completed" && day <= end && (days === 0 || day >= start)
    }).sort((a,b) => a.startedAt.localeCompare(b.startedAt))
}
export function muscleStats(sessions: TrainingSession[]) {
    const groups = new Map<string, number>()
    for (const session of sessions) for (const set of workingSets(session)) {
        const group = session.items.find(item => item.exerciseId === set.exerciseId)?.muscle || "Другое"
        groups.set(group, (groups.get(group) || 0) + 1)
    }
    return [...groups].map(([group, sets]) => ({ group, sets })).sort((a,b) => b.sets-a.sets)
}
export function trainingTotals(sessions: TrainingSession[]) {
    const moods = sessions.map(session => session.mood).filter(validMood)
    return { sessions: sessions.length, sets: sessions.reduce((sum, session) => sum + workingSets(session).length, 0),
        volume: sessions.reduce((sum, session) => sum + trainingVolume(session), 0),
        mood: moods.length ? moods.reduce((sum,value) => sum+value,0)/moods.length : null, moodCount: moods.length }
}
export function relativeStrength(session: TrainingSession, exerciseId: string): number | null {
    if (!session.bodyWeight || session.bodyWeight <= 0) return null
    const maximum = Math.max(0,...workingSets(session,exerciseId).map(set=>estimatedMax(set)||0))
    return maximum ? maximum/session.bodyWeight : null
}
export function chartPoints(rows: { date: string; value: number }[]) {
    const values = rows.filter(row => Number.isFinite(row.value)).sort((a,b)=>a.date.localeCompare(b.date))
    if (!values.length) return { points: [], low: 0, high: 1, path: "" }
    const min = Math.min(...values.map(row=>row.value)), max = Math.max(...values.map(row=>row.value))
    const pad = Math.max(.5,(max-min)*.2), low = min-pad, high = max+pad
    const first = Date.parse(values[0].date), last = Date.parse(values[values.length-1].date)
    const points = values.map(row => ({ ...row, x: first===last ? 280 : 48+(Date.parse(row.date)-first)/(last-first)*464, y: 150-(row.value-low)/(high-low)*112 }))
    return { points, low, high, path: points.map((point,index)=>`${index ? "L":"M"} ${point.x} ${point.y}`).join(" ") }
}
