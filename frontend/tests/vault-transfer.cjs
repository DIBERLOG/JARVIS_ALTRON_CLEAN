const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{Module}=require('node:module'),ts=require('typescript')
globalThis.crypto=require('node:crypto').webcrypto
const filename=path.resolve('src/lib/passwordVault.ts'),mod=new Module(filename,module);mod.paths=module.paths
mod._compile(ts.transpileModule(fs.readFileSync(filename,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText,filename)
async function main(){
 const v=mod.exports,created=await v.createVault('test-master-password'),cards=[{id:'test',title:'Test',login:'private-login',password:'private-password',site:'',html:'',updatedAt:new Date().toISOString()}]
 const file=await v.updateVault(created.file,created.key,cards),raw=JSON.stringify(v.readVault(JSON.stringify(file)))
 assert.ok(!raw.includes('private-password'));assert.ok(!raw.includes('private-login'))
 assert.deepEqual((await v.unlockVault(raw,'test-master-password')).cards,cards)
 await assert.rejects(()=>v.unlockVault(raw,'wrong-password'))
 const recovered=await v.recoverVault(raw,created.recovery,'replacement-password');assert.deepEqual((await v.unlockVault(JSON.stringify(recovered.file),'replacement-password')).cards,cards)
 assert.throws(()=>v.readVault(JSON.stringify({...file,version:99})))
 assert.throws(()=>v.readVault(JSON.stringify({...file,salt:'invalid'})))
 if(process.argv[2]&&fs.existsSync(process.argv[2]))v.readVault(fs.readFileSync(process.argv[2],'utf8'))
 console.log('Encrypted vault: transfer, wrong-password rejection, recovery and format validation passed; existing vault checked read-only')
}
main().catch(e=>{console.error(e);process.exitCode=1})
