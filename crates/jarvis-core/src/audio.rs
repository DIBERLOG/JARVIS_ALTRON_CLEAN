mod kira;
mod rodio;

use once_cell::sync::OnceCell;
use std::path::PathBuf;

use crate::config::structs::AudioType;
use crate::{config, DB, SOUND_DIR};

static AUDIO_TYPE: OnceCell<AudioType> = OnceCell::new();

pub fn init() -> Result<(), ()> {
    if AUDIO_TYPE.get().is_some() {
        return Ok(());
    } // already initialized

    // set default audio type
    // @TODO. Make it configurable?
    AUDIO_TYPE.set(config::DEFAULT_AUDIO_TYPE).unwrap();

    // load given audio backend
    match AUDIO_TYPE.get().unwrap() {
        AudioType::Rodio => {
            // Init Rodio
            info!("Initializing Rodio audio backend.");

            match rodio::init() {
                Ok(_) => {
                    info!("Successfully initialized Rodio audio backend.");
                }
                Err(()) => {
                    error!("Failed to initialize Rodio audio backend.");

                    return Err(());
                }
            }
        }
        AudioType::Kira => {
            // Init Kira
            info!("Initializing Kira audio backend.");

            match kira::init() {
                Ok(_) => {
                    info!("Successfully initialized Kira audio backend.");
                }
                Err(_msg) => {
                    error!("Failed to initialize Kira audio backend.");

                    return Err(());
                }
            }
        }
    }

    Ok(())
}

pub fn play_sound(filename: &PathBuf) {
    let audio_type = match AUDIO_TYPE.get() {
        Some(t) => t,
        None => {
            warn!("Audio not initialized, cannot play: {}", filename.display());
            return;
        }
    };
    
    info!("Playing {}", filename.display());

    match audio_type {
        AudioType::Rodio => {
            rodio::play_sound(filename, true);
        }
        AudioType::Kira => kira::play_sound(filename),
    }
}

pub fn play_sound_blocking(filename: &PathBuf) -> bool {
    match AUDIO_TYPE.get() {
        Some(AudioType::Rodio) => {
            rodio::play_sound(filename, false);
            true
        }
        Some(AudioType::Kira) => kira::play_sound_blocking(filename),
        None => {
            warn!("Audio not initialized, cannot play: {}", filename.display());
            false
        }
    }
}

pub fn stop() {
    match AUDIO_TYPE.get() {
        Some(AudioType::Kira) => kira::stop(),
        Some(AudioType::Rodio) => rodio::stop(),
        None => {}
    }
}

pub fn is_playing() -> bool {
    match AUDIO_TYPE.get() {
        Some(AudioType::Kira) => kira::is_playing(),
        Some(AudioType::Rodio) => rodio::is_playing(),
        None => false
    }
}

pub fn play_sound_cancellable(filename: &PathBuf, cancelled: impl Fn() -> bool) -> bool {
    if cancelled() { return false; }
    match AUDIO_TYPE.get() {
        Some(AudioType::Kira) => kira::play_sound_cancellable(filename, cancelled),
        Some(AudioType::Rodio) => {
            rodio::play_sound(filename, false);
            while rodio::is_playing() {
                if cancelled() { rodio::stop(); return false; }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            true
        }
        None => false
    }
}

pub fn get_sound_directory() -> Option<PathBuf> {
    let db = DB.get()?;

    let voice_path = {
        let s = db.read();
        SOUND_DIR.join(&s.voice)
    };

    match voice_path.exists() {
        true => Some(voice_path),
        _ => {
            error!("No sounds folder found. Search path - {:?}", voice_path);
            None
        }
    }
}
