<script lang="ts">
    import { Container } from "@svelteuidev/core"
    import Header from "@/components/Header.svelte"
    import CenterVoiceController from '@/components/CenterVoiceController.svelte'
</script>

<Container fluid id="wrapper">
    <Header />
    <CenterVoiceController />
    <main>
        <slot></slot>
    </main>
</Container>
