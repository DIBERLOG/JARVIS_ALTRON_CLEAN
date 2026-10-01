import { get, writable } from "svelte/store"
import { activeNotification } from "@/lib/ipc"

export type TimerPreset = { id: string; label: string; seconds: number; icon: string; owner?: string }
export type TimerState = { label: string; total: number; remaining: number; deadline: number | null; phase: "idle" | "running" | "paused" | "finished"; owner?: string }
export const timerState = writable<TimerState>({ label: "5 минут", total: 300, remaining: 300, deadline: null, phase: "idle" })
export const timerSound = writable(true)
export const customPresets = writable<TimerPreset[]>([])
export const timerPresets: TimerPreset[] = [
    { id: "5", label: "5 минут", seconds: 300, icon: "ϟ" },
    { id: "10", label: "10 минут", seconds: 600, icon: "☕" },
    { id: "15", label: "15 минут", seconds: 900, icon: "▣" },
    { id: "30", label: "30 минут", seconds: 1800, icon: "◎" },
    { id: "60", label: "1 час", seconds: 3600, icon: "◌" },
    { id: "120", label: "2 часа", seconds: 7200, icon: "↔" },
    { id: "study", label: "Учёба", seconds: 1500, icon: "▤" },
    { id: "work", label: "Работа", seconds: 3000, icon: "▣" },
    { id: "break", label: "Перерыв", seconds: 600, icon: "☕" },
    { id: "games", label: "Игры", seconds: 3600, icon: "✚" },
    { id: "gym", label: "Тренировка", seconds: 2700, icon: "♡" },
    { id: "sleep", label: "Сон", seconds: 7200, icon: "☾" },
]
let audio: AudioContext | null = null
const key = "jarvis-center-timer"
function persist() {
    try { localStorage.setItem(key, JSON.stringify({ timer: get(timerState), sound: get(timerSound), presets: get(customPresets) })) }
    catch (e) { console.error("Не удалось сохранить таймер", e) }
}
export function initTimer() {
    try {
        const saved = JSON.parse(localStorage.getItem(key) || "null")
        const state = saved?.timer
        if (state && ["idle", "running", "paused", "finished"].includes(state.phase) && Number.isFinite(state.total) && state.total > 0 && state.total <= 86400 && Number.isFinite(state.remaining) && state.remaining >= 0 && state.remaining <= state.total && typeof state.label === "string" && (state.phase !== "running" || Number.isFinite(state.deadline))) timerState.set(state)
        if (typeof saved?.sound === "boolean") timerSound.set(saved.sound)
        if (Array.isArray(saved?.presets)) customPresets.set(saved.presets.filter((preset: TimerPreset) => typeof preset.id === "string" && typeof preset.label === "string" && Number.isInteger(preset.seconds) && preset.seconds > 0 && preset.seconds <= 86400).slice(0, 20))
    } catch (e) { console.error("Не удалось восстановить таймер", e) }
    tickTimer()
}
export function tickTimer(now = Date.now()) {
    const state = get(timerState)
    if (state.phase !== "running" || state.deadline === null) return
    const remaining = Math.max(0, Math.ceil((state.deadline - now) / 1000))
    if (remaining > 0) { if (remaining !== state.remaining) timerState.set({ ...state, remaining }); return }
    timerState.set({ ...state, remaining: 0, deadline: null, phase: "finished" })
    persist()
    activeNotification.set({ id: Date.now(), title: "Таймер завершён", primary: state.label, detail: "Время вышло, сэр." })
    if (get(timerSound)) playAlarm()
}
function prepareAudio() {
    try { audio ??= new AudioContext(); void audio.resume().catch(() => {}) } catch { /* Visual notification remains available. */ }
}
function playAlarm() {
    if (!audio || audio.state !== "running") return
    for (let i = 0; i < 3; i++) {
        const start = audio.currentTime + i * .35
        const oscillator = audio.createOscillator(), gain = audio.createGain()
        oscillator.frequency.value = i === 1 ? 880 : 660
        gain.gain.setValueAtTime(0, start); gain.gain.linearRampToValueAtTime(.12, start + .025); gain.gain.exponentialRampToValueAtTime(.001, start + .28)
        oscillator.connect(gain); gain.connect(audio.destination)
        oscillator.start(start); oscillator.stop(start + .3)
        oscillator.onended = () => { oscillator.disconnect(); gain.disconnect() }
    }
}
export function selectTimer(preset: TimerPreset) {
    timerState.set({ label: preset.label, total: preset.seconds, remaining: preset.seconds, deadline: null, phase: "idle", owner: preset.owner }); persist()
}
export function startTimer() {
    const state = get(timerState)
    if (state.phase === "running") return
    prepareAudio()
    const remaining = state.remaining || state.total
    timerState.set({ ...state, remaining, deadline: Date.now() + remaining * 1000, phase: "running" }); persist()
}
export function pauseTimer() {
    tickTimer()
    const state = get(timerState)
    if (state.phase === "running") { timerState.set({ ...state, deadline: null, phase: "paused" }); persist() }
}
export function resetTimer() {
    const state = get(timerState)
    timerState.set({ ...state, remaining: state.total, deadline: null, phase: "idle" }); persist()
}
export function setTimerSound(enabled: boolean) { timerSound.set(enabled); persist() }
export function addTimerPreset(label: string, seconds: number) {
    if (!label.trim() || !Number.isInteger(seconds) || seconds < 1 || seconds > 86400 || get(customPresets).length >= 20) return false
    const preset = { id: crypto.randomUUID(), label: label.trim().slice(0, 40), seconds, icon: "◷" }
    customPresets.update(presets => [...presets, preset]); selectTimer(preset); return true
}
export function removeTimerPreset(id: string) { customPresets.update(presets => presets.filter(preset => preset.id !== id)); persist() }
export function formatTimer(seconds: number, full = false) {
    const hours = Math.floor(seconds / 3600), minutes = Math.floor(seconds % 3600 / 60), rest = seconds % 60
    const pair = (value: number) => String(value).padStart(2, "0")
    return `${hours || full ? `${pair(hours)}:` : ""}${pair(minutes)}:${pair(rest)}`
}
