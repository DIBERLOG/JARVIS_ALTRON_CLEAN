use std::sync::mpsc::Receiver;
use std::time::SystemTime;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use jarvis_core::{audio_buffer::AudioRingBuffer, audio_processing, commands, config, listener, recorder, stt, COMMANDS_LIST, intent, voices, ipc::{self, IpcEvent}, i18n, slots, tts};
use rand::seq::SliceRandom;

use crate::{microphone_muted, should_stop};

static DIALOGUE_MODE: AtomicBool = AtomicBool::new(false);
static LAST_COMMAND: Mutex<Option<String>> = Mutex::new(None);
static DIALOGUE_HISTORY: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn dialogue_start_phrase(text: &str) -> bool {
    matches!(dialogue_phrase(text), "давай пообщаемся" | "давай поговорим" | "включи диалоговый режим")
}

fn dialogue_stop_phrase(text: &str) -> bool {
    matches!(dialogue_phrase(text), "закончи разговор" | "закончим разговор" |
        "хватит общаться" | "выключи диалоговый режим" |
        "закрой диалог" | "режим команд")
}

fn dialogue_phrase(text: &str) -> &str {
    text.trim().trim_matches(|ch: char| ch.is_ascii_punctuation() || ch == '«' || ch == '»' || ch == '…').trim()
}

#[cfg(test)]
mod dialogue_tests {
    use super::{dialogue_start_phrase, dialogue_stop_phrase};

    #[test]
    fn start_and_exit_phrases_accept_terminal_punctuation() {
        assert!(dialogue_start_phrase("давай пообщаемся!"));
        assert!(dialogue_stop_phrase("закрой диалог."));
        assert!(dialogue_stop_phrase("режим команд"));
        assert!(!dialogue_stop_phrase("расскажи про режим команд"));
    }
}

// VAD state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum VadState {
    WaitingForVoice,
    VoiceActive,
}

pub fn start(text_cmd_rx: Receiver<String>, rt: &tokio::runtime::Runtime) -> Result<(), ()> {
    main_loop(text_cmd_rx, rt)
}

fn main_loop(text_cmd_rx: Receiver<String>, rt: &tokio::runtime::Runtime) -> Result<(), ()> {
    let frame_length: usize = 512;
    let sample_rate: usize = 16000;
    let mut frame_buffer: Vec<i16> = vec![0; frame_length];
    
    // ring buffer: keeps last 5 seconds of audio (pre-roll)
    let mut audio_buffer = AudioRingBuffer::new(5.0, frame_length, sample_rate);

    // VAD state
    let mut vad_state = VadState::WaitingForVoice;
    let mut silence_frames: u32 = 0;
    
    // how many frames of silence before we consider speech ended
    // 1.5 seconds = 1.5 * (16000 / 512) ≈ 47 frames
    let silence_threshold: u32 = ((1.5 * sample_rate as f32) / frame_length as f32) as u32;
    
    voices::play_greet();

    let mut recording_active = false;
    if !microphone_muted() {
        match recorder::start_recording() {
            Ok(_) => {
                recording_active = true;
                info!("Recording started. Microphone: {}",
                    recorder::get_audio_device_name(recorder::get_selected_microphone_index()));
            }
            Err(_) => {
                error!("Cannot start recording.");
                return Err(());
            }
        }
    }

    ipc::send(IpcEvent::MicrophoneMuted { muted: microphone_muted() });
    ipc::send(IpcEvent::Idle);

    // ### WAKE WORD DETECTION LOOP
    'wake_word: loop {
        if should_stop() {
            info!("Stop signal received, shutting down...");
            voices::play_goodbye();
            ipc::send(IpcEvent::Stopping);
            break;
        }

        if microphone_muted() {
            if recording_active {
                if recorder::stop_recording().is_err() {
                    warn!("Could not stop microphone capture; retrying while audio processing stays paused");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue 'wake_word;
                }
                recording_active = false;
                audio_buffer.clear();
                vad_state = VadState::WaitingForVoice;
                silence_frames = 0;
                stt::reset_wake_recognizer();
                stt::reset_speech_recognizer();
                ipc::send(IpcEvent::MicrophoneMuted { muted: true });
                ipc::send(IpcEvent::Idle);
            }
            if let Ok(text) = text_cmd_rx.try_recv() {
                process_text_command(&text, &rt);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue 'wake_word;
        }

        if !recording_active {
            match recorder::start_recording() {
                Ok(()) => {
                    recording_active = true;
                    audio_buffer.clear();
                    vad_state = VadState::WaitingForVoice;
                    silence_frames = 0;
                    stt::reset_wake_recognizer();
                    stt::reset_speech_recognizer();
                    ipc::send(IpcEvent::MicrophoneMuted { muted: false });
                    ipc::send(IpcEvent::Idle);
                }
                Err(()) => {
                    error!("Cannot resume microphone recording");
                    crate::MICROPHONE_MUTED.store(true, Ordering::SeqCst);
                    ipc::send(IpcEvent::MicrophoneMuted { muted: true });
                    ipc::send(IpcEvent::Error { message: "Не удалось включить микрофон".to_string() });
                    continue 'wake_word;
                }
            }
        }

        // Do not feed Jarvis's own spoken answer back into wake-word detection.
        if tts::is_speaking() {
            recorder::read_microphone(&mut frame_buffer);
            audio_buffer.clear();
            vad_state = VadState::WaitingForVoice;
            stt::reset_wake_recognizer();
            stt::reset_speech_recognizer();
            std::thread::sleep(std::time::Duration::from_millis(20));
            continue 'wake_word;
        }

        if let Ok(text) = text_cmd_rx.try_recv() {
            process_text_command(&text, &rt);
            continue 'wake_word;
        }

        // Dialogue stays open across utterances: no wake word is required until exit.
        if DIALOGUE_MODE.load(Ordering::SeqCst) {
            ipc::send(IpcEvent::Listening);
            recognize_command(&mut frame_buffer, &rt, frame_length, sample_rate, false);
            vad_state = VadState::WaitingForVoice;
            silence_frames = 0;
            audio_buffer.clear();
            stt::reset_wake_recognizer();
            stt::reset_speech_recognizer();
            audio_processing::reset();
            if !DIALOGUE_MODE.load(Ordering::SeqCst) {
                ipc::send(IpcEvent::Idle);
            }
            continue 'wake_word;
        }

        recorder::read_microphone(&mut frame_buffer);
        let processed = audio_processing::process(&frame_buffer);
        
        match vad_state {
            VadState::WaitingForVoice => {
                // always buffer audio
                audio_buffer.push(&frame_buffer);
                
                if processed.is_voice {
                    // voice started! flush buffer to Vosk
                    info!("VAD: Voice started, flushing {} buffered frames", audio_buffer.len());
                    
                    for buffered_frame in audio_buffer.drain_all() {
                        listener::data_callback(&buffered_frame);
                    }
                    
                    vad_state = VadState::VoiceActive;
                    silence_frames = 0;
                }
            }
            
            VadState::VoiceActive => {
                // dual-feed: speech recognizer gets frames in parallel with wake word detector
                let _ = stt::recognize(&frame_buffer, false);

                // feed to wake word detector
                if let Some(_keyword_index) = listener::data_callback(&frame_buffer) {
                    // WAKE WORD DETECTED!
                    info!("Wake word activated!");
                    ipc::send(IpcEvent::WakeWordDetected);
                    
                    stt::reset_wake_recognizer();
                    audio_processing::reset();

                    // brief sniff to keep feeding STT while transitioning
                    let sniff_frames = ((0.3 * sample_rate as f32) / frame_length as f32) as u32;
                    for _ in 0..sniff_frames {
                        recorder::read_microphone(&mut frame_buffer);
                        audio_processing::process(&frame_buffer);
                        stt::recognize(&frame_buffer, false);
                    }

                    ipc::send(IpcEvent::Listening);
                    recognize_command(&mut frame_buffer, &rt, frame_length, sample_rate, true);

                    // reset state after command
                    vad_state = VadState::WaitingForVoice;
                    silence_frames = 0;
                    audio_buffer.clear();
                    stt::reset_wake_recognizer();
                    stt::reset_speech_recognizer(); // NOW reset, after command is done
                    audio_processing::reset();
                    ipc::send(IpcEvent::Idle);
                    
                    continue 'wake_word;
                }
                
                // track silence
                if processed.is_voice {
                    silence_frames = 0;
                } else {
                    silence_frames += 1;
                    
                    if silence_frames > silence_threshold {
                        debug!("VAD: Silence timeout, returning to wait state");
                        vad_state = VadState::WaitingForVoice;
                        silence_frames = 0;
                        stt::reset_wake_recognizer();
                        stt::reset_speech_recognizer(); // reset since we were dual-feeding
                    }
                }
            }
        }
    }

    recorder::stop_recording().ok();
    ipc::send(IpcEvent::Stopping);

    Ok(())
}


// Voice recognition for command after wake word
fn recognize_command(
    frame_buffer: &mut [i16],
    rt: &tokio::runtime::Runtime,
    frame_length: usize,
    sample_rate: usize,
    prefed_audio: bool
) {
    let mut audio_buffer = AudioRingBuffer::new(2.0, frame_length, sample_rate);
    let mut vad_state = if prefed_audio {
        VadState::VoiceActive
    } else {
        VadState::WaitingForVoice
    };
    let mut silence_frames: u32 = 0;
    let mut start = SystemTime::now();
    let mut first_recognition = prefed_audio;
    
    // longer silence threshold for commands (user might pause to think)
    // 5 seconds
    let silence_threshold: u32 = ((5.0 * sample_rate as f32) / frame_length as f32) as u32;
    
    loop {
        if crate::should_stop() {
            return;
        }
        if microphone_muted() {
            return;
        }

        // Do not transcribe the assistant's own queued TTS response.
        if tts::is_speaking() {
            recorder::read_microphone(frame_buffer);
            audio_buffer.clear();
            vad_state = VadState::WaitingForVoice;
            stt::reset_speech_recognizer();
            silence_frames = 0;
            start = SystemTime::now();
            std::thread::sleep(std::time::Duration::from_millis(20));
            continue;
        }
        
        recorder::read_microphone(frame_buffer);
        let processed = audio_processing::process(frame_buffer);
        
        match vad_state {
            VadState::WaitingForVoice => {
                audio_buffer.push(frame_buffer);
                
                if processed.is_voice {
                    // flush buffer to STT
                    for buffered_frame in audio_buffer.drain_all() {
                        stt::recognize(&buffered_frame, false);
                    }
                    vad_state = VadState::VoiceActive;
                    silence_frames = 0;
                } else {
                    silence_frames += 1;
                    
                    if silence_frames > silence_threshold {
                        info!("Long silence detected, returning to wake word mode.");
                        return;
                    }
                }
            }
            
            VadState::VoiceActive => {
                // feed to STT
                if let Some(mut recognized_voice) = stt::recognize(frame_buffer, false) {
                    info!("Recognized voice: {}", recognized_voice);
                    
                    ipc::send(IpcEvent::SpeechRecognized {
                        text: recognized_voice.clone(),
                    });
                    
                    recognized_voice = recognized_voice.to_lowercase();
                    
                    // check if wake word repeated (reactivate)
                    let wake_phrases = config::get_wake_phrases(&i18n::get_language());
                    let contains_wake = wake_phrases.iter().any(|wp| recognized_voice.contains(wp));

                    if contains_wake {
                        // strip the wake word
                        let mut remaining = recognized_voice.clone();
                        for wp in wake_phrases {
                            remaining = remaining.replace(wp, "");
                        }
                        let remaining = remaining.trim();

                        if remaining.is_empty() {
                            if first_recognition {
                                // leftover wake word from dual-feed, just discard it
                                info!("Discarding initial wake word from prefed audio");
                                first_recognition = false;
                                stt::reset_speech_recognizer();
                                voices::play_reply();
                                vad_state = VadState::WaitingForVoice;
                                silence_frames = 0;
                                start = SystemTime::now();
                                audio_buffer.clear();
                                continue;
                            }

                            // just wake word, no command - reactivate
                            info!("Wake word repeated during chaining, reactivating...");
                            voices::play_reply();
                            stt::reset_speech_recognizer();
                            ipc::send(IpcEvent::Listening);
                            
                            vad_state = VadState::WaitingForVoice;
                            silence_frames = 0;
                            start = SystemTime::now();
                            audio_buffer.clear();
                            continue;
                        } else {
                            // wake word + command in one phrase - execute the command part
                            info!("Wake word + command during chaining: '{}'", remaining);
                            recognized_voice = remaining.to_string();
                            // fall through to command execution below
                        }
                    }

                    first_recognition = false;
                    
                    recognized_voice = remove_assistant_phrases(recognized_voice);
                    
                    if recognized_voice.len() < 5 {
                        debug!("Ignoring too short recognition: '{}'", recognized_voice);
                        continue;
                    }

                    if recognized_voice.is_empty() {
                        continue;
                    }
                    
                    // execute command and check if we should chain
                    let should_chain = execute_command(&recognized_voice, rt);
                    
                    if should_chain {
                        // chain: reset and continue listening
                        info!("Chaining enabled, continuing to listen...");
                        stt::reset_speech_recognizer();
                        vad_state = VadState::WaitingForVoice;
                        silence_frames = 0;
                        start = SystemTime::now();
                        audio_buffer.clear();
                        ipc::send(IpcEvent::Listening);
                        continue;
                    } else {
                        // no chain: return to wake word
                        info!("No chain, returning to wake word mode.");
                        return;
                    }
                }
                
                // track silence
                if processed.is_voice {
                    silence_frames = 0;
                } else {
                    silence_frames += 1;
                    
                    if silence_frames > silence_threshold {
                        info!("Long silence detected, returning to wake word mode.");
                        return;
                    }
                }
            }
        }
        
        // timeout
        if let Ok(elapsed) = start.elapsed() {
            if elapsed > config::CMS_WAIT_DELAY {
                info!("Command timeout, returning to wake word mode.");
                return;
            }
        }
    }
}


fn process_text_command(text: &str, rt: &tokio::runtime::Runtime) {
    info!("Processing text command: {}", text);
    
    ipc::send(IpcEvent::SpeechRecognized { text: text.to_string() });
    
    let filtered = remove_assistant_phrases(text.to_lowercase());

    let filtered = filtered.trim();
    
    if filtered.is_empty() {
        ipc::send(IpcEvent::Idle);
        return;
    }
    
    // text commands never chain
    execute_command(filtered, rt);
}


// Execute command, returns true if chaining should continue
fn execute_command(text: &str, rt: &tokio::runtime::Runtime) -> bool {
    let language = i18n::get_language();
    if dialogue_start_phrase(text) {
        DIALOGUE_MODE.store(true, Ordering::SeqCst);
        if let Ok(mut history) = DIALOGUE_HISTORY.lock() { history.clear(); }
        if !voices::play_command_reply("dialogue_start", &language) {
            tts::speak("Слушаю, сэр.");
        }
        ipc::send(IpcEvent::CommandExecuted { id: "dialogue_start".to_string(), success: true });
        ipc::send(IpcEvent::Listening);
        return true;
    }
    if DIALOGUE_MODE.load(Ordering::SeqCst) && dialogue_stop_phrase(text) {
        DIALOGUE_MODE.store(false, Ordering::SeqCst);
        if let Ok(mut history) = DIALOGUE_HISTORY.lock() { history.clear(); }
        if !voices::play_command_reply("dialogue_stop", &language) {
            tts::speak("Как скажете, сэр. Возвращаюсь в режим команд.");
        }
        ipc::send(IpcEvent::CommandExecuted { id: "dialogue_stop".to_string(), success: true });
        ipc::send(IpcEvent::Idle);
        return false;
    }
    // Re-read command manifests so added phrases and edited packs take effect
    // without a restart. The startup snapshot remains a safe fallback.
    let current_commands = commands::parse_commands().ok();
    let commands_list = match current_commands.as_deref().or_else(|| COMMANDS_LIST.get().map(Vec::as_slice)) {
        Some(c) => c,
        None => {
            ipc::send(IpcEvent::Error { message: "Commands not loaded".to_string() });
            ipc::send(IpcEvent::Idle);
            return false;
        }
    };
    
    let exact_command = commands::fetch_exact_command(text, commands_list);
    let in_dialogue = DIALOGUE_MODE.load(Ordering::SeqCst);
    // In dialogue, recognize real commands before treating the utterance as chat.
    // Fuzzy phrase matching handles natural variants such as "открыть браузер".
    let dialogue_command = if in_dialogue {
        exact_command.or_else(|| commands::fetch_command(text, commands_list))
    } else {
        None
    };
    if in_dialogue && dialogue_command.is_none() {
        let history = DIALOGUE_HISTORY.lock().map(|saved| saved.clone()).unwrap_or_default();
        match jarvis_core::chat::ask_with_history(text, &history) {
            Ok(answer) => {
                if let Ok(mut saved) = DIALOGUE_HISTORY.lock() {
                    saved.push((text.to_string(), answer.clone()));
                    let excess = saved.len().saturating_sub(8);
                    if excess > 0 { saved.drain(..excess); }
                }
                tts::speak(&answer);
                return true;
            }
            Err(error) => {
                warn!("Dialogue chat failed: {}", error);
                tts::speak(&format!("Не удалось получить ответ: {error}"));
                return true;
            }
        }
    }
    let cmd_result = dialogue_command.or(exact_command).or_else(|| {
        if let Some((intent_id, confidence)) = rt.block_on(intent::classify(text)) {
            info!("Intent recognized: {} (confidence: {:.2})", intent_id, confidence);
            intent::get_command_by_intent(commands_list, &intent_id)
        } else {
            info!("Intent not recognized, trying phrase similarity...");
            commands::fetch_command(text, commands_list)
        }
    });
    
    if let Some((cmd_path, cmd_config)) = cmd_result {
        info!("Command found: {:?}", cmd_path);

        if cmd_config.id == "repeat_command" {
            let previous = LAST_COMMAND.lock().ok().and_then(|saved| saved.clone());
            if let Some(previous) = previous {
                if !voices::play_command_reply("repeat_command", &language) {
                    tts::speak("Повторяю последнюю команду.");
                }
                ipc::send(IpcEvent::CommandExecuted {
                    id: "repeat_command".to_string(),
                    success: true,
                });
                return execute_command(&previous, rt);
            }
            tts::speak("Пока нечего повторять.");
            ipc::send(IpcEvent::CommandExecuted {
                id: "repeat_command".to_string(),
                success: false,
            });
            ipc::send(IpcEvent::Idle);
            return false;
        }
        
        // extract slots if needed
        let extracted_slots = if !cmd_config.slots.is_empty() {
            let s = slots::extract(text, &cmd_config.slots);
            if !s.is_empty() {
                info!("Extracted slots: {:?}", s);
            }
            Some(s)
        } else {
            None
        };

        // Dynamic Lua responses still speak their values after the recorded introduction.
        // Restart actions must wait for their warning clip before terminating the process.
        let play_before = matches!(cmd_config.id.as_str(),
            "counter" | "counter_add" | "counter_subtract" | "telegram_open" |
            "set_city" | "test_greet_name" | "jarvis_restart" | "computer_restart");
        let played_before = play_before && voices::play_command_reply(&cmd_config.id, &language);
        if !played_before {
            match cmd_config.id.as_str() {
                "counter_add" => { tts::speak("Добавляю один к счётчику."); }
                "counter_subtract" => { tts::speak("Убираю один из счётчика."); }
                "telegram_open" => { tts::speak("Открываю Телеграм."); }
                _ => {}
            }
        }

        match commands::execute_command(&cmd_path, &cmd_config, Some(&text), extracted_slots.as_ref()) {
            Ok(chain) => {
                info!("Command executed successfully");
                if let Ok(mut saved) = LAST_COMMAND.lock() {
                    *saved = Some(text.to_string());
                }
                if !matches!(cmd_config.id.as_str(), "weather" | "counter_add" | "counter_subtract" | "telegram_open") && !played_before
                    && !voices::play_command_reply(&cmd_config.id, &language) {
                    voices::play_random_from(cmd_config.get_sounds(&language).as_slice());
                }
                ipc::send(IpcEvent::CommandExecuted {
                    id: cmd_config.id.clone(),
                    success: true,
                });
                ipc::send(IpcEvent::Idle);
                return chain || DIALOGUE_MODE.load(Ordering::SeqCst);
            }
            Err(msg) => {
                error!("Error executing command: {}", msg);
                voices::play_error();
                ipc::send(IpcEvent::CommandExecuted {
                    id: cmd_config.id.clone(),
                    success: false,
                });
                ipc::send(IpcEvent::Error { message: msg.to_string() });
            }
        }
    } else {
        info!("No command found for: {}", text);
        voices::play_not_found();
        ipc::send(IpcEvent::Error { 
            message: format!("Command not found: {}", text) 
        });
    }
    
    ipc::send(IpcEvent::Idle);
    DIALOGUE_MODE.load(Ordering::SeqCst)
}


pub fn close(code: i32) {
    info!("Closing application.");
    voices::play_goodbye();
    ipc::send(IpcEvent::Stopping);
    std::process::exit(code);
}

fn remove_assistant_phrases(mut text: String) -> String {
    // Strip only the wake word. Verbs such as "давай" and "покажи" are part
    // of real commands and must survive recognition.
    for phrase in config::get_wake_phrases(&i18n::get_language()) {
        if text == *phrase {
            return String::new();
        }

        if let Some(rest) = text.strip_prefix(phrase) {
            if rest.starts_with(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ':' | '!' | '.' | '،')) {
                text = rest.trim_start_matches(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ':' | '!' | '.' | '،')).to_string();
            }
        }
    }
    text
}
