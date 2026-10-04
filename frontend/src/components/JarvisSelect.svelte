<script lang="ts">
    import { createEventDispatcher, onMount } from 'svelte'
    import { fly } from 'svelte/transition'
    export let value: string | number
    export let options: {value:string|number;label:string;icon?:string;detail?:string}[]=[]
    export let label=''
    export let disabled=false
    const dispatch=createEventDispatcher<{change:string|number}>()
    let root:HTMLDivElement,trigger:HTMLButtonElement,opened=false,active=0,left=0,top=0,width=240,maxHeight=320
    const id='jarvis-select-'+Math.random().toString(36).slice(2)
    $: chosen=options.find(option=>option.value===value)
    $: if(disabled)opened=false
    function show(){
        if(disabled)return
        const rect=trigger.getBoundingClientRect()
        width=Math.min(Math.max(rect.width,260),window.innerWidth-24)
        left=Math.max(12,Math.min(rect.left,window.innerWidth-width-12))
        const below=window.innerHeight-rect.bottom-16,above=rect.top-16
        const height=Math.min(320,options.length*58+16)
        const useAbove=below<height&&above>below
        maxHeight=Math.max(80,Math.min(height,useAbove?above:below))
        top=useAbove?Math.max(12,rect.top-maxHeight-8):rect.bottom+8
        active=Math.max(0,options.findIndex(option=>option.value===value));opened=true
    }
    function choose(index:number){value=options[index].value;opened=false;dispatch('change',value);trigger.focus()}
    function keydown(event:KeyboardEvent){
        if(['ArrowDown','ArrowUp','Home','End','Escape','Enter',' '].includes(event.key)){
            if(event.key==='Escape'){opened=false;return}
            event.preventDefault()
            if(!opened){show();return}
            if(event.key==='Enter'||event.key===' '){choose(active);return}
            active=event.key==='Home'?0:event.key==='End'?options.length-1:(active+(event.key==='ArrowDown'?1:-1)+options.length)%options.length
            document.getElementById(id+'-'+active)?.scrollIntoView({block:'nearest'})
        }
        if(event.key==='Tab')opened=false
    }
    onMount(()=>{
        const outside=(event:PointerEvent)=>{if(!root.contains(event.target as Node))opened=false}
        const close=()=>opened=false
        const scroll=(event:Event)=>{if(!(event.target instanceof Node)||!root.contains(event.target))close()}
        window.addEventListener('pointerdown',outside);window.addEventListener('resize',close);window.addEventListener('scroll',scroll,true)
        return()=>{window.removeEventListener('pointerdown',outside);window.removeEventListener('resize',close);window.removeEventListener('scroll',scroll,true)}
    })
</script>

<div class="jarvis-select" bind:this={root}>
    {#if label}<span class="field-label" id={id+'-label'}>{label}</span>{/if}
    <button class="select-trigger" class:opened bind:this={trigger} {disabled} role="combobox" aria-label={label} aria-expanded={opened} aria-controls={id} aria-haspopup="listbox" aria-activedescendant={opened?id+'-'+active:undefined} on:keydown={keydown} on:click={()=>opened?opened=false:show()}>
        {#if chosen?.icon}<span class="option-icon">{chosen.icon}</span>{/if}
        <span class="selection">{chosen?.label||'Выберите'}</span>
        <svg class:rotated={opened} viewBox="0 0 20 20" fill="none" aria-hidden="true"><path d="m5 7 5 5 5-5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
    {#if opened}
        <div class="select-menu" role="listbox" {id} aria-label={label} style:left={left+'px'} style:top={top+'px'} style:width={width+'px'} style:max-height={maxHeight+'px'} transition:fly={{y:6,duration:160}}>
            {#each options as option,index}
                <button type="button" role="option" id={id+'-'+index} aria-selected={option.value===value} tabindex="-1" class:chosen={option.value===value} class:active={active===index} on:pointerenter={()=>active=index} on:click={()=>choose(index)}>
                    {#if option.icon}<span class="option-icon">{option.icon}</span>{/if}
                    <span class="option-copy"><strong>{option.label}</strong>{#if option.detail}<small>{option.detail}</small>{/if}</span>
                    <span class="check" aria-hidden="true">{option.value===value?'✓':''}</span>
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
    .jarvis-select{display:flex;flex-direction:column;gap:.35rem;min-width:0;font-family:'Manrope Variable',sans-serif}
    .field-label{color:#91abb1;font-size:.62rem;padding-left:.2rem}
    .select-trigger{display:flex;align-items:center;gap:.65rem;min-height:42px;width:100%;padding:.6rem .85rem;border:1px solid #31565c;border-radius:14px;background:linear-gradient(135deg,#17383d,#10262c);color:#e4fcfc;font:600 .7rem 'Manrope Variable',sans-serif;cursor:pointer;box-shadow:inset 0 1px #ffffff06;transition:border-color .18s,box-shadow .18s,background .18s}
    .select-trigger:hover,.select-trigger.opened{border-color:#58d9d2;box-shadow:0 0 0 3px #52e7dd12}
    .selection{flex:1;text-align:left;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
    svg{width:16px;height:16px;color:#79b2b7;flex:none;transition:transform .2s}.rotated{transform:rotate(180deg)}
    .select-trigger:disabled{opacity:.5;cursor:wait}
    .select-menu{position:fixed;z-index:10000;box-sizing:border-box;overflow-y:auto;padding:7px;border:1px solid #3b7378;border-radius:18px;background:linear-gradient(145deg,#153239fa,#091c22fc);box-shadow:0 16px 45px #0008,inset 0 1px #a1fff416;backdrop-filter:blur(20px);scrollbar-width:thin;scrollbar-color:#3b7278 transparent}
    .select-menu button{display:flex;align-items:center;gap:.65rem;width:100%;min-height:42px;padding:.6rem .7rem;border:1px solid transparent;border-radius:11px;background:transparent;color:#b8d9de;text-align:left;cursor:pointer;transition:background .14s,border-color .14s,color .14s}
    .select-menu button.active{background:#2b59644d;color:#f1ffff}
    .select-menu button.chosen{background:linear-gradient(100deg,#1e74765c,#17484f70);border-color:#51e4db45;color:#a0fff1}
    .option-icon{display:grid;place-items:center;flex:none;width:25px;height:25px;border-radius:8px;background:#67eee312;color:#7bf7e8;font-size:.85rem}
    .option-copy{flex:1;min-width:0}.option-copy strong{display:block;font:650 .7rem 'Manrope Variable',sans-serif}.option-copy small{display:block;margin-top:.18rem;font:500 .59rem 'Manrope Variable',sans-serif;color:#81a8b1}
    .check{width:18px;color:#6effe8;font-size:.85rem}button:focus-visible{outline:2px solid #76f5e5;outline-offset:3px}
    @media(prefers-reduced-motion:reduce){button,svg{transition:none}}
</style>
