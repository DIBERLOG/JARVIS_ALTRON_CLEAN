import { writable, get } from "svelte/store"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { ipcConnected, microphoneMuteKnown, microphoneMuted, setMicrophoneMuted, activeNotification } from "./ipc"

const storageKey = "jarvis-listening-shortcut"
export const listeningShortcut = writable(localStorage.getItem(storageKey) ?? "Ctrl+Alt+J")
export const shortcutError = writable("")
export const recordingShortcut = writable(false)

export async function applyListeningShortcut(shortcut: string) {
    try {
        await invoke("set_listening_shortcut", { shortcut })
        localStorage.setItem(storageKey, shortcut)
        listeningShortcut.set(shortcut)
        shortcutError.set("")
    } catch (error) {
        shortcutError.set(String(error))
        throw error
    }
}

export async function initListeningShortcut() {
    const stop = await listen("toggle-listening-shortcut", () => {
        if (get(recordingShortcut)) return
        if (get(ipcConnected) && get(microphoneMuteKnown)) {
            setMicrophoneMuted(!get(microphoneMuted))
        } else {
            activeNotification.set({ title: "МИКРОФОН JARVIS", primary: "Голосовой модуль не подключён", detail: "Запустите JARVIS и дождитесь подключения.", id: Date.now() })
        }
    })
    try { await applyListeningShortcut(get(listeningShortcut)) } catch { /* Shown in settings. */ }
    return stop
}
