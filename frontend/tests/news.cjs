// Run: node --conditions=browser tests/news.cjs <test dependency directory>
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict')
const {createRequire,Module}=require('node:module')
const deps=createRequire(path.resolve(process.argv[2],'package.json'))
const {JSDOM}=deps('jsdom')
const dom=new JSDOM('<!doctype html><body></body>',{url:'http://localhost'})
for(const name of ['window','document','navigator','HTMLElement','Node','Event','MouseEvent','getComputedStyle','localStorage']) Object.defineProperty(globalThis,name,{value:dom.window[name],configurable:true})
const {screen}=deps('@testing-library/dom'),userEvent=deps('@testing-library/user-event').default
const ts=require('typescript'),{compile,preprocess}=require('svelte/compiler')
async function main(){
    const svelte=await import('svelte')
    const filename=path.resolve('src/components/CenterNews.svelte')
    const processed=await preprocess(fs.readFileSync(filename,'utf8'),{script:({content,attributes})=>attributes.lang==='ts'?{code:ts.transpileModule(content,{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ESNext}}).outputText}:undefined},{filename})
    const compiled=compile(processed.code,{filename,generate:'dom',css:'external'})
    let newsRequests=0,translationRequests=0,resolveTranslation
    const article={source:'BBC News',url:'https://example.com/news',title:'Original headline',summary:'Original description',published_at:'2026-10-01T10:00:00Z'}
    const invoke=async command=>{
        if(command==='center_get_news'){newsRequests++;return [{...article}]}
        if(command==='center_translate_news'){translationRequests++;return new Promise(resolve=>{resolveTranslation=resolve})}
        throw Error('Unexpected command: '+command)
    }
    const mod=new Module(filename,module);mod.filename=filename;mod.paths=module.paths
    const overrides={svelte,'@tauri-apps/api/core':{invoke},'@tauri-apps/plugin-shell':{open:async()=>{throw Error('Translation must not open browser')}}}
    mod.require=name=>overrides[name]||require(name)
    mod._compile(ts.transpileModule(compiled.js.code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2020}}).outputText,filename)
    const component=new mod.exports.default({target:document.body})
    const user=userEvent.setup({document})
    await screen.findByRole('heading',{name:'Original headline'})
    await user.click(screen.getByRole('button',{name:'Перевести на русский'}))
    assert.ok(screen.getByRole('button',{name:/Перевожу/}).disabled)
    resolveTranslation({title:'Русский заголовок',summary:'Русское описание'})
    await screen.findByRole('heading',{name:'Русский заголовок'})
    assert.ok(screen.getByText('Русское описание'))
    assert.equal(screen.queryByRole('heading',{name:'Original headline'}),null)
    await user.click(screen.getByRole('button',{name:'Показать оригинал'}))
    await screen.findByRole('heading',{name:'Original headline'})
    assert.ok(screen.getByText('Original description'))
    assert.equal(screen.queryByText('Русское описание'),null)
    await user.click(screen.getByRole('button',{name:'Показать перевод'}))
    await screen.findByRole('heading',{name:'Русский заголовок'})
    assert.equal(translationRequests,1)
    assert.equal(newsRequests,1,'Switching language must not refresh the feed')
    component.$destroy()
    console.log('PASS: translation renders immediately; original and cached translation toggle without refreshing news')
}
main().catch(error=>{console.error(error);process.exitCode=1})
