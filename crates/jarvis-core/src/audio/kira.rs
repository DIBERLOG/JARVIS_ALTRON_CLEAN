use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::Mutex;

// use kira::{
//     manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
//     sound::static_sound::{StaticSoundData, StaticSoundSettings},
// };

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::{static_sound::{StaticSoundData, StaticSoundHandle}, PlaybackState}, Tween,
};

static MANAGER: OnceCell<Mutex<AudioManager>> = OnceCell::new();
static PLAYING: Mutex<Vec<StaticSoundHandle>> = Mutex::new(Vec::new());
static OUTPUT_MODE: Mutex<String> = Mutex::new(String::new());

pub fn stop() {
    if let Ok(mut playing) = PLAYING.lock() {
        for handle in playing.iter_mut() { handle.stop(Tween { duration: std::time::Duration::from_millis(40), ..Default::default() }); }
    }
}

pub fn is_playing() -> bool {
    if let Ok(mut playing) = PLAYING.lock() {
        playing.retain(|handle| handle.state() != PlaybackState::Stopped);
        return !playing.is_empty();
    }
    false
}

pub fn init() -> Result<(), ()> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let mode=crate::db::latest_settings().map(|s|s.audio_output_mode).unwrap_or_else(||"direct".into());
    let mut current=OUTPUT_MODE.lock().map_err(|_|())?;
    if MANAGER.get().is_some() && *current==mode {
        return Ok(());
    }  // already initialized
    let host=cpal::default_host();
    let virtual_device=|name:&str| {let name=name.to_lowercase();["cable","voicemod","virtual"].iter().any(|word|name.contains(word))};
    let device=if mode=="voicemod" {
        host.output_devices().ok().and_then(|mut devices|devices.find(|d|d.name().is_ok_and(|n|n.to_lowercase().contains("cable input"))))
    } else {
        host.default_output_device().filter(|d|d.name().is_ok_and(|n|!virtual_device(&n))).or_else(|| {
            let devices:Vec<_>=host.output_devices().ok()?.filter(|d|d.name().is_ok_and(|n|!virtual_device(&n))).collect();
            for word in ["jbl","headphone","наушник","speaker","динамик"] {
                if let Some(device)=devices.iter().find(|d|d.name().is_ok_and(|n|n.to_lowercase().contains(word))) {return Some(device.clone())}
            }
            devices.into_iter().next()
        })
    };
    let Some(device)=device else {warn!("No audio output available for mode {mode}");return Err(())};
    info!("JARVIS output ({mode}): {}",device.name().unwrap_or_default());
    let settings=AudioManagerSettings {backend_settings:kira::backend::cpal::CpalBackendSettings {device:Some(device),..Default::default()},..Default::default()};

    // Create an audio manager. This plays sounds and manages resources.
    match AudioManager::<DefaultBackend>::new(settings) {
        Ok(manager) => {
            // store
            if let Some(existing)=MANAGER.get() {*existing.lock().map_err(|_|())?=manager;} else {MANAGER.set(Mutex::new(manager)).ok();}
            *current=mode;

            // success
            Ok(())
        }
        Err(msg) => {
            error!("Failed to initialize audio stream.\nError details: {}", msg);

            // failed
            Err(())
        }
    }
}

// @TODO. Cache sounds in memory? With a pool of a certain size, for instance.
pub fn play_sound(filename: &PathBuf) {
    if init().is_err() {return;}
    // load the file
    match StaticSoundData::from_file(filename) {
        Ok(sound_data) => {
            // sound_data.duration() can be used in order to sleep, if (for some reason) blocking behaviour is required

            // play it (non-blocking)
            if let Some(manager) = MANAGER.get() {
                if let Ok(mut audio_manager) = manager.lock() {
                    match audio_manager.play(sound_data) {
                        Ok(handle) => if let Ok(mut playing) = PLAYING.lock() { playing.retain(|sound| sound.state() != PlaybackState::Stopped); playing.push(handle); },
                        Err(e) => warn!("Failed to play sound: {}", e),
                    }
                }
            } else {
                warn!("Audio manager not initialized");
            }
        }
        Err(err) => {
            warn!("Cannot find sound file: {} (err: {})", filename.display(), err);
        }
    }
}

pub fn play_sound_blocking(filename: &PathBuf) -> bool {
    play_sound_cancellable(filename, || false)
}

pub fn play_sound_cancellable(filename: &PathBuf, cancelled: impl Fn() -> bool) -> bool {
    if init().is_err() {return false;}
    let sound_data = match StaticSoundData::from_file(filename) {
        Ok(data) => data,
        Err(err) => {
            warn!("Cannot load command reply: {} (err: {})", filename.display(), err);
            return false;
        }
    };
    let duration = sound_data.duration();
    let Some(manager) = MANAGER.get() else { return false; };
    let Ok(mut audio_manager) = manager.lock() else { return false; };
    if cancelled() { return false; }
    let handle = match audio_manager.play(sound_data) {
        Ok(handle) => handle,
        Err(err) => { warn!("Failed to play command reply: {}", err); return false; }
    };
    let started = std::time::Instant::now();
    if let Ok(mut playing) = PLAYING.lock() { playing.retain(|sound| sound.state() != PlaybackState::Stopped); playing.push(handle); }
    drop(audio_manager);
    while started.elapsed() < duration + std::time::Duration::from_millis(200) && is_playing() {
        if cancelled() { stop(); return false; }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    true
}
