import {writable} from 'svelte/store'
export type RequestEntry={id:string,text:string,source:'chat'|'voice',at:string}
const key='jarvis-request-history-v1'
let saved:RequestEntry[]=[]
try{const value=JSON.parse(localStorage.getItem(key)||'[]');if(Array.isArray(value))saved=value.filter(item=>typeof item.id==='string'&&typeof item.text==='string'&&item.text.length<=12000&&['chat','voice'].includes(item.source)&&Number.isFinite(Date.parse(item.at))).slice(0,500)}catch{}
export const requestHistory=writable<RequestEntry[]>(saved)
function persist(entries:RequestEntry[]){try{localStorage.setItem(key,JSON.stringify(entries))}catch{console.warn('Не удалось сохранить историю запросов')}}
export function recordRequest(text:string,source:RequestEntry['source']){
    if(!text.trim())return
    requestHistory.update(entries=>{const next=[{id:crypto.randomUUID(),text:text.trim().slice(0,12000),source,at:new Date().toISOString()},...entries].slice(0,500);persist(next);return next})
}
export function clearRequestHistory(){persist([]);requestHistory.set([])}
