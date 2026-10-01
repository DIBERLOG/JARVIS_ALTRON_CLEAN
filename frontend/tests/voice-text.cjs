// Run: node --conditions=browser tests/voice-text.cjs <test dependency directory>
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),vm=require('node:vm')
const {createRequire,Module}=require('node:module')
const deps=createRequire(path.resolve(process.argv[2],'package.json'))
const {JSDOM}=deps('jsdom')
const dom=new JSDOM('<!doctype html><body></body>',{url:'http://localhost'})
for(const name of ['window','document','navigator','HTMLElement','Node','Event','MouseEvent','getComputedStyle','localStorage'])Object.defineProperty(globalThis,name,{value:dom.window[name],configurable:true})
const {screen,waitFor}=deps('@testing-library/dom'),userEvent=deps('@testing-library/user-event').default
const ts=require('typescript'),{compile,preprocess}=require('svelte/compiler')
function evaluate(code,filename,overrides){const mod=new Module(filename,module);mod.filename=filename;mod.paths=module.paths;mod.require=name=>overrides[name]||require(name);mod._compile(ts.transpileModule(code,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2020}}).outputText,filename);return mod.exports}
async function main(){
    const svelte=await import('svelte'),stores=await import('svelte/store')
    const muted=stores.writable(false),connected=stores.writable(true),known=stores.writable(true)
    const mutedChanges=[],requests=[];let listenerRunning=false
    const audio=evaluate(fs.readFileSync('src/lib/voiceToText.ts','utf8'),path.resolve('src/lib/voiceToText.ts'),{'svelte/store':stores,'@tauri-apps/api/core':{invoke:async(...args)=>{requests.push(args);return args[0]==='is_jarvis_app_running'?listenerRunning:'test'}},'./ipc':{ipcConnected:connected,microphoneMuted:muted,microphoneMuteKnown:known,setMicrophoneMuted:value=>{mutedChanges.push(value);muted.set(value);return true}}})
    const pcm=audio.pcm16(new Float32Array([1,1,1,-1,-1,-1]),48000)
    assert.equal(new DataView(pcm.buffer).getInt16(0,true),32767)
    assert.equal(new DataView(pcm.buffer).getInt16(2,true),-32768)
    assert.throws(()=>audio.pcm16(new Float32Array(1),8000))
    assert.throws(()=>audio.pcm16(new Float32Array(48000*121),48000))
    const restore=await audio.pauseCommandListener();assert.equal(stores.get(muted),true);restore();assert.deepEqual(mutedChanges,[true,false])
    mutedChanges.length=0;muted.set(true);(await audio.pauseCommandListener())();assert.deepEqual(mutedChanges,[],'Already muted listener must stay muted')
    await audio.transcribeAudio(pcm);assert.equal(requests[0][0],'center_transcribe_audio');assert.equal(requests[0][1],pcm)
    connected.set(false);listenerRunning=true;await assert.rejects(audio.pauseCommandListener(),/Нет связи/);listenerRunning=false;(await audio.pauseCommandListener())();connected.set(true)
    // Test the actual AudioWorklet batching and tail flush without capturing a microphone.
    let Recorder
    vm.runInNewContext(fs.readFileSync('public/dictation-worklet.js','utf8'),{Float32Array,AudioWorkletProcessor:class{constructor(){this.port={postMessage:value=>this.messages.push(value)};this.messages=[]}},registerProcessor:(_,value)=>Recorder=value})
    const worklet=new Recorder();worklet.process([[new Float32Array(128).fill(.5)]]);worklet.port.onmessage({data:'flush'});assert.equal(worklet.messages[0].length,128);assert.equal(worklet.messages[1],'flushed')
    let stoppedTracks=0,closedContexts=0,currentNode,rejectPermission=false
    Object.defineProperty(navigator,'mediaDevices',{configurable:true,value:{getUserMedia:async()=>{if(rejectPermission)throw {name:'NotAllowedError'};return {getTracks:()=>[{stop:()=>stoppedTracks++}]}}}})
    class FakeContext {
        constructor(){this.sampleRate=48000;this.audioWorklet={addModule:async url=>assert.equal(url,'/dictation-worklet.js')};this.destination={}}
        async resume(){}
        createMediaStreamSource(){return {connect(){},disconnect(){}}}
        createGain(){return {gain:{value:1},connect(){}}}
        async close(){closedContexts++}
        async decodeAudioData(){return {duration:1,length:48000,sampleRate:48000,numberOfChannels:2,getChannelData:channel=>new Float32Array(48000).fill(channel===0?1:-1)}}
    }
    global.AudioContext=FakeContext
    global.AudioWorkletNode=class {
        constructor(){currentNode=this;this.port={close(){},postMessage:()=>queueMicrotask(()=>this.port.onmessage({data:'flushed'}))}}
        connect(){} disconnect(){}
    }
    const capture=await audio.recordVoice(()=>{})
    currentNode.port.onmessage({data:new Float32Array(4800).fill(.5)})
    assert.equal((await capture.stop()).length,3200);assert.equal(stoppedTracks,1);assert.equal(closedContexts,1)
    const cancelCapture=await audio.recordVoice(()=>{});cancelCapture.cancel();assert.equal(stoppedTracks,2)
    const silentCapture=await audio.recordVoice(()=>{});currentNode.port.onmessage({data:new Float32Array(4800)});await assert.rejects(silentCapture.stop(),/нет звука/)
    const quietCapture=await audio.recordVoice(()=>{},'physical-mic');currentNode.port.onmessage({data:new Float32Array(4800).fill(.01)});const quietPcm=await quietCapture.stop();assert.ok(new DataView(quietPcm.buffer).getInt16(0,true)>3000,'Quiet microphone audio must be amplified')
    rejectPermission=true;await assert.rejects(audio.recordVoice(()=>{}),/Разрешите доступ/);rejectPermission=false
    const decoded=await audio.audioFilePcm({size:100,arrayBuffer:async()=>new ArrayBuffer(0)})
    assert.equal(decoded.length,32000);assert.ok(decoded.every(value=>value===0),'Stereo channels must be mixed to mono')
    await assert.rejects(audio.audioFilePcm({size:11*1024*1024}),/10 МБ/)
    // Primary capture uses MediaRecorder data, not potentially silent worklet frames.
    Object.defineProperty(navigator,'mediaDevices',{configurable:true,value:{getUserMedia:async()=>({getTracks:()=>[{stop:()=>stoppedTracks++}],getAudioTracks:()=>[{muted:false}]})}})
    global.File=class extends Blob{constructor(parts,name,options){super(parts,options);this.name=name}}
    class RecorderContext extends FakeContext{
        createAnalyser(){return {fftSize:2048,getFloatTimeDomainData:values=>values.fill(.1)}}
        async decodeAudioData(){return {duration:1,length:48000,sampleRate:48000,numberOfChannels:1,getChannelData:()=>new Float32Array(48000).fill(.1)}}
    }
    global.AudioContext=RecorderContext
    let recorderStopped=0
    global.MediaRecorder=class{
        static isTypeSupported(){return true}
        constructor(){this.mimeType='audio/webm';this.state='inactive'}
        start(){this.state='recording'}
        stop(){recorderStopped++;this.state='inactive';queueMicrotask(()=>{this.ondataavailable({data:new Blob(['audio'])});this.onstop?.()})}
    }
    const mediaCapture=await audio.recordVoice(()=>{})
    assert.equal((await mediaCapture.stop()).length,32000)
    const canceledMedia=await audio.recordVoice(()=>{});canceledMedia.cancel();assert.equal(recorderStopped,2)
    delete global.MediaRecorder
    const draft=stores.writable({title:'',text:''});let canceled=0,restored=0,recognized=0,fail=false,pendingStart=false,resolveStart
    const helper={dictationDraft:draft,MAX_RECORDING_SECONDS:120,pauseCommandListener:async()=>()=>restored++,recordVoice:async()=>{const session={cancel:()=>canceled++,stop:async()=>pcm};if(pendingStart)return new Promise(resolve=>resolveStart=()=>resolve(session));return session},audioFilePcm:async()=>pcm,transcribeAudio:async()=>{recognized++;if(fail)throw 'Понятная ошибка распознавания';return 'Это проверка диктовки'}}
    const filename=path.resolve('src/components/CenterVoiceText.svelte')
    const processed=await preprocess(fs.readFileSync(filename,'utf8'),{script:({content,attributes})=>attributes.lang==='ts'?{code:ts.transpileModule(content,{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ESNext}}).outputText}:undefined},{filename})
    const Component=evaluate(compile(processed.code,{filename,generate:'dom',css:'external'}).js.code,filename,{svelte,'@/lib/voiceToText':helper,'@/lib/center':{makeId:()=> 'dictation-note'}}).default
    let saved
    const component=new Component({target:document.body,props:{onSave:async note=>{saved=note;return true}}})
    const user=userEvent.setup({document})
    await user.click(screen.getByRole('button',{name:/Начать запись/}))
    assert.ok(await screen.findByRole('img',{name:/Запись: ожидаю голос/}))
    assert.equal(screen.getByRole('combobox',{name:'Микрофон для диктовки'}).disabled,true)
    await user.click(await screen.findByRole('button',{name:/Остановить и распознать/}))
    await screen.findByDisplayValue('Это проверка диктовки')
    assert.equal(restored,1)
    await user.type(screen.getByRole('textbox',{name:'Название заметки из диктовки'}),'Моя мысль')
    await user.click(screen.getByRole('button',{name:/Сохранить в заметки/}))
    await waitFor(()=>assert.equal(saved.title,'Моя мысль'));assert.equal(saved.text,'Это проверка диктовки');assert.equal(saved.format,'plain')
    await user.click(screen.getByRole('button',{name:/Начать запись/}))
    await user.click(await screen.findByRole('button',{name:/Остановить и распознать/}))
    await screen.findByDisplayValue(/Это проверка диктовки\s+Это проверка диктовки/)
    fail=true
    await user.click(screen.getByRole('button',{name:/Начать запись/}));await user.click(await screen.findByRole('button',{name:/Остановить и распознать/}))
    assert.ok(await screen.findByRole('alert'));assert.equal(stores.get(draft).text,'Это проверка диктовки\n\nЭто проверка диктовки')
    await user.click(screen.getByRole('button',{name:/Начать запись/}));await screen.findByRole('button',{name:/Остановить и распознать/});component.$destroy()
    assert.equal(restored,4);assert.equal(recognized,3)
    const remounted=new Component({target:document.body,props:{onSave:async()=>true}})
    assert.ok(screen.getByDisplayValue(/Это проверка диктовки\s+Это проверка диктовки/))
    pendingStart=true;await user.click(screen.getByRole('button',{name:/Начать запись/}));await waitFor(()=>assert.ok(resolveStart));remounted.$destroy();resolveStart();await svelte.tick();await new Promise(resolve=>setTimeout(resolve,0));assert.equal(restored,5);assert.ok(canceled>=5)
    fail=false;pendingStart=false
    const fileComponent=new Component({target:document.body,props:{onSave:async()=>true}})
    await user.upload(screen.getByLabelText('Загрузить аудиофайл'),new dom.window.File(['audio'],'voice.wav',{type:'audio/wav'}))
    await waitFor(()=>assert.equal(recognized,4))
    await screen.findByDisplayValue(/Это проверка диктовки\s+Это проверка диктовки\s+Это проверка диктовки/)
    fileComponent.$destroy()
    console.log('PASS: PCM conversion, listener pause/restore, worklet flush, dictation, append, notes, errors, draft retention and cleanup')
}
main().catch(error=>{console.error(error);process.exitCode=1})
