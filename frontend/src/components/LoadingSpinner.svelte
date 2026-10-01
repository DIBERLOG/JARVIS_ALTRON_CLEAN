<script lang="ts">
    export let size = 16
</script>

<span class="loading-spinner" style:width={`${size}px`} style:height={`${size}px`} aria-hidden="true"></span>

<style>
    .loading-spinner{display:inline-block;flex:none;box-sizing:border-box;border:2px solid #52fefe33;border-top-color:#70fff3;border-right-color:#70fff3;border-radius:50%;animation:loading-spin .85s linear infinite;box-shadow:0 0 10px #52fefe12}
    @keyframes loading-spin{to{transform:rotate(360deg)}}
    @media(prefers-reduced-motion:reduce){.loading-spinner{animation:none}}
</style>
