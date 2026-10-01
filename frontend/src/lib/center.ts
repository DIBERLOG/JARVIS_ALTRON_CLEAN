import { invoke } from "@tauri-apps/api/core"
import type { TrainingData } from "./training"

export type Reminder = { id: string, title: string, dueAt: string, done: boolean }
export type Birthday = { id: string, name: string, month: number, day: number, year?: number }
export type ChecklistItem = { id: string, text: string, done: boolean }
export type Note = { id: string, title: string, text: string, updatedAt: string, type?: "text" | "checklist", format?: "plain" | "markdown" | "rich", html?: string, items?: ChecklistItem[] }
export type Habit = { id: string; title: string; caption: string; icon: string; target: number; unit: string; createdOn: string; archivedOn?: string; entries: Record<string, number> }
export type CenterData = { reminders: Reminder[], birthdays: Birthday[], notes: Note[], habits: Habit[], training?: TrainingData }

export function defaultHabits(): Habit[] {
    return [
        ["Сходил в зал", "Движение и сила", "🏋", 1, "раз"],
        ["Сделал зарядку", "5–10 минут для бодрого дня", "⚡", 1, "раз"],
        ["Позавтракал", "Спокойное начало дня", "🍳", 1, "раз"],
        ["Выпил воду", "Отмечайте каждый стакан", "💧", 8, "стаканов"],
        ["Прогулка", "30 минут на свежем воздухе", "🌿", 1, "раз"],
        ["Чтение 20 минут", "Немного времени для книги", "📖", 1, "раз"],
        ["Лёг спать вовремя", "Ваша цель: до 23:00", "☾", 1, "раз"],
    ].map(([title, caption, icon, target, unit]) => ({ id: makeId(), title: String(title), caption: String(caption), icon: String(icon), target: Number(target), unit: String(unit), createdOn: dayKey(new Date()), entries: {} }))
}

export const emptyCenterData = (): CenterData => ({ reminders: [], birthdays: [], notes: [], habits: [] })

export async function loadCenterData(): Promise<CenterData> {
    const raw = await invoke<string>("db_read", { key: "center_data" })
    if (!raw) return { ...emptyCenterData(), habits: defaultHabits() }
    const parsed = JSON.parse(raw)
    if (!parsed || !Array.isArray(parsed.reminders) || !Array.isArray(parsed.birthdays)) {
        throw new Error("Данные Центра повреждены")
    }
    return { reminders: parsed.reminders, birthdays: parsed.birthdays, notes: Array.isArray(parsed.notes) ? parsed.notes : [], habits: Array.isArray(parsed.habits) ? parsed.habits : defaultHabits(), training: parsed.training } as CenterData
}

export async function saveCenterData(data: CenterData): Promise<void> {
    const saved = await invoke<boolean>("db_write", { key: "center_data", val: JSON.stringify(data) })
    if (!saved) throw new Error("Не удалось сохранить данные Центра")
}

export function dayKey(date: Date): string {
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`
}

export function birthdayDate(birthday: Birthday, year: number): Date {
    const date = new Date(year, birthday.month - 1, birthday.day)
    if (birthday.month === 2 && birthday.day === 29 && date.getMonth() !== 1) return new Date(year, 2, 1)
    return date
}

export function nextBirthday(birthday: Birthday, now = new Date()): Date {
    let date = birthdayDate(birthday, now.getFullYear())
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
    if (date < today) date = birthdayDate(birthday, now.getFullYear() + 1)
    return date
}

export function makeId(): string {
    return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`
}
