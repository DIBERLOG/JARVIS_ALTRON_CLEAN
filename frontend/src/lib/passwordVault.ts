export type PasswordCard={id:string,title:string,login:string,password:string,site:string,html:string,updatedAt:string}
type Sealed={iv:string,data:string}
export type VaultFile={version:1,salt:string,passwordKey:Sealed,recoveryKey:Sealed,cards:Sealed}
const encoder=new TextEncoder(),decoder=new TextDecoder()
const b64=(bytes:Uint8Array)=>btoa(Array.from(bytes,value=>String.fromCharCode(value)).join(''))
const un64=(value:string)=>Uint8Array.from(atob(value),char=>char.charCodeAt(0))
const random=(length:number)=>crypto.getRandomValues(new Uint8Array(length))
async function importKey(bytes:Uint8Array){return crypto.subtle.importKey('raw',bytes as BufferSource,'AES-GCM',false,['encrypt','decrypt'])}
async function passwordKey(password:string,salt:string){
    const material=await crypto.subtle.importKey('raw',encoder.encode(password),'PBKDF2',false,['deriveKey'])
    return crypto.subtle.deriveKey({name:'PBKDF2',salt:un64(salt) as BufferSource,iterations:600000,hash:'SHA-256'},material,{name:'AES-GCM',length:256},false,['encrypt','decrypt'])
}
async function seal(key:CryptoKey,bytes:Uint8Array):Promise<Sealed>{const iv=random(12);return {iv:b64(iv),data:b64(new Uint8Array(await crypto.subtle.encrypt({name:'AES-GCM',iv:iv as BufferSource},key,bytes as BufferSource)))}}
async function open(key:CryptoKey,sealed:Sealed){return new Uint8Array(await crypto.subtle.decrypt({name:'AES-GCM',iv:un64(sealed.iv) as BufferSource},key,un64(sealed.data) as BufferSource))}
function requirePassword(password:string){if(password.length<6||password.length>1024)throw Error('Мастер-пароль должен содержать от 6 до 1024 символов. Лучше используйте длинную уникальную фразу.')}
function readVault(value:string):VaultFile{const file=JSON.parse(value);if(file.version!==1||typeof file.salt!=='string'||!file.passwordKey||!file.recoveryKey||!file.cards)throw Error('Формат хранилища повреждён или не поддерживается');return file}
export async function createVault(password:string){
    requirePassword(password)
    const raw=random(32),recovery=random(32),salt=b64(random(16)),key=await importKey(raw)
    const file:VaultFile={version:1,salt,passwordKey:await seal(await passwordKey(password,salt),raw),recoveryKey:await seal(await importKey(recovery),raw),cards:await seal(key,encoder.encode('[]'))}
    raw.fill(0)
    return {file,key,recovery:Array.from(recovery,byte=>byte.toString(16).padStart(2,'0')).join('').match(/.{1,8}/g)!.join('-')}
}
export async function unlockVault(value:string,password:string){
    const file=readVault(value)
    let raw:Uint8Array
    try{raw=await open(await passwordKey(password,file.salt),file.passwordKey)}catch{throw Error('Неверный мастер-пароль или повреждённое хранилище.')}
    const key=await importKey(raw);raw.fill(0)
    return {file,key,cards:await readCards(file,key)}
}
export async function recoverVault(value:string,recovery:string,newPassword:string){
    requirePassword(newPassword)
    const hex=recovery.replace(/[-\s]/g,'')
    if(!/^[a-f0-9]{64}$/i.test(hex))throw Error('Введите полный ключ восстановления из 64 символов.')
    const file=readVault(value)
    let raw:Uint8Array
    try{raw=await open(await importKey(Uint8Array.from(hex.match(/../g)!,part=>parseInt(part,16))),file.recoveryKey)}catch{throw Error('Неверный ключ восстановления или повреждённое хранилище.')}
    try{const key=await importKey(raw),cards=await readCards(file,key),salt=b64(random(16));return {file:{...file,salt,passwordKey:await seal(await passwordKey(newPassword,salt),raw)},key,cards}}finally{raw.fill(0)}
}
async function readCards(file:VaultFile,key:CryptoKey):Promise<PasswordCard[]>{const cards=JSON.parse(decoder.decode(await open(key,file.cards)));if(!Array.isArray(cards))throw Error('Данные хранилища повреждены');return cards}
export async function updateVault(file:VaultFile,key:CryptoKey,cards:PasswordCard[]){const bytes=encoder.encode(JSON.stringify(cards));if(bytes.length>24_000_000)throw Error('Хранилище больше 24 МБ. Уменьшите количество картинок.');return {...file,cards:await seal(key,bytes)}}
