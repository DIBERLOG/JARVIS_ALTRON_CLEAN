class DictationRecorder extends AudioWorkletProcessor {
    constructor(){
        super();this.buffer=new Float32Array(2048);this.offset=0
        this.port.onmessage=event=>{if(event.data==='flush'){this.flush();this.port.postMessage('flushed')}}
    }
    flush(){
        if(!this.offset)return
        const frame=this.buffer.slice(0,this.offset)
        this.port.postMessage(frame,[frame.buffer]);this.offset=0
    }
    process(inputs){
        const input=inputs[0]?.[0]
        if(input)for(const sample of input){this.buffer[this.offset++]=sample;if(this.offset===this.buffer.length)this.flush()}
        return true
    }
}
registerProcessor('jarvis-dictation',DictationRecorder)
