<script lang="ts">
    import { onMount } from 'svelte'
    import { goto } from '@roxi/routify'
    import { handleCenterVoice, centerVoiceError } from '@/lib/centerVoice'
    import { sendAction } from '@/lib/ipc'
    let navigate: (path: string) => void
    $: navigate = $goto
    onMount(() => {
        let queue = Promise.resolve()
        const open = () => navigate('/center')
        const command = (event: Event) => {
            const text = (event as CustomEvent<string>).detail
            queue = queue.then(() => handleCenterVoice(text)).catch(error => {
                centerVoiceError(error)
            })
        }
        window.addEventListener('jarvis-center-open',open)
        window.addEventListener('jarvis-center-command',command)
        return () => {window.removeEventListener('jarvis-center-open',open);window.removeEventListener('jarvis-center-command',command)}
    })
</script>
