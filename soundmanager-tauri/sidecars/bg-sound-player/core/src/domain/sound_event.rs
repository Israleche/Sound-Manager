//! Static table of all sound events. Port of SoundManager/SoundEvent.cs.
//! Internal names MUST stay stable: they are used in sound archives,
//! translations and disabled-event settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Startup,
    Shutdown,
    Logon,
    Logoff,
    LoadScheme,
    Select,
}

/// One system sound event handled by the app.
#[derive(Debug, Clone, Serialize)]
pub struct SoundEvent {
    pub internal_name: &'static str,
    /// Registry subkeys relative to AppEvents\Schemes\Apps, e.g. ".Default\\SystemStart"
    pub registry_keys: &'static [&'static str],
    /// Legacy file name from old (XP FR) sound archive format, if any.
    pub legacy_file_name: Option<&'static str>,
    pub event_type: Option<EventType>,
}

impl SoundEvent {
    /// Canonical wav file name inside the media dir, e.g. "Startup.wav".
    pub fn file_name(&self) -> String {
        format!("{}.wav", self.internal_name)
    }

    /// Full path inside the media dir.
    pub fn file_path(&self, media_dir: &std::path::Path) -> PathBuf {
        media_dir.join(self.file_name())
    }

    /// Legacy file name from the old archive format.
    pub fn legacy_name(&self) -> String {
        // Legacy archives used "Windows XP <FR name>.wav"
        let fr = self.legacy_file_name.unwrap_or("");
        format!("Windows XP {fr}.wav")
    }
}

pub const ALL_EVENTS: &[SoundEvent] = &[
    // ======================================================================================================
    //  internal name       registry key(s)                                                   legacy (XP FR)
    // ======================================================================================================
    SoundEvent { internal_name: "Startup",          registry_keys: &[".Default\\SystemStart"],        legacy_file_name: Some("Démarrage"),             event_type: Some(EventType::Startup) },
    SoundEvent { internal_name: "Shutdown",         registry_keys: &[".Default\\SystemExit"],         legacy_file_name: Some("Arrêt du système"),      event_type: Some(EventType::Shutdown) },
    SoundEvent { internal_name: "Logon",            registry_keys: &[".Default\\WindowsLogon"],       legacy_file_name: Some("Ouverture de session"),  event_type: Some(EventType::Logon) },
    SoundEvent { internal_name: "Logoff",           registry_keys: &[".Default\\WindowsLogoff"],      legacy_file_name: Some("Fermeture de session"),  event_type: Some(EventType::Logoff) },
    SoundEvent { internal_name: "Information",      registry_keys: &[".Default\\SystemAsterisk"],     legacy_file_name: Some("Erreur"),                event_type: None },
    SoundEvent { internal_name: "Question",         registry_keys: &[".Default\\SystemQuestion"],     legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "Warning",          registry_keys: &[".Default\\SystemExclamation"],  legacy_file_name: Some("Exclamation"),           event_type: None },
    SoundEvent { internal_name: "Error",            registry_keys: &[".Default\\SystemHand"],         legacy_file_name: Some("Arrêt critique"),        event_type: None },
    SoundEvent { internal_name: "DeviceConnect",    registry_keys: &[".Default\\DeviceConnect"],      legacy_file_name: Some("Insertion d'un matériel"), event_type: None },
    SoundEvent { internal_name: "DeviceDisconnect", registry_keys: &[".Default\\DeviceDisconnect"],   legacy_file_name: Some("Suppression d'un matériel"), event_type: None },
    SoundEvent { internal_name: "DeviceFail",       registry_keys: &[".Default\\DeviceFail"],         legacy_file_name: Some("Échec d'un matériel"),   event_type: None },
    SoundEvent { internal_name: "Default",          registry_keys: &[".Default\\.Default"],           legacy_file_name: Some("Ding"),                  event_type: None },
    // Windows Vista..8 balloon read from Explorer instead of .Default; Win10 uses Notification.Default
    SoundEvent { internal_name: "Balloon",          registry_keys: &[".Default\\SystemNotification", ".Default\\Notification.Default", "Explorer\\SystemNotification"], legacy_file_name: Some("Infobulle"), event_type: None },
    SoundEvent { internal_name: "Navigate",         registry_keys: &["Explorer\\Navigating"],         legacy_file_name: Some("Menu Démarrer"),         event_type: None },
    SoundEvent { internal_name: "RecycleBin",       registry_keys: &["Explorer\\EmptyRecycleBin"],    legacy_file_name: Some("Corbeille"),             event_type: None },
    SoundEvent { internal_name: "UAC",              registry_keys: &[".Default\\WindowsUAC"],         legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "BatteryLow",       registry_keys: &[".Default\\LowBatteryAlarm"],    legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "BatteryCritical",  registry_keys: &[".Default\\CriticalBatteryAlarm"], legacy_file_name: None,                        event_type: None },
    SoundEvent { internal_name: "Email",            registry_keys: &[".Default\\MailBeep", ".Default\\Notification.Mail"], legacy_file_name: None,     event_type: None },
    SoundEvent { internal_name: "Reminder",         registry_keys: &[".Default\\Notification.Reminder"], legacy_file_name: None,                       event_type: None },
    SoundEvent { internal_name: "Print",            registry_keys: &[".Default\\PrintComplete"],      legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "AppOpen",          registry_keys: &[".Default\\Open"],               legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "AppClose",         registry_keys: &[".Default\\Close"],              legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "Minimize",         registry_keys: &[".Default\\Minimize"],           legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "UnMinimize",       registry_keys: &[".Default\\RestoreUp"],          legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "Maximize",         registry_keys: &[".Default\\Maximize"],           legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "UnMaximize",       registry_keys: &[".Default\\RestoreDown"],        legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "Menu",             registry_keys: &[".Default\\MenuPopup"],          legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "MenuCommand",      registry_keys: &[".Default\\MenuCommand"],        legacy_file_name: None,                          event_type: None },
    SoundEvent { internal_name: "Select",           registry_keys: &[".Default\\CCSelect"],           legacy_file_name: None,                          event_type: Some(EventType::Select) },
    SoundEvent { internal_name: "LoadScheme",       registry_keys: &[".Default\\ChangeTheme"],        legacy_file_name: None,                          event_type: Some(EventType::LoadScheme) },
];

/// Look up an event by its internal name.
pub fn event_by_name(name: &str) -> Option<&'static SoundEvent> {
    ALL_EVENTS.iter().find(|e| e.internal_name == name)
}

/// Look up an event by its special type.
pub fn event_by_type(t: EventType) -> &'static SoundEvent {
    ALL_EVENTS.iter().find(|e| e.event_type == Some(t)).expect("all EventTypes must exist in ALL_EVENTS")
}

/// Default media directory: %APPDATA%\SoundManager\Media
pub fn default_media_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("SoundManager").join("Media")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_events_unique() {
        let mut names: Vec<_> = ALL_EVENTS.iter().map(|e| e.internal_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ALL_EVENTS.len());
    }

    #[test]
    fn lookups_work() {
        assert_eq!(event_by_name("Startup").unwrap().internal_name, "Startup");
        assert!(event_by_name("Nope").is_none());
        assert_eq!(event_by_type(EventType::Shutdown).internal_name, "Shutdown");
    }
}
