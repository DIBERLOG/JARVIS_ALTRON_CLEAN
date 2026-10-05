// node --conditions=browser tests/backup-ui.cjs <test dependency directory>
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createRequire,Module}=require('node:module')
const deps=createRequire(path.resolve(process.argv[2],'package.json')),{JSDOM}=deps('jsdom'),dom=new JSDOM('<body></body>',{url:'http://localhost'})
for(const name of ['window','document','navigator','HTMLElement','Node','Event','MouseEvent','getComputedStyle'])Object.defineProperty(globalThis,name,{value:dom.window[name],configurable:true})
const {screen}=deps('@testing-library/dom'),userEvent=deps('@testing-library/user-event').default,ts=require('typescript'),{compile,preprocess}=require('svelte/compiler')
async function main(){
 const svelte=await import('svelte'),store=await import('svelte/store'),writes=[];let restores=0,dialogPath='backup.json',valid=true
 const snapshot={city:'Москва',data:{notes:[],reminders:[],birthdays:[]}}
 const filename=path.resolve('src/components/CenterDataTransfer.svelte')
 const processed=await preprocess(fs.readFileSync(filename,'utf8'),{script:({content})=>({code:ts.transpileModule(content,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022,verbatimModuleSyntax:true}}).outputText})},{filename})
 const stubs={svelte,'@tauri-apps/plugin-dialog':{open:async()=> 'incoming.json',save:async()=>dialogPath},'@tauri-apps/plugin-fs':{readTextFile:async()=> '{}',writeTextFile:async(p,v)=>writes.push({p,v})},'@/lib/centerBackup':{createCenterBackup:async()=>snapshot,parseCenterBackup:()=>{if(!valid)throw new Error('Некорректный файл');return snapshot},restoreCenterBackup:async()=>restores++},'@/lib/centerVoice':{centerRevision:store.writable(0),resetCenterVoice(){}}}
 const empty=new Module(filename+'.stub',module);empty.paths=module.paths;empty._compile(ts.transpileModule(compile('<div></div>',{generate:'dom',css:'external'}).js.code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText,filename+'.stub');stubs['./VaultDataTransfer.svelte']=empty.exports
 const mod=new Module(filename,module);mod.paths=module.paths;mod.require=name=>stubs[name]||require(name)
 mod._compile(ts.transpileModule(compile(processed.code,{filename,generate:'dom',css:'external'}).js.code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText,filename)
 const component=new mod.exports.default({target:document.body}),user=userEvent.setup({document})
 await user.click(screen.getByRole('button',{name:/Экспортировать данные/}));await screen.findByText(/Данные экспортированы/);assert.equal(writes.length,1)
 await user.click(screen.getByRole('button',{name:/Импортировать файл/}));await screen.findByRole('region',{name:'Подтверждение импорта'});assert.equal(restores,0)
 await user.click(screen.getByRole('button',{name:'Отмена'}));assert.equal(screen.queryByRole('region',{name:'Подтверждение импорта'}),null);assert.equal(restores,0)
 await user.click(screen.getByRole('button',{name:/Импортировать файл/}));await screen.findByRole('region',{name:'Подтверждение импорта'})
 dialogPath=null;await user.click(screen.getByRole('button',{name:'Сохранить копию и заменить'}));await screen.findByText(/Импорт отменён/);assert.equal(restores,0)
 dialogPath='before.json';await user.click(screen.getByRole('button',{name:'Сохранить копию и заменить'}));await screen.findByText(/Данные восстановлены/);assert.equal(restores,1);assert.equal(writes.at(-1).p,'before.json')
 valid=false;await user.click(screen.getByRole('button',{name:/Импортировать файл/}));await screen.findByRole('alert');assert.equal(restores,1)
 component.$destroy();console.log('Backup UI: export, preview, cancellation, mandatory rollback copy and invalid-file rejection passed')
}
main().then(()=>process.exit(0)).catch(e=>{console.error(e);process.exit(1)})
