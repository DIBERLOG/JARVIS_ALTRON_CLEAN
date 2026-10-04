// Run: node --conditions=browser tests/terminal.cjs <test dependency directory>
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict')
const {createRequire,Module}=require('node:module')
const deps=createRequire(path.resolve(process.argv[2],'package.json'))
const {JSDOM}=deps('jsdom')
const dom=new JSDOM('<!doctype html><body></body>',{url:'http://localhost'})
for(const name of ['window','document','navigator','HTMLElement','Node','Event','MouseEvent','getComputedStyle']) Object.defineProperty(globalThis,name,{value:dom.window[name],configurable:true})
const {screen}=deps('@testing-library/dom'),userEvent=deps('@testing-library/user-event').default
const ts=require('typescript'),{compile,preprocess}=require('svelte/compiler')
async function main(){
    const svelte=await import('svelte'),{writable}=await import('svelte/store')
    const filename=path.resolve('src/components/LiveTerminal.svelte')
    const processed=await preprocess(fs.readFileSync(filename,'utf8'),{script:({content,attributes})=>attributes.lang==='ts'?{code:ts.transpileModule(content,{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ESNext,verbatimModuleSyntax:true}}).outputText}:undefined},{filename})
    const compiled=compile(processed.code,{filename,generate:'dom',css:'external'})
    let calls=0,text='Распознана команда: погода',sent=[]
    const connected=writable(true)
    const overrides={svelte,'@tauri-apps/api/core':{invoke:async command=>{assert.equal(command,'get_jarvis_terminal_log');calls++;return text}},'@/stores':{ipcConnected:connected,sendTextCommand:async command=>{sent.push(command)}}}
    const mod=new Module(filename,module);mod.filename=filename;mod.paths=module.paths
    mod.require=name=>overrides[name]||require(name)
    mod._compile(ts.transpileModule(compiled.js.code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2020}}).outputText,filename)
    const component=new mod.exports.default({target:document.body})
    const user=userEvent.setup({document})
    try {
        assert.equal(calls,0)
        await user.click(screen.getByRole('button',{name:'Открыть терминал'}))
        await screen.findByText(text)
        text='Выполнена новая команда'
        await screen.findByText(text,{}, {timeout:2500})
        await user.type(screen.getByRole('textbox',{name:'Команда JARVIS'}),'какая погода')
        await user.click(screen.getByRole('button',{name:'Отправить'}))
        assert.deepEqual(sent,['какая погода'])
        assert.equal(screen.getByRole('textbox',{name:'Команда JARVIS'}).value,'')
        await user.type(screen.getByRole('textbox',{name:'Команда JARVIS'}),'черновик')
        await user.click(screen.getByRole('button',{name:'Скрыть терминал'}))
        assert.equal(screen.queryByRole('textbox',{name:'Команда JARVIS'}),null)
        const hiddenCalls=calls
        await new Promise(resolve=>setTimeout(resolve,1200))
        assert.equal(calls,hiddenCalls,'Hidden terminal must stop polling, not the assistant')
        text='Лог продолжался при скрытой панели'
        await user.click(screen.getByRole('button',{name:'Открыть терминал'}))
        await screen.findByText(text)
        assert.equal(screen.getByRole('textbox',{name:'Команда JARVIS'}).value,'черновик')
        connected.set(false)
        await user.click(screen.getByRole('button',{name:'Отправить'}))
        await screen.findByRole('alert')
        assert.equal(sent.length,1)
        console.log('PASS: live logs, command submission, hide/reopen, draft retention and disconnected module')
    } finally {component.$destroy()}
}
main().catch(error=>{console.error(error);process.exitCode=1})
