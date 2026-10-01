<script lang="ts">
    import { onMount } from "svelte"
    import { initTimer, tickTimer } from "@/lib/timer"
    onMount(() => {
        initTimer()
        const interval = setInterval(tickTimer, 250)
        const onFocus = () => tickTimer()
        window.addEventListener("focus", onFocus)
        const onVisibility = () => tickTimer()
        document.addEventListener("visibilitychange", onVisibility)
        return () => { clearInterval(interval); window.removeEventListener("focus", onFocus); document.removeEventListener("visibilitychange", onVisibility) }
    })
</script>
