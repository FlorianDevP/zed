use settings::{ClockLocation, ClockSettingsContent, RegisterSetting, Settings, SettingsContent};

#[derive(Debug, Clone, RegisterSetting)]
pub struct ClockSettings {
    pub show: bool,
    pub position: ClockLocation,
    pub use_12_hour_clock: bool,
    pub offset: String,
}

impl Settings for ClockSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let clock: &ClockSettingsContent = content.clock.as_ref().unwrap();
        ClockSettings {
            show: clock.show.unwrap(),
            position: clock.position.unwrap(),
            use_12_hour_clock: clock.use_12_hour_clock.unwrap(),
            offset: clock.offset.as_ref().unwrap().clone(),
        }
    }
}
