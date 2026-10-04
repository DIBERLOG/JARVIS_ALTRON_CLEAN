<script lang="ts">
    export let showFork = true
    export let showOriginal = false
    import { currentLanguage } from "@/stores"

    const forkGithub = "https://github.com/DIBERLOG"
    const originalGithub = "https://github.com/Priler/jarvis"
    const originalTelegram = "https://t.me/howdyho_official"
    const originalBoosty = "https://boosty.to/howdyho"

    $: isRussian = $currentLanguage === "ru"
</script>

<footer class="credits">
    {#if showFork}
    <div class="fork-credit">
        <span class="credit-marker" aria-hidden="true"></span>
        <div>
            <span class="credit-label">{isRussian ? "АВТОР ФОРКА" : "FORK AUTHOR"}</span>
            <strong>Егор Ангелов</strong>
        </div>
        <a href={forkGithub} target="_blank" rel="noopener noreferrer" aria-label="GitHub Егора Ангелова">
            GitHub <span aria-hidden="true">↗</span>
        </a>
    </div>

    {/if}
    {#if showOriginal}
    <details class="original-credits">
        <summary>
            <span>{isRussian ? "Первоначальный автор и ссылки проекта" : "Original author and project links"}</span>
            <span class="chevron" aria-hidden="true"></span>
        </summary>
        <div class="original-content">
            <p>© 2026. {isRussian ? "Первоначальный автор проекта:" : "Original project author:"} <strong>Abraham Tugalov</strong></p>
            <div class="original-links">
                <a href={originalTelegram} target="_blank" rel="noopener noreferrer">{isRussian ? "Наш телеграм канал" : "Telegram channel"}</a>
                <a href={originalGithub} target="_blank" rel="noopener noreferrer">{isRussian ? "GitHub репозиторий проекта" : "Original GitHub repository"}</a>
                <a href={originalBoosty} target="_blank" rel="noopener noreferrer">{isRussian ? "Поддержать проект на Boosty" : "Support on Boosty"}</a>
            </div>
        </div>
    </details>
    {/if}
</footer>

<style lang="scss">
    .credits {
        width: min(100%, 520px);
        margin: 42px auto 16px;
        color: #abc1c5;
        font-family: "Roboto Condensed", sans-serif;
    }

    .fork-credit {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 13px 16px;
        border: 1px solid rgba(82, 254, 254, 0.18);
        border-radius: 8px;
        background: linear-gradient(110deg, rgba(17, 60, 68, 0.54), rgba(12, 22, 29, 0.82) 68%);
        box-shadow: inset 0 1px 0 rgba(82, 254, 254, 0.06);
    }

    .credit-marker {
        width: 7px;
        height: 26px;
        border-radius: 6px;
        background: #52fefe;
        box-shadow: 0 0 15px rgba(82, 254, 254, 0.56);
        flex: none;
    }

    .fork-credit div { display: grid; gap: 2px; }
    .credit-label { color: #6fa3ab; font-size: 10px; font-weight: 700; letter-spacing: .14em; }
    .fork-credit strong { color: #f1fcfc; font-size: 16px; letter-spacing: .025em; }

    .fork-credit a {
        margin-left: auto;
        padding: 7px 10px;
        border: 1px solid rgba(82, 254, 254, 0.35);
        border-radius: 5px;
        color: #82f7f7;
        font-size: 12px;
        text-decoration: none;
        white-space: nowrap;
        transition: background .2s ease, border-color .2s ease;
    }
    .fork-credit a:hover { background: rgba(82, 254, 254, 0.12); border-color: #52fefe; }

    .original-credits {
        margin-top: 9px;
        border: 1px solid rgba(153, 189, 195, 0.12);
        border-radius: 7px;
        background: rgba(6, 16, 22, 0.58);
    }

    summary {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        padding: 10px 15px;
        color: #92adb2;
        font-size: 12px;
        letter-spacing: .02em;
        cursor: pointer;
        list-style: none;
    }
    summary::-webkit-details-marker { display: none; }
    summary:hover { color: #d4f7f7; }
    summary:focus-visible, a:focus-visible { outline: 2px solid #52fefe; outline-offset: 2px; }
    .chevron { width: 8px; height: 8px; border-right: 1.5px solid currentColor; border-bottom: 1.5px solid currentColor; transform: rotate(45deg); transition: transform .2s ease; }
    details[open] .chevron { transform: rotate(225deg); }

    .original-content { padding: 2px 15px 14px; border-top: 1px solid rgba(153, 189, 195, 0.1); }
    .original-content p { margin: 10px 0; color: #8fa5aa; font-size: 12px; }
    .original-content strong { color: #dcebec; }
    .original-links { display: flex; flex-wrap: wrap; gap: 8px 16px; }
    .original-links a { color: #67bac7; font-size: 12px; text-decoration: underline; text-underline-offset: 3px; }
    .original-links a:hover { color: #9cfafa; }

    @media (max-width: 800px) {
        .credits { margin-top: 36px; }
    }
</style>
