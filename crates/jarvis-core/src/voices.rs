use std::fs;
use std::path::{Path, PathBuf};
use rand::prelude::*;
use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
// use chrono::Timelike;

use crate::{DB, SOUND_DIR, audio, config, time};

mod structs;
pub use structs::*;

static VOICES: OnceCell<Vec<structs::VoiceConfig>> = OnceCell::new();
static CURRENT_VOICE_ID: OnceCell<RwLock<String>> = OnceCell::new();
static LAST_JOKE_REPLY: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(default_voice: &str, language: &str) -> Result<(), String> {
    let voices = scan_voices()?;
    
    if voices.is_empty() {
        return Err("No voices found".into());
    }
    
    info!("Loaded {} voice(s): {:?}", 
        voices.len(), 
        voices.iter().map(|v| &v.voice.id).collect::<Vec<_>>()
    );

    // resolve which voice to use
    let voice_id = if !default_voice.is_empty() && voices.iter().any(|v| v.voice.id == default_voice) {
        default_voice.to_string()
    } else {
        // auto-detect: pick the first voice that supports the active language
        let auto = voices.iter()
            .find(|v| v.voice.languages.contains(&language.to_string()))
            .or_else(|| voices.first());

        match auto {
            Some(v) => {
                if default_voice.is_empty() {
                    info!("No voice configured, auto-selected '{}' for language '{}'", v.voice.id, language);
                } else {
                    warn!("Voice '{}' not found, auto-selected '{}'", default_voice, v.voice.id);
                }
                v.voice.id.clone()
            }
            None => return Err("No compatible voice found".into()),
        }
    };

    CURRENT_VOICE_ID.get_or_init(|| RwLock::new(voice_id));
    VOICES.set(voices).map_err(|_| "Voices already initialized")?;
    
    Ok(())
}

pub fn scan_voices() -> Result<Vec<structs::VoiceConfig>, String> {
    let voices_dir = SOUND_DIR.join(&config::VOICES_PATH);
    
    if !voices_dir.exists() {
        return Err(format!("Voices directory not found: {:?}", voices_dir));
    }
    
    let mut voices = Vec::new();
    
    let entries = fs::read_dir(&voices_dir)
        .map_err(|e| format!("Failed to read voices directory: {}", e))?;
    
    for entry in entries.flatten() {
        let voice_path = entry.path();
        if !voice_path.is_dir() {
            continue;
        }
        
        let toml_path = voice_path.join("voice.toml");
        if !toml_path.exists() {
            warn!("Voice folder {:?} missing voice.toml, skipping", voice_path);
            continue;
        }
        
        match load_voice_config(&toml_path, &voice_path) {
            Ok(config) => voices.push(config),
            Err(e) => warn!("Failed to load voice {:?}: {}", voice_path, e),
        }
    }
    
    Ok(voices)
}

fn load_voice_config(toml_path: &Path, voice_path: &Path) -> Result<structs::VoiceConfig, String> {
    let content = fs::read_to_string(toml_path)
        .map_err(|e| format!("Failed to read voice.toml: {}", e))?;
    
    let mut config: structs::VoiceConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse voice.toml: {}", e))?;
    
    config.path = voice_path.to_path_buf();
    
    Ok(config)
}



pub fn list_voices() -> &'static [structs::VoiceConfig] {
    VOICES.get().map(|v| v.as_slice()).unwrap_or(&[])
}

pub fn get_voice(voice_id: &str) -> Option<&'static structs::VoiceConfig> {
    VOICES.get()?.iter().find(|v| v.voice.id == voice_id)
}

pub fn get_current_voice() -> Option<&'static structs::VoiceConfig> {
    let current_id = CURRENT_VOICE_ID.get()?.read().clone();
    get_voice(&current_id)
}

pub fn set_current_voice(voice_id: &str) {
    if let Some(lock) = CURRENT_VOICE_ID.get() {
        *lock.write() = voice_id.to_string();
    }
}

fn get_current_language() -> String {
    DB.get()
        .map(|db| db.read().language.clone())
        .unwrap_or_else(|| "ru".to_string())
}



fn find_sound_file(voice_path: &Path, lang: &str, sound_name: &str) -> Option<PathBuf> {
    const EXTENSIONS: &[&str] = &["mp3", "wav", "ogg"];
    let lang_path = voice_path.join(lang);
    
    // try language subfolder first (/en, /ua, /ru, etc)
    for ext in EXTENSIONS {
        let file_path = lang_path.join(format!("{}.{}", sound_name, ext));
        if file_path.exists() {
            return Some(file_path);
        }
    }
    
    // fallback to root voice folder
    for ext in EXTENSIONS {
        let file_path = voice_path.join(format!("{}.{}", sound_name, ext));
        if file_path.exists() {
            return Some(file_path);
        }
    }
    
    None
}

fn play_random_from_list(voice_path: &Path, lang: &str, sounds: &[String]) -> bool {
    if sounds.is_empty() {
        return false;
    }
    
    let available: Vec<PathBuf> = sounds.iter()
        .filter_map(|name| find_sound_file(voice_path, lang, name))
        .collect();

    if let Some(path) = available.choose(&mut rand::thread_rng()) {
        debug!("Playing: {:?}", path);
        audio::play_sound(path);
        true
    } else {
        warn!("No available sounds for {:?} (lang: {})", sounds, lang);
        false
    }
}

fn greeting_sounds(reactions: &structs::VoiceReactions, period: time::TimeOfDay) -> &[String] {
    match period {
        time::TimeOfDay::Morning => &reactions.greet_morning,
        time::TimeOfDay::Day => &reactions.greet_day,
        time::TimeOfDay::Evening => &reactions.greet_evening,
        time::TimeOfDay::Night => &reactions.greet_night,
    }
}

pub fn play(reaction: structs::Reaction) {
    let voice = match get_current_voice() {
        Some(v) => v,
        None => {
            warn!("No current voice set");
            return;
        }
    };
    
    let lang = get_current_language();
    
    let reactions = match voice.reactions.get(&lang) {
        Some(r) => r,
        None => {
            warn!("No reactions for language: {}", lang);
            return;
        }
    };

    let sounds = match reaction {
        structs::Reaction::Greet => {
            let period = time::TimeOfDay::now();
            if play_random_from_list(&voice.path, &lang, greeting_sounds(reactions, period)) {
                return;
            }

            // Older voice packs have only a generic startup clip. Use a recorded
            // time-aware greeting from the remaster pack before that generic clip.
            if voice.voice.id != "jarvis-remaster" {
                if let Some(remaster) = get_voice("jarvis-remaster") {
                    if let Some(remaster_reactions) = remaster.reactions.get(&lang) {
                        if play_random_from_list(&remaster.path, &lang, greeting_sounds(remaster_reactions, period)) {
                            return;
                        }
                    }
                }
            }
            &reactions.greet
        }
        structs::Reaction::Reply => &reactions.reply,
        structs::Reaction::Ok => &reactions.ok,
        structs::Reaction::NotFound => &reactions.not_found,
        structs::Reaction::Thanks => &reactions.thanks,
        structs::Reaction::Error => &reactions.error,
        structs::Reaction::Goodbye => &reactions.goodbye,
    };
    
    play_random_from_list(&voice.path, &lang, sounds);
}

pub fn play_random_from(sounds: &[String]) {
    let voice = match get_current_voice() {
        Some(v) => v,
        None => {
            warn!("No current voice set");
            return;
        }
    };
    
    let lang = get_current_language();
    if play_random_from_list(&voice.path, &lang, sounds) {
        return;
    }

    // Some voice packs have fewer reactions (for example, no joke clips).
    // Use the pack with the most matching clips when the selected pack has none.
    if let Some(fallback) = list_voices().iter()
        .filter(|candidate| candidate.voice.id != voice.voice.id)
        .max_by_key(|candidate| sounds.iter()
            .filter(|name| find_sound_file(&candidate.path, &lang, name).is_some())
            .count()) {
        play_random_from_list(&fallback.path, &lang, sounds);
    }
}

/// Command replies are independent of the selected legacy voice pack.
/// Missing clips fall back to the command's configured sounds.
pub fn play_command_reply(command_id: &str, language: &str) -> bool {
    if !command_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return false;
    }
    let reply_dir = SOUND_DIR.join("command-replies").join(language);
    if command_id == "jarvis_joke" {
        let mut jokes: Vec<PathBuf> = fs::read_dir(&reply_dir).ok().into_iter().flatten()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "mp3"))
            .filter(|path| path.file_stem().and_then(|stem| stem.to_str()).is_some_and(|stem| {
                stem == "jarvis_joke" || stem.strip_prefix("jarvis_joke_")
                    .is_some_and(|number| number.parse::<u32>().is_ok())
            }))
            .collect();
        jokes.sort();
        let selected = pick_without_repeat(&jokes, LAST_JOKE_REPLY.lock().as_ref());
        if let Some(path) = selected {
            if audio::play_sound_blocking(&path) {
                *LAST_JOKE_REPLY.lock() = Some(path);
                return true;
            }
        }
        return false;
    }
    let path = reply_dir.join(format!("{command_id}.mp3"));
    if !path.is_file() { return false; }
    audio::play_sound_blocking(&path)
}

fn pick_without_repeat(paths: &[PathBuf], previous: Option<&PathBuf>) -> Option<PathBuf> {
    let choices: Vec<&PathBuf> = paths.iter()
        .filter(|path| paths.len() == 1 || Some(*path) != previous)
        .collect();
    choices.choose(&mut rand::thread_rng()).map(|path| (*path).clone())
}

#[cfg(test)]
mod command_reply_tests {
    use super::pick_without_repeat;
    use std::path::PathBuf;

    #[test]
    fn joke_does_not_repeat_when_alternatives_exist() {
        let paths = vec![PathBuf::from("one.mp3"), PathBuf::from("two.mp3"), PathBuf::from("three.mp3")];
        for previous in &paths {
            for _ in 0..20 {
                assert_ne!(pick_without_repeat(&paths, Some(previous)).as_ref(), Some(previous));
            }
        }
    }
}

// Play a preview sound for a specific voice
pub fn play_preview(voice_id: &str) {
    let voice = match get_voice(voice_id) {
        Some(v) => v,
        None => {
            warn!("Voice not found for preview: {}", voice_id);
            return;
        }
    };
    
    let lang = get_current_language();
    
    let reactions = match voice.reactions.get(&lang) {
        Some(r) => r,
        None => {
            warn!("No reactions for language {} in voice {}", lang, voice_id);
            return;
        }
    };
    
    // pick from reply, ok, or greet sounds for preview
    let sounds: Vec<&String> = reactions.reply.iter()
        .chain(reactions.ok.iter())
        .chain(reactions.greet.iter())
        .collect();
    
    if sounds.is_empty() {
        warn!("No preview sounds for voice: {}", voice_id);
        return;
    }
    
    let sound_name = sounds.choose(&mut rand::thread_rng()).unwrap();
    
    if let Some(path) = find_sound_file(&voice.path, &lang, sound_name) {
        debug!("Playing preview: {:?}", path);
        audio::play_sound(&path);
    }
}


// shortcuts
pub fn play_greet() { play(structs::Reaction::Greet); } // app startup
pub fn play_reply() { play(structs::Reaction::Reply); } // wake word detected
pub fn play_ok() { play(structs::Reaction::Ok); } // command executed
pub fn play_not_found() { play(structs::Reaction::NotFound); }
pub fn play_thanks() { play(structs::Reaction::Thanks); }
pub fn play_error() { play(structs::Reaction::Error); }
pub fn play_goodbye() { play(structs::Reaction::Goodbye); }
