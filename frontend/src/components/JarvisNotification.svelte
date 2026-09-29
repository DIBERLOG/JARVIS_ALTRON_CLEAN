<script lang="ts">
    import { fly, fade } from "svelte/transition"
    import { activeNotification, type JarvisNotification } from "@/lib/ipc"

    let notification: JarvisNotification | null = null
    let dismissTimer: ReturnType<typeof setTimeout> | undefined

    $: if ($activeNotification?.id !== notification?.id) {
        notification = $activeNotification
        if (dismissTimer) clearTimeout(dismissTimer)
        if (notification) {
            dismissTimer = setTimeout(() => {
                notification = null
                activeNotification.set(null)
            }, 7000)
        }
    }

    $: variant = notification?.title.toLowerCase().includes("погод") || notification?.title.toLowerCase().includes("weather")
        ? "weather"
        : notification?.title.toLowerCase().includes("счёт") || notification?.title.toLowerCase().includes("counter")
            ? "counter"
            : "system"
</script>

{#if notification}
    <section class="jarvis-notification-layer" class:weather={variant === "weather"} class:counter={variant === "counter"} aria-live="polite">
        <article class="jarvis-notification" in:fly={{ y: 260, duration: 620, opacity: 0 }} out:fade={{ duration: 180 }}>
            <img class="notification-logo" src="/media/128x128.png" alt="JARVIS" />
            <div class="notification-copy">
                <p class="notification-label">{notification.title}</p>
                <h2>{notification.primary}</h2>
                {#if notification.detail}<p class="notification-detail">{notification.detail}</p>{/if}
            </div>
            <span class="notification-pulse" aria-hidden="true"></span>
        </article>
    </section>
{/if}

<style lang="scss">
    :global(:root) { --notice-accent: #50f6e7; --notice-glow: rgba(80, 246, 231, .34); }
    .jarvis-notification-layer { position: fixed; inset: 0; z-index: 10000; display: grid; place-items: center; pointer-events: none; }
    .weather { --notice-accent: #66c8ff; --notice-glow: rgba(102, 200, 255, .34); }
    .counter { --notice-accent: #b58cff; --notice-glow: rgba(181, 140, 255, .34); }
    .jarvis-notification { width: min(520px, calc(100vw - 48px)); min-height: 132px; display: flex; align-items: center; gap: 18px; padding: 22px 54px 22px 24px; color: #e9ffff; background: linear-gradient(120deg, rgba(7,19,24,.97), rgba(8,31,38,.94)); border: 1px solid var(--notice-accent); border-radius: 18px 6px 18px 6px; box-shadow: 0 24px 72px rgba(0,0,0,.52), 0 0 42px var(--notice-glow); position: relative; overflow: hidden; }
    .jarvis-notification::before { content: ""; position: absolute; inset: 0 auto 0 0; width: 4px; background: var(--notice-accent); box-shadow: 0 0 18px var(--notice-accent); }
    .notification-logo { width: 58px; height: 58px; flex: 0 0 auto; object-fit: contain; border-radius: 50%; filter: drop-shadow(0 0 10px var(--notice-glow)); }
    .notification-copy { min-width: 0; }
    .notification-label, .notification-detail { margin: 0; font-family: "Roboto Condensed", "Trebuchet MS", sans-serif; }
    .notification-label { color: var(--notice-accent); font-size: 12px; font-weight: 700; letter-spacing: 2px; text-transform: uppercase; }
    h2 { margin: 5px 0 6px; font-family: "Roboto Condensed", "Trebuchet MS", sans-serif; font-size: clamp(20px, 3vw, 27px); font-weight: 700; line-height: 1.05; }
    .notification-detail { color: rgba(233,255,255,.72); font-size: 14px; line-height: 1.35; }
    .notification-pulse { position: absolute; right: 22px; top: 25px; width: 9px; height: 9px; border-radius: 50%; background: var(--notice-accent); box-shadow: 0 0 0 0 var(--notice-glow); animation: pulse 1.8s infinite; }
    @keyframes pulse { 70% { box-shadow: 0 0 0 12px transparent; } 100% { box-shadow: 0 0 0 0 transparent; } }
    @media (prefers-reduced-motion: reduce) { .notification-pulse { animation: none; } }
</style>
