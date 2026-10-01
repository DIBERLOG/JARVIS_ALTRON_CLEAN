<script lang="ts">
    import { onMount } from "svelte"
    import { activeNotification } from "@/lib/ipc"
    import { birthdayDate, dayKey, loadCenterData } from "@/lib/center"

    onMount(() => {
        const shown = new Set<string>()
        let busy = false
        async function check() {
            if (busy) return
            busy = true
            try {
                const data = await loadCenterData()
                const now = new Date()
                const due = data.reminders
                    .filter(item => !item.done && new Date(item.dueAt).getTime() <= now.getTime() && !shown.has(item.id))
                    .sort((a, b) => b.dueAt.localeCompare(a.dueAt))[0]
                if (due) {
                    shown.add(due.id)
                    activeNotification.set({ id: Date.now(), title: "Напоминание", primary: due.title, detail: new Date(due.dueAt).toLocaleString("ru-RU"), })
                    return
                }
                const birthday = data.birthdays.find(item => dayKey(birthdayDate(item, now.getFullYear())) === dayKey(now) && !shown.has(`birthday-${item.id}-${dayKey(now)}`))
                if (birthday) {
                    shown.add(`birthday-${birthday.id}-${dayKey(now)}`)
                    activeNotification.set({ id: Date.now(), title: "День рождения", primary: birthday.name, detail: "Сегодня важная дата", })
                }
            } catch (error) {
                console.error("Не удалось проверить напоминания Центра", error)
            } finally { busy = false }
        }
        check()
        const timer = window.setInterval(check, 45_000)
        return () => window.clearInterval(timer)
    })
</script>
