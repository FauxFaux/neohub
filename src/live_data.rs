use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Timestamp(i64);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Header {
    pub hub_away: bool,
    pub hub_time: Timestamp,
    pub hub_holiday: bool,
    pub holiday_end: Timestamp,

    pub timestamp_device_lists: Timestamp,
    pub timestamp_engineers: Timestamp,
    pub timestamp_profile_0: Timestamp,
    pub timestamp_profile_comfort_levels: Timestamp,
    pub timestamp_profile_timers: Timestamp,
    pub timestamp_profile_timers_0: Timestamp,
    pub timestamp_recipes: Timestamp,

    pub cool_input: bool,
    pub close_delay: i64,
    pub open_delay: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Device {
    pub zone_name: String,

    pub active_level: i64,
    pub active_profile: i64,
    pub device_id: i64,

    pub available_modes: Vec<String>,

    pub away: bool,
    pub holiday: bool,

    pub cool_mode: bool,
    pub cool_on: bool,
    pub cool_temp: f64,

    pub actual_temp: String,
    pub current_floor_temperature: f64,
    pub prg_temp: i64,
    pub recent_temps: Vec<String>,
    pub relative_humidity: i64,
    pub set_temp: String,

    // day of week
    pub date: String,
    pub time: String,

    pub fan_control: String,
    pub fan_speed: String,
    pub floor_limit: bool,
    pub hc_mode: String,
    pub heat_mode: bool,
    pub heat_on: bool,

    pub hold_cool: f64,
    pub hold_off: bool,
    pub hold_on: bool,
    pub hold_temp: f64,
    pub hold_time: String,

    pub lock: bool,
    pub low_battery: bool,
    pub manual_off: bool,
    pub modelock: bool,
    pub modulation_level: i64,
    pub offline: bool,
    pub pin_number: String,
    pub preheat_active: bool,
    pub prg_timer: bool,
    pub standby: bool,
    pub switch_delay_left: String,
    pub temporary_set_flag: bool,
    pub timer_on: bool,
    pub window_open: bool,

    pub thermostat: Option<bool>,

    pub write_count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct LiveData {
    #[serde(flatten)]
    header: Header,
    pub devices: Vec<Device>,
}
