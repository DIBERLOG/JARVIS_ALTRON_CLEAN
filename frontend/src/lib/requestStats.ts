import type {RequestEntry} from './requestHistory'

export function requestStats(entries:RequestEntry[]){
    const hours=Array(24).fill(0) as number[]
    const phrases=new Map<string,{text:string,count:number}>()
    for(const item of entries){
        const date=new Date(item.at)
        if(Number.isFinite(date.getTime()))hours[date.getHours()]++
        const key=item.text.trim().replace(/\s+/g,' ').toLocaleLowerCase('ru-RU')
        if(!key)continue
        const phrase=phrases.get(key)
        if(phrase)phrase.count++
        else phrases.set(key,{text:item.text.trim(),count:1})
    }
    const top=[...phrases.values()].sort((a,b)=>b.count-a.count||a.text.localeCompare(b.text,'ru')).slice(0,5)
    return {hours,top,maxHour:Math.max(1,...hours),maxPhrase:Math.max(1,...top.map(item=>item.count)),unique:phrases.size}
}
