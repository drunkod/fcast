use crate::migration::protocol::{DestinationFamily, DestinationInfo, NodeInfo, State};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DestinationNode {
    pub id: String,
    pub family: DestinationFamily,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
}

impl DestinationNode {
    pub fn new(
        id: String,
        family: DestinationFamily,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Self {
        Self {
            id,
            family,
            audio_enabled,
            video_enabled,
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
        }
    }

    pub fn connect_input(&mut self, link_id: &str, audio: bool, video: bool) -> Result<(), String> {
        if audio {
            if self.audio_slot_id.is_some() {
                return Err(format!(
                    "Destination {} already has an audio input slot",
                    self.id
                ));
            }
            self.audio_slot_id = Some(link_id.to_string());
        }
        if video {
            if self.video_slot_id.is_some() {
                return Err(format!(
                    "Destination {} already has a video input slot",
                    self.id
                ));
            }
            self.video_slot_id = Some(link_id.to_string());
        }
        Ok(())
    }

    pub fn disconnect_input(&mut self, link_id: &str) {
        if self.audio_slot_id.as_deref() == Some(link_id) {
            self.audio_slot_id = None;
        }
        if self.video_slot_id.as_deref() == Some(link_id) {
            self.video_slot_id = None;
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
        NodeInfo::Destination(DestinationInfo {
            family: self.family.clone(),
            audio_slot_id: self.audio_slot_id.clone(),
            video_slot_id: self.video_slot_id.clone(),
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }
}
