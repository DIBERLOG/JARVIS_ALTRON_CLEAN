<script lang="ts">
    import { weatherKind } from "@/lib/weather"
    export let code: number
    export let id = "weather"
    $: kind = weatherKind(code)
</script>

<svg viewBox="0 0 64 64" fill="none" aria-hidden="true">
    <defs>
        <linearGradient id={`${id}-sun`} x1="18" y1="9" x2="45" y2="45" gradientUnits="userSpaceOnUse"><stop stop-color="#ffed93"/><stop offset="1" stop-color="#ffb328"/></linearGradient>
        <linearGradient id={`${id}-cloud`} x1="24" y1="22" x2="36" y2="49" gradientUnits="userSpaceOnUse"><stop stop-color="#e9f6ff"/><stop offset="1" stop-color="#779cac"/></linearGradient>
    </defs>
    {#if kind === "sun" || kind === "partly"}
        <g stroke="#ffd267" stroke-width="2.6" stroke-linecap="round">
            <path d="M32 4v5m0 41v5M6 30h5m42 0h5M13 11l4 4m30 30 4 4M13 49l4-4m30-30 4-4"/>
            <circle cx="32" cy="30" r="15" fill={`url(#${id}-sun)`} stroke="#ffe393"/>
        </g>
    {/if}
    {#if kind !== "sun" && kind !== "unknown"}
        <path d="M14 45a9 9 0 0 1-1-18 14 14 0 0 1 27-3 11 11 0 1 1 7 21H14Z" fill={`url(#${id}-cloud)`} stroke="#b7d6e3" stroke-width="1.2"/>
    {/if}
    {#if kind === "rain"}<path d="m20 50-3 6m14-6-3 6m14-6-3 6" stroke="#51c7ff" stroke-width="3" stroke-linecap="round"/>{/if}
    {#if kind === "snow"}<g stroke="#b5efff" stroke-width="1.8" stroke-linecap="round"><path d="M20 50v10m-4-8 8 6m-8 0 8-6M43 50v10m-4-8 8 6m-8 0 8-6"/></g>{/if}
    {#if kind === "storm"}<path d="m32 42-7 11h7l-3 9 12-15h-9l4-5" fill="#ffdc76"/>{/if}
    {#if kind === "fog"}<path d="M10 50h44M17 57h33" stroke="#8eafbf" stroke-width="2.5" stroke-linecap="round"/>{/if}
    {#if kind === "unknown"}<circle cx="32" cy="32" r="22" stroke="#8eafbf" stroke-width="2"/><text x="32" y="41" text-anchor="middle" fill="#8eafbf" font-size="28">?</text>{/if}
</svg>
