// Run: node --conditions=browser tests/route-connection.cjs <test dependency directory>
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict')
const {createRequire,Module}=require('node:module')
const deps=createRequire(path.resolve(process.argv[2],'package.json'))
const {JSDOM}=deps('jsdom')
const dom=new JSDOM('<!doctype html><body></body>',{url:'http://localhost'})
for(const name of ['window','document','navigator','HTMLElement','Node','Event','MouseEvent','getComputedStyle'])Object.defineProperty(globalThis,name,{value:dom.window[name],configurable:true})
const {screen}=deps('@testing-library/dom'),userEvent=deps('@testing-library/user-event').default
const ts=require('typescript'),{compile,preprocess}=require('svelte/compiler')
async function main(){
    const svelte=await import('svelte'),stores=await import('svelte/store')
    const running=stores.writable(true),connected=stores.writable(true)
    let disconnections=0
    const appStores={isJarvisRunning:running,ipcConnected:connected,translations:stores.writable({}),translate:(_,key)=>key,updateJarvisStats:async()=>{},enableIpc:()=>connected.set(true),disableIpc:()=>{disconnections++;connected.set(false)}}
    async function component(source,filename,overrides={}){
        const processed=await preprocess(source,{script:({content,attributes})=>attributes.lang==='ts'?{code:ts.transpileModule(content,{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ESNext,verbatimModuleSyntax:true}}).outputText}:undefined},{filename})
        const mod=new Module(filename,module);mod.filename=filename;mod.paths=module.paths
        mod.require=name=>({svelte,'@/stores':appStores,'@tauri-apps/api/core':{invoke:async()=>{}},...overrides})[name]||require(name)
        mod._compile(ts.transpileModule(compile(processed.code,{filename,generate:'dom',css:'external'}).js.code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2020}}).outputText,filename)
        return {default:mod.exports.default}
    }
    const blank=await component('<span></span>',path.resolve('tests/blank.svelte'))
    const home=await component(fs.readFileSync('src/routes/index.svelte','utf8'),path.resolve('src/routes/index.svelte'),{'@/components/elements/SearchBar.svelte':blank,'@/components/elements/ArcReactor.svelte':blank,'@/components/elements/HDivider.svelte':blank,'@/components/elements/Stats.svelte':blank,'@/components/Footer.svelte':blank})
    const host=await component('<script>import Home from "./Home.svelte";import {ipcConnected} from "@/stores";let center=false</script><p role="status">{$ipcConnected?"Связь с JARVIS активна":"Нет связи с JARVIS"}</p><button on:click={()=>center=!center}>{center?"Главная":"Центр"}</button>{#if center}<h2>Центр</h2>{:else}<Home/>{/if}',path.resolve('tests/route-host.svelte'),{'./Home.svelte':home})
    const app=new host.default({target:document.body}),user=userEvent.setup({document})
    assert.ok(screen.getByRole('status').textContent.includes('активна'))
    await user.click(screen.getByRole('button',{name:'Центр',exact:true}))
    assert.ok(screen.getByRole('heading',{name:'Центр'}))
    assert.equal(screen.getByRole('status').textContent,'Связь с JARVIS активна','Leaving the home page must not disconnect the application')
    await user.click(screen.getByRole('button',{name:'Главная',exact:true}));await user.click(screen.getByRole('button',{name:'Центр',exact:true}))
    running.set(false);await svelte.tick()
    assert.equal(disconnections,0,'Unmounted home pages must not retain IPC subscriptions')
    app.$destroy()
    console.log('PASS: navigating away from home preserves IPC; repeated navigation does not leak connection subscriptions')
}
main().catch(error=>{console.error(error);process.exitCode=1})
