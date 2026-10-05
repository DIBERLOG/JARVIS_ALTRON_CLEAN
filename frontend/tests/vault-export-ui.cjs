// node --conditions=browser tests/vault-export-ui.cjs <test dependency directory>
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{createRequire,Module}=require('node:module')
const deps=createRequire(path.resolve(process.argv[2],'package.json')),{JSDOM}=deps('jsdom'),dom=new JSDOM('<body></body>',{url:'http://localhost'})
for(const name of ['window','document','navigator','HTMLElement','Node','Event','MouseEvent','getComputedStyle'])Object.defineProperty(globalThis,name,{value:dom.window[name],configurable:true})
const {screen}=deps('@testing-library/dom'),userEvent=deps('@testing-library/user-event').default,ts=require('typescript'),{compile,preprocess}=require('svelte/compiler')
async function main(){
 const svelte=await import('svelte'),writes=[];let dialogs=0,checks=0
 const filename=path.resolve('src/components/VaultDataTransfer.svelte'),encrypted={version:1,ciphertext:'encrypted-only'}
 const processed=await preprocess(fs.readFileSync(filename,'utf8'),{script:({content})=>({code:ts.transpileModule(content,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022,verbatimModuleSyntax:true}}).outputText})},{filename})
 const stubs={svelte,'@tauri-apps/api/core':{invoke:async(name)=>{assert.equal(name,'password_vault_load');return JSON.stringify(encrypted)}},'@tauri-apps/plugin-dialog':{save:async()=>{dialogs++;return 'export.json'},open:async()=>null},'@tauri-apps/plugin-fs':{writeTextFile:async(p,v)=>writes.push({p,v})},'@/lib/passwordVault':{readVault:JSON.parse,unlockVault:async(raw,password)=>{checks++;assert.equal(raw,JSON.stringify(encrypted));if(password!=='correct-master')throw Error('Неверный мастер-пароль')}}}
 const mod=new Module(filename,module);mod.paths=module.paths;mod.require=name=>stubs[name]||require(name)
 mod._compile(ts.transpileModule(compile(processed.code,{filename,generate:'dom',css:'external'}).js.code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText,filename)
 const component=new mod.exports.default({target:document.body}),user=userEvent.setup({document})
 await user.click(screen.getByRole('button',{name:'Экспортировать пароли'}));assert.equal(dialogs,0);assert.equal(writes.length,0)
 await user.type(screen.getByLabelText('Мастер-пароль текущего хранилища'),'wrong-master');await user.click(screen.getByRole('button',{name:'Проверить и экспортировать'}));await screen.findByRole('alert');assert.equal(dialogs,0);assert.equal(writes.length,0);assert.equal(screen.getByLabelText('Мастер-пароль текущего хранилища').value,'')
 await user.click(screen.getByRole('button',{name:'Отмена'}));assert.equal(screen.queryByRole('form',{name:'Подтверждение экспорта паролей'}),null)
 await user.click(screen.getByRole('button',{name:'Экспортировать пароли'}));await user.type(screen.getByLabelText('Мастер-пароль текущего хранилища'),'correct-master');await user.click(screen.getByRole('button',{name:'Проверить и экспортировать'}));await screen.findByRole('status')
 assert.equal(checks,2);assert.equal(dialogs,1);assert.deepEqual(writes,[{p:'export.json',v:JSON.stringify(encrypted)}]);assert.equal(screen.queryByRole('form',{name:'Подтверждение экспорта паролей'}),null)
 component.$destroy();console.log('Vault export UI: password required, wrong password blocks file dialog/write, cancellation clears form, authenticated export stays encrypted')
}
main().then(()=>process.exit(0)).catch(e=>{console.error(e);process.exit(1)})
