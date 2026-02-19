use crate::migration::protocol::{ControlPoint, MixerInfo, MixerSlotInfo, NodeInfo, State};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

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
}

impl MixerNode {
    pub fn new(
        id: String,
        config: Option<HashMap<String, Value>>,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Self {
        let mut settings = HashMap::from([
            ("width".to_string(), Value::from(1920)),
            ("height".to_string(), Value::from(1080)),
            ("sample-rate".to_string(), Value::from(48000)),
            ("fallback-image".to_string(), Value::from("")),
            ("fallback-timeout".to_string(), Value::from(500)),
        ]);
        if let Some(cfg) = config {
            settings.extend(cfg);
        }

        Self {
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
        }
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
    ) {
        let cfg = slot_config.unwrap_or_default();
        self.slot_settings.insert(link_id.to_string(), cfg.clone());

        if audio || video {
            let volume = cfg
                .get("audio::volume")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            self.slots
                .entry(link_id.to_string())
                .or_insert(MixerSlotInfo { volume });
        }

        self.slot_control_points
            .entry(link_id.to_string())
            .or_default();
    }

    pub fn disconnect_input_slot(&mut self, link_id: &str) {
        self.slots.remove(link_id);
        self.slot_settings.remove(link_id);
        self.slot_control_points.remove(link_id);
    }

    pub fn add_control_point(&mut self, property: &str, cp: ControlPoint) {
        let points = self.control_points.entry(property.to_string()).or_default();
        points.push(cp);
        points.sort();
    }

    pub fn remove_control_point(&mut self, controller_id: &str, property: &str) {
        if let Some(points) = self.control_points.get_mut(property) {
            points.retain(|point| point.id != controller_id);
        }
    }

    pub fn add_slot_control_point(&mut self, slot_id: &str, property: &str, cp: ControlPoint) {
        let slot = self
            .slot_control_points
            .entry(slot_id.to_string())
            .or_default();
        let points = slot.entry(property.to_string()).or_default();
        points.push(cp);
        points.sort();
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
    }

    pub fn set_schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        state: State,
    ) {
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.state = state;
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
