use crate::migration::nodes::control::evaluate_control_points;
use crate::migration::protocol::{ControlPoint, MixerInfo, MixerSlotInfo, NodeInfo, State};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerPipelineStage {
    Idle,
    Starting,
    Playing,
}

#[derive(Debug, Clone)]
pub struct MixerPipelineProfile {
    pub video_branch_elements: Vec<String>,
    pub audio_branch_elements: Vec<String>,
    pub width: i64,
    pub height: i64,
    pub sample_rate: i64,
    pub fallback_image: String,
    pub fallback_timeout_ms: i64,
    pub stage: MixerPipelineStage,
}

impl MixerPipelineProfile {
    fn from_settings(
        settings: &HashMap<String, Value>,
        audio_enabled: bool,
        video_enabled: bool,
        stage: MixerPipelineStage,
    ) -> Self {
        let width = settings
            .get("width")
            .and_then(Value::as_i64)
            .or_else(|| settings.get("width").and_then(Value::as_f64).map(|v| v as i64))
            .unwrap_or(1920);
        let height = settings
            .get("height")
            .and_then(Value::as_i64)
            .or_else(|| settings.get("height").and_then(Value::as_f64).map(|v| v as i64))
            .unwrap_or(1080);
        let sample_rate = settings
            .get("sample-rate")
            .and_then(Value::as_i64)
            .or_else(|| settings.get("sample-rate").and_then(Value::as_f64).map(|v| v as i64))
            .unwrap_or(48000);
        let fallback_timeout_ms = settings
            .get("fallback-timeout")
            .and_then(Value::as_i64)
            .or_else(|| {
                settings
                    .get("fallback-timeout")
                    .and_then(Value::as_f64)
                    .map(|v| v as i64)
            })
            .unwrap_or(500);
        let fallback_image = settings
            .get("fallback-image")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let video_branch_elements = if video_enabled {
            vec![
                "compositor".to_string(),
                "capsfilter".to_string(),
                "queue".to_string(),
                "appsink".to_string(),
            ]
        } else {
            Vec::new()
        };

        let audio_branch_elements = if audio_enabled {
            vec![
                "audiomixer".to_string(),
                "audioconvert".to_string(),
                "audioresample".to_string(),
                "appsink".to_string(),
            ]
        } else {
            Vec::new()
        };

        Self {
            video_branch_elements,
            audio_branch_elements,
            width,
            height,
            sample_rate,
            fallback_image,
            fallback_timeout_ms,
            stage,
        }
    }
}

fn parse_slot_config_key(property: &str) -> Result<(bool, &str), String> {
    let split: Vec<&str> = property.splitn(2, "::").collect();
    match split.len() {
        2 => match split[0] {
            "video" => Ok((true, split[1])),
            "audio" => Ok((false, split[1])),
            _ => Err("Slot property media type must be one of [audio, video]".to_string()),
        },
        _ => Err("Slot property name must be in form media-type::property-name".to_string()),
    }
}

fn validate_setting_value(name: &str, value: &Value) -> Result<(), String> {
    match name {
        "width" | "height" | "sample-rate" | "fallback-timeout" => {
            if value.is_number() {
                Ok(())
            } else {
                Err(format!("Setting `{name}` expects a numeric value"))
            }
        }
        "fallback-image" => {
            if value.is_string() {
                Ok(())
            } else {
                Err("Setting `fallback-image` expects a string value".to_string())
            }
        }
        _ => Err(format!("No setting with name {name} on mixers")),
    }
}

fn validate_slot_value(is_video: bool, property: &str, value: &Value) -> Result<(), String> {
    let numeric = value.is_number();

    if is_video {
        match property {
            "x" | "y" | "width" | "height" | "zorder" | "alpha" => {
                if numeric {
                    Ok(())
                } else {
                    Err(format!("video::{property} expects a numeric value"))
                }
            }
            _ => Ok(()),
        }
    } else {
        match property {
            "volume" => {
                if numeric {
                    Ok(())
                } else {
                    Err("audio::volume expects a numeric value".to_string())
                }
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MixerNode {
    pub id: String,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub slots: HashMap<String, MixerSlotInfo>,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub audio_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub settings: HashMap<String, Value>,
    pub control_points: HashMap<String, Vec<ControlPoint>>,
    pub slot_settings: HashMap<String, HashMap<String, Value>>,
    pub slot_control_points: HashMap<String, HashMap<String, Vec<ControlPoint>>>,
    pub pipeline: MixerPipelineProfile,
    pub last_error: Option<String>,
}

impl MixerNode {
    pub fn new(
        id: String,
        config: Option<HashMap<String, Value>>,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Result<Self, String> {
        let mut settings = HashMap::from([
            ("width".to_string(), Value::from(1920)),
            ("height".to_string(), Value::from(1080)),
            ("sample-rate".to_string(), Value::from(48000)),
            ("fallback-image".to_string(), Value::from("")),
            ("fallback-timeout".to_string(), Value::from(500)),
        ]);

        if let Some(cfg) = config {
            for (key, value) in cfg {
                validate_setting_value(&key, &value)?;
                settings.insert(key, value);
            }
        }

        let pipeline =
            MixerPipelineProfile::from_settings(&settings, audio_enabled, video_enabled, MixerPipelineStage::Idle);

        Ok(Self {
            id,
            audio_enabled,
            video_enabled,
            slots: HashMap::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            audio_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            settings,
            control_points: HashMap::new(),
            slot_settings: HashMap::new(),
            slot_control_points: HashMap::new(),
            pipeline,
            last_error: None,
        })
    }

    fn default_slot_settings(&self, audio: bool, video: bool) -> HashMap<String, Value> {
        let mut defaults = HashMap::new();
        if video {
            let width = self
                .settings
                .get("width")
                .and_then(Value::as_i64)
                .or_else(|| self.settings.get("width").and_then(Value::as_f64).map(|v| v as i64))
                .unwrap_or(1920);
            let height = self
                .settings
                .get("height")
                .and_then(Value::as_i64)
                .or_else(|| self.settings.get("height").and_then(Value::as_f64).map(|v| v as i64))
                .unwrap_or(1080);

            defaults.insert("video::x".to_string(), Value::from(0));
            defaults.insert("video::y".to_string(), Value::from(0));
            defaults.insert("video::width".to_string(), Value::from(width));
            defaults.insert("video::height".to_string(), Value::from(height));
            defaults.insert("video::alpha".to_string(), Value::from(1.0));
            defaults.insert("video::zorder".to_string(), Value::from(0));
        }
        if audio {
            defaults.insert("audio::volume".to_string(), Value::from(1.0));
        }
        defaults
    }

    pub fn connect_output_consumer(&mut self, link_id: &str, audio: bool, video: bool) {
        if audio {
            self.audio_consumer_slot_ids.insert(link_id.to_string());
        }
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn disconnect_output_consumer(&mut self, link_id: &str) {
        self.audio_consumer_slot_ids.remove(link_id);
        self.video_consumer_slot_ids.remove(link_id);
    }

    pub fn connect_input_slot(
        &mut self,
        link_id: &str,
        audio: bool,
        video: bool,
        slot_config: Option<HashMap<String, Value>>,
    ) -> Result<(), String> {
        let mut merged = self.default_slot_settings(audio, video);

        if let Some(cfg) = slot_config {
            for (key, value) in cfg {
                let (is_video, property) = parse_slot_config_key(&key)?;
                if is_video && !video {
                    return Err(format!(
                        "Cannot set {key} on link {link_id}; video is not enabled for this link"
                    ));
                }
                if !is_video && !audio {
                    return Err(format!(
                        "Cannot set {key} on link {link_id}; audio is not enabled for this link"
                    ));
                }
                validate_slot_value(is_video, property, &value)?;
                merged.insert(key, value);
            }
        }

        let volume = merged
            .get("audio::volume")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);

        if audio || video {
            self.slots
                .entry(link_id.to_string())
                .or_insert(MixerSlotInfo { volume });
        }
        if let Some(slot) = self.slots.get_mut(link_id) {
            slot.volume = volume;
        }

        self.slot_settings.insert(link_id.to_string(), merged);
        self.slot_control_points
            .entry(link_id.to_string())
            .or_default();
        Ok(())
    }

    pub fn disconnect_input_slot(&mut self, link_id: &str) {
        self.slots.remove(link_id);
        self.slot_settings.remove(link_id);
        self.slot_control_points.remove(link_id);
    }

    pub fn add_control_point(&mut self, property: &str, cp: ControlPoint) -> Result<(), String> {
        if !self.settings.contains_key(property) {
            return Err(format!("Mixer {} has no setting with name {property}", self.id));
        }
        validate_setting_value(property, &cp.value)?;
        let points = self.control_points.entry(property.to_string()).or_default();
        points.push(cp);
        points.sort();
        self.apply_control_points(Utc::now());
        Ok(())
    }

    pub fn remove_control_point(&mut self, controller_id: &str, property: &str) {
        if let Some(points) = self.control_points.get_mut(property) {
            points.retain(|point| point.id != controller_id);
        }
        self.apply_control_points(Utc::now());
    }

    pub fn add_slot_control_point(
        &mut self,
        slot_id: &str,
        property: &str,
        cp: ControlPoint,
    ) -> Result<(), String> {
        if !self.slot_settings.contains_key(slot_id) {
            return Err(format!("Mixer {} has no slot with id {slot_id}", self.id));
        }
        let (is_video, prop_name) = parse_slot_config_key(property)?;
        validate_slot_value(is_video, prop_name, &cp.value)?;

        let slot = self
            .slot_control_points
            .entry(slot_id.to_string())
            .or_default();
        let points = slot.entry(property.to_string()).or_default();
        points.push(cp);
        points.sort();
        self.apply_control_points(Utc::now());
        Ok(())
    }

    pub fn remove_slot_control_point(
        &mut self,
        controller_id: &str,
        slot_id: &str,
        property: &str,
    ) {
        if let Some(slot) = self.slot_control_points.get_mut(slot_id) {
            if let Some(points) = slot.get_mut(property) {
                points.retain(|point| point.id != controller_id);
            }
        }
        self.apply_control_points(Utc::now());
    }

    pub fn apply_control_points(&mut self, at: DateTime<Utc>) {
        let mixer_updates: Vec<(String, Value)> = self
            .control_points
            .iter()
            .filter_map(|(property, points)| {
                evaluate_control_points(points, at).map(|value| (property.clone(), value))
            })
            .collect();

        for (property, value) in mixer_updates {
            self.settings.insert(property, value);
        }

        let slot_updates: Vec<(String, String, Value)> = self
            .slot_control_points
            .iter()
            .flat_map(|(slot_id, properties)| {
                properties.iter().filter_map(|(property, points)| {
                    evaluate_control_points(points, at)
                        .map(|value| (slot_id.clone(), property.clone(), value))
                })
            })
            .collect();

        for (slot_id, property, value) in slot_updates {
            self.slot_settings
                .entry(slot_id.clone())
                .or_default()
                .insert(property.clone(), value.clone());

            if property == "audio::volume" {
                if let Some(slot) = self.slots.get_mut(&slot_id) {
                    slot.volume = value.as_f64().unwrap_or(slot.volume);
                }
            }
        }
    }

    pub fn schedule(&mut self, cue_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) {
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;
        self.apply_control_points(Utc::now());

        let now = Utc::now();
        let stage = match cue_time {
            Some(cue) => {
                let preroll_at = cue - Duration::seconds(PREROLL_LEAD_TIME_SECONDS);
                if now < preroll_at {
                    self.state = State::Initial;
                    MixerPipelineStage::Idle
                } else if now < cue {
                    self.state = State::Starting;
                    MixerPipelineStage::Starting
                } else {
                    self.state = State::Started;
                    MixerPipelineStage::Playing
                }
            }
            None => {
                self.state = State::Started;
                MixerPipelineStage::Playing
            }
        };

        self.pipeline =
            MixerPipelineProfile::from_settings(&self.settings, self.audio_enabled, self.video_enabled, stage);
    }

    pub fn stop(&mut self) {
        self.state = State::Stopped;
        self.pipeline.stage = MixerPipelineStage::Idle;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Mixer(MixerInfo {
            slots: self.slots.clone(),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: Some(self.audio_consumer_slot_ids.iter().cloned().collect()),
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
            settings: self.settings.clone(),
            control_points: self.control_points.clone(),
            slot_settings: self.slot_settings.clone(),
            slot_control_points: self.slot_control_points.clone(),
        })
    }
}
