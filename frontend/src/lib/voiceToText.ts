import {writable,get} from 'svelte/store'
import {invoke} from '@tauri-apps/api/core'
import {ipcConnected,microphoneMuted,microphoneMuteKnown,setMicrophoneMuted} from './ipc'

export const dictationDraft=writable({title:'',text:''})
export const MAX_RECORDING_SECONDS=120
export type VoiceRecording={stop:()=>Promise<Uint8Array>;cancel:()=>void}

/** Average downsampling to mono PCM16 at the recognizer's 16 kHz rate. */
export function pcm16(samples:Float32Array,sampleRate:number):Uint8Array {
    if(!Number.isFinite(sampleRate)||sampleRate<16000)throw Error('Неподдерживаемая частота аудио')
    if(samples.length/sampleRate>MAX_RECORDING_SECONDS+.25)throw Error('Аудио длиннее двух минут. Разделите его на части.')
    const ratio=sampleRate/16000,length=Math.floor(samples.length/ratio)
    const bytes=new Uint8Array(length*2),view=new DataView(bytes.buffer)
    for(let i=0;i<length;i++){
        const from=Math.floor(i*ratio),to=Math.min(samples.length,Math.floor((i+1)*ratio))
        let total=0;for(let j=from;j<to;j++)total+=samples[j]
        const sample=Math.max(-1,Math.min(1,total/Math.max(1,to-from)))
        view.setInt16(i*2,Math.round(sample*(sample<0?32768:32767)),true)
    }
    return bytes
}

export async function pauseCommandListener():Promise<()=>void> {
    if(!get(ipcConnected)){
        if(await invoke<boolean>('is_jarvis_app_running'))throw Error('Нет связи со слушателем JARVIS. Дождитесь подключения или остановите его перед диктовкой.')
        return ()=>{}
    }
    if(!get(microphoneMuteKnown))throw Error('Дождитесь подключения JARVIS и повторите запись.')
    if(get(microphoneMuted))return ()=>{}
    if(!setMicrophoneMuted(true))throw Error('Не удалось приостановить слушание команд.')
    const restore=()=>{if(get(microphoneMuted))setMicrophoneMuted(false)}
    try {
        await new Promise<void>((resolve,reject)=>{
            let unsubscribe=()=>{}
            const timeout=setTimeout(()=>{unsubscribe();reject(Error('JARVIS не подтвердил паузу слушания. Повторите попытку.'))},1500)
            unsubscribe=microphoneMuted.subscribe(muted=>{if(muted){clearTimeout(timeout);queueMicrotask(()=>unsubscribe());resolve()}})
        })
    }catch(error){setMicrophoneMuted(false);throw error}
    return restore
}

export async function recordVoice(onLevel:(level:number)=>void,deviceId=''):Promise<VoiceRecording> {
    if(!navigator.mediaDevices?.getUserMedia)throw Error('Микрофон недоступен. Проверьте разрешения Windows.')
    let stream:MediaStream
    try{stream=await navigator.mediaDevices.getUserMedia({audio:{channelCount:1,...(deviceId?{deviceId:{exact:deviceId}}:{}),echoCancellation:false,noiseSuppression:false,autoGainControl:true},video:false})}
    catch(error){const name=(error as DOMException)?.name;throw Error(name==='NotAllowedError'?'Разрешите доступ к микрофону в настройках Windows.':name==='NotFoundError'?'Микрофон не найден. Подключите его и повторите попытку.':'Не удалось открыть микрофон. Проверьте, что устройство доступно.')}
    let context:AudioContext|undefined,source:MediaStreamAudioSourceNode|undefined,node:AudioWorkletNode|undefined
    let closed=false,frames:Float32Array[]=[],total=0,flushed:()=>void=()=>{}
    // Record the original media stream, independently of the Web Audio graph.
    // Some WebView audio graphs deliver silent worklet input despite a live track.
    if(typeof MediaRecorder!=='undefined'){
        let recorder:MediaRecorder|undefined,meter:ReturnType<typeof setInterval>|undefined
        const chunks:Blob[]=[],track=stream.getAudioTracks()[0]
        const cleanup=()=>{clearInterval(meter);stream.getTracks().forEach(track=>track.stop());source?.disconnect();void context?.close().catch(()=>{});onLevel(0)}
        try{
            context=new AudioContext()
            source=context.createMediaStreamSource(stream)
            const analyser=context.createAnalyser();analyser.fftSize=2048
            source.connect(analyser);await context.resume()
            const values=new Float32Array(analyser.fftSize)
            meter=setInterval(()=>{analyser.getFloatTimeDomainData(values);let energy=0;for(const value of values)energy+=value*value;onLevel(Math.min(1,Math.sqrt(energy/values.length)*8))},60)
            const mime=['audio/webm;codecs=opus','audio/webm','audio/mp4'].find(type=>MediaRecorder.isTypeSupported(type))
            recorder=new MediaRecorder(stream,mime?{mimeType:mime}:{})
            recorder.ondataavailable=event=>{if(event.data.size)chunks.push(event.data)}
            let failed=false
            recorder.onerror=()=>{failed=true}
            recorder.start(250)
            return {cancel:()=>{if(closed)return;closed=true;if(recorder!.state!=='inactive')recorder!.stop();cleanup();chunks.length=0},stop:async()=>{
                if(closed)throw Error('Запись отменена')
                closed=true
                try{
                    await new Promise<void>((resolve,reject)=>{const timeout=setTimeout(()=>reject(Error('Не удалось завершить запись. Повторите попытку.')),3000);recorder!.onstop=()=>{clearTimeout(timeout);resolve()};recorder!.stop()})
                    if(failed||track?.muted)throw Error('Микрофон не передаёт звук. Выберите физический микрофон вместо виртуального устройства.')
                    const blob=new Blob(chunks,{type:recorder!.mimeType})
                    const bytes=await audioFilePcm(new File([blob],'dictation.webm',{type:blob.type}))
                    let peak=0,energy=0;const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength)
                    for(let i=0;i<bytes.length;i+=2){const value=view.getInt16(i,true);peak=Math.max(peak,Math.abs(value));energy+=value*value}
                    if(!bytes.length||Math.sqrt(energy/(bytes.length/2))<3)throw Error('Микрофон записал тишину. Выберите другое устройство в списке микрофонов; виртуальный микрофон может не передавать голос.')
                    const gain=Math.min(12,26000/Math.max(1,peak))
                    if(gain>1)for(let i=0;i<bytes.length;i+=2)view.setInt16(i,Math.round(view.getInt16(i,true)*gain),true)
                    return bytes
                }finally{cleanup();chunks.length=0}
            }}
        }catch(error){cleanup();throw error}
    }
    const cancel=()=>{if(closed)return;closed=true;source?.disconnect();node?.disconnect();node?.port.close();stream.getTracks().forEach(track=>track.stop());void context?.close().catch(()=>{});onLevel(0);frames=[]}
    try{
        context=new AudioContext({sampleRate:48000})
        await context.audioWorklet.addModule('/dictation-worklet.js')
        await context.resume()
        source=context.createMediaStreamSource(stream);node=new AudioWorkletNode(context,'jarvis-dictation')
        const mute=context.createGain();mute.gain.value=0
        node.port.onmessage=event=>{
            if(event.data==='flushed'){flushed();return}
            if(closed||!(event.data instanceof Float32Array))return
            const available=Math.max(0,Math.floor(context!.sampleRate*MAX_RECORDING_SECONDS)-total)
            const frame=event.data.slice(0,available)
            if(frame.length){frames.push(frame);total+=frame.length;let energy=0;for(const value of frame)energy+=value*value;onLevel(Math.min(1,Math.sqrt(energy/frame.length)*5))}
        }
        source.connect(node);node.connect(mute);mute.connect(context.destination)
        return {cancel,stop:async()=>{
            if(closed)throw Error('Запись отменена')
            source?.disconnect()
            await new Promise<void>(resolve=>{const timer=setTimeout(resolve,500);flushed=()=>{clearTimeout(timer);resolve()};node?.port.postMessage('flush')})
            const samples=new Float32Array(total);let offset=0
            for(const frame of frames){samples.set(frame,offset);offset+=frame.length}
            const sampleRate=context!.sampleRate;cancel()
            let peak=0,energy=0
            for(const value of samples){peak=Math.max(peak,Math.abs(value));energy+=value*value}
            if(!samples.length||Math.sqrt(energy/samples.length)<.0001)throw Error('В записи нет звука. Выберите нужный микрофон и проверьте, двигаются ли волны при разговоре.')
            // Quiet physical microphones need consistent input levels for Vosk.
            const gain=Math.min(12,.8/Math.max(peak,.0001))
            if(gain>1)for(let i=0;i<samples.length;i++)samples[i]*=gain
            return pcm16(samples,sampleRate)
        }}
    }catch(error){cancel();throw error}
}

export async function audioFilePcm(file:File):Promise<Uint8Array> {
    if(file.size>10*1024*1024)throw Error('Файл больше 10 МБ. Выберите короткую запись.')
    const context=new AudioContext({sampleRate:48000})
    try{
        const audio=await context.decodeAudioData(await file.arrayBuffer())
        if(audio.duration>MAX_RECORDING_SECONDS)throw Error('Аудио длиннее двух минут. Разделите его на части.')
        const mono=new Float32Array(audio.length)
        for(let channel=0;channel<audio.numberOfChannels;channel++){const values=audio.getChannelData(channel);for(let i=0;i<values.length;i++)mono[i]+=values[i]/audio.numberOfChannels}
        return pcm16(mono,audio.sampleRate)
    }catch(error){if(error instanceof Error&&error.message.includes('двух минут'))throw error;throw Error('Не удалось прочитать аудио. Попробуйте файл WAV, MP3 или M4A.')}
    finally{await context.close()}
}

export function transcribeAudio(bytes:Uint8Array):Promise<string>{return invoke<string>('center_transcribe_audio',bytes)}
