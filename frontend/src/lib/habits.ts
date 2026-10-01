import { dayKey, type Habit } from "./center"

export const activeOn = (habit: Habit, day: string) => habit.createdOn <= day && (!habit.archivedOn || day < habit.archivedOn)
export const habitValue = (habit: Habit, day: string) => Math.min(habit.target, Math.max(0, Number(habit.entries[day]) || 0))
export function habitStats(habits: Habit[], day: string) {
    const active = habits.filter(habit => activeOn(habit, day))
    const done = active.filter(habit => habitValue(habit, day) >= habit.target).length
    return { total: active.length, done, percent: active.length ? Math.round(done / active.length * 100) : null }
}
export function habitSeries(habits: Habit[], end: string, count: number) {
    const date = new Date(`${end}T12:00:00`)
    return Array.from({ length: count }, (_, index) => {
        const current = new Date(date); current.setDate(current.getDate() - count + index + 1)
        const day = dayKey(current)
        return { day, ...habitStats(habits, day) }
    })
}
