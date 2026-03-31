use crate::config::RuneAlertConfig;
use lazy_static::lazy_static;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuneAlertSettings {
    pub enabled: bool,
    pub alert_lead_seconds: i32,
    pub interval_seconds: i32,
    pub audio_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuneAlert {
    pub rune_time_seconds: i32,
    pub seconds_until_rune: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuneAlertSnapshot {
    pub enabled: bool,
    pub next_rune_time_seconds: Option<i32>,
    pub seconds_until_next_rune: Option<i32>,
    pub last_alerted_rune_time_seconds: Option<i32>,
    pub last_alert_clock_time_seconds: Option<i32>,
    pub audio_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct RuneAlertManager {
    settings: RuneAlertSettings,
    last_alerted_rune_time_seconds: Option<i32>,
    last_alert_clock_time_seconds: Option<i32>,
}

impl From<&RuneAlertConfig> for RuneAlertSettings {
    fn from(value: &RuneAlertConfig) -> Self {
        Self {
            enabled: value.enabled,
            alert_lead_seconds: value.alert_lead_seconds,
            interval_seconds: value.interval_seconds,
            audio_enabled: value.audio_enabled,
        }
    }
}

impl RuneAlertManager {
    pub fn new(settings: RuneAlertSettings) -> Self {
        Self {
            settings,
            last_alerted_rune_time_seconds: None,
            last_alert_clock_time_seconds: None,
        }
    }

    pub fn update_settings(&mut self, settings: RuneAlertSettings) {
        self.settings = settings;
    }

    pub fn update(&mut self, clock_time_seconds: i32) -> Option<RuneAlert> {
        let next_rune_time_seconds = self.next_rune_time_seconds(clock_time_seconds)?;
        let seconds_until_rune = next_rune_time_seconds - clock_time_seconds;

        if !self.settings.enabled
            || seconds_until_rune > self.settings.alert_lead_seconds
            || seconds_until_rune < 0
            || self.last_alerted_rune_time_seconds == Some(next_rune_time_seconds)
        {
            return None;
        }

        self.last_alerted_rune_time_seconds = Some(next_rune_time_seconds);
        self.last_alert_clock_time_seconds = Some(clock_time_seconds);

        Some(RuneAlert {
            rune_time_seconds: next_rune_time_seconds,
            seconds_until_rune,
        })
    }

    pub fn snapshot(&self, clock_time_seconds: i32) -> RuneAlertSnapshot {
        let next_rune_time_seconds = self.next_rune_time_seconds(clock_time_seconds);
        RuneAlertSnapshot {
            enabled: self.settings.enabled,
            next_rune_time_seconds,
            seconds_until_next_rune: next_rune_time_seconds.map(|rune_time| rune_time - clock_time_seconds),
            last_alerted_rune_time_seconds: self.last_alerted_rune_time_seconds,
            last_alert_clock_time_seconds: self.last_alert_clock_time_seconds,
            audio_enabled: self.settings.audio_enabled,
        }
    }

    fn next_rune_time_seconds(&self, clock_time_seconds: i32) -> Option<i32> {
        if self.settings.interval_seconds <= 0 {
            return None;
        }

        if clock_time_seconds < 0 {
            return Some(self.settings.interval_seconds);
        }

        let remainder = clock_time_seconds % self.settings.interval_seconds;
        if remainder == 0 {
            Some(clock_time_seconds + self.settings.interval_seconds)
        } else {
            Some(clock_time_seconds + (self.settings.interval_seconds - remainder))
        }
    }
}

lazy_static! {
    static ref RUNE_ALERT_MANAGER: Mutex<RuneAlertManager> =
        Mutex::new(RuneAlertManager::new(RuneAlertSettings {
            enabled: true,
            alert_lead_seconds: 10,
            interval_seconds: 120,
            audio_enabled: true,
        }));
    static ref LATEST_RUNE_ALERT_SNAPSHOT: Mutex<Option<RuneAlertSnapshot>> = Mutex::new(None);
}

type SoundHook = fn();

fn default_sound_hook() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[console]::beep(880,180)",
            ])
            .spawn();
    }
}

fn sound_hook_cell() -> &'static Mutex<SoundHook> {
    static SOUND_HOOK: OnceLock<Mutex<SoundHook>> = OnceLock::new();
    SOUND_HOOK.get_or_init(|| Mutex::new(default_sound_hook))
}

pub fn process_clock_time(clock_time_seconds: i32, config: &RuneAlertConfig) -> RuneAlertSnapshot {
    let mut manager = RUNE_ALERT_MANAGER.lock().unwrap();
    manager.update_settings(RuneAlertSettings::from(config));
    let alert = manager.update(clock_time_seconds);
    let snapshot = manager.snapshot(clock_time_seconds);
    drop(manager);

    if alert.is_some() && config.audio_enabled {
        let hook = *sound_hook_cell().lock().unwrap();
        hook();
    }

    *LATEST_RUNE_ALERT_SNAPSHOT.lock().unwrap() = Some(snapshot.clone());
    snapshot
}

#[cfg(test)]
pub fn latest_rune_alert_snapshot() -> Option<RuneAlertSnapshot> {
    LATEST_RUNE_ALERT_SNAPSHOT.lock().unwrap().clone()
}

#[cfg(test)]
pub fn reset_rune_alert_state_for_tests() {
    *RUNE_ALERT_MANAGER.lock().unwrap() = RuneAlertManager::new(RuneAlertSettings {
        enabled: true,
        alert_lead_seconds: 10,
        interval_seconds: 120,
        audio_enabled: true,
    });
    *LATEST_RUNE_ALERT_SNAPSHOT.lock().unwrap() = None;
    set_sound_hook_for_tests(default_sound_hook);
}

#[cfg(test)]
pub fn set_sound_hook_for_tests(hook: SoundHook) {
    *sound_hook_cell().lock().unwrap() = hook;
}
