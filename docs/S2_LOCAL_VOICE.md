# Local S2 Pro voice

Settings now offer `tts_mode=s2`, `xtts`, and `silero` as separate choices. After
saving, the speech queue switches its local worker on the next generated reply.
S2 uses S2Speak directly; XTTS uses the original trained XTTS implementation.
The selected XTTS checkpoint remains unchanged. The legacy local-engine.txt flag
is no longer used. Release/debug runtime resources must be kept in sync.

S2Speak starts or reuses audio.cpp 0.9.0 at **127.0.0.1:29871**. It never uses a
proxy for localhost requests. The model stays resident; GUI and the voice module
share one server. A Windows file lock prevents duplicate simultaneous startup.
The server has no browser UI and listens only on loopback. Generated audio and
reference recordings are not sent to an external service.

Model: S2 Pro Q4_K with the fast audio decoder and codec preserved at their input
precision. Runtime: Vulkan, device 1 (RTX 5060 on this machine). Reference:
jarvis_ru_012.wav. Seed 28, temperature .7, top_p .8, top_k 30. No voice filters or
postprocessing; playback alone adds 80 ms silence to let the sound device drain.

Cache: `%LOCALAPPDATA%/JarvisVoiceStudio/speech-cache/s2-pro`. Keys include model
path/modification time, reference modification time, sampling parameters and the
exact text. `latency.jsonl` records cache hits and preparation time without logging
the spoken text. Ready-made command MP3s continue using the existing audio queue.

The first request also prepares backend graphs/reference conditioning. Measured
locally: first request 23.8 s; a different warm 110-character request 4.7 s; cached
request 0.2 ms preparation. These are observations, not fixed latency guarantees.
This engine path is offline synthesis: playback starts after a full WAV is ready.
CUDA and streaming are not enabled or claimed as tested improvements.

Server config/logs and the isolated model live in
`tools/voice_training/s2_local_test` (ignored by Git). The resident process may
outlive a client so both JARVIS clients can reuse it. If explicitly stopped it is
started again on the next local-voice request. Cancellation stops client playback
immediately, but an in-flight server generation may finish in the background.
