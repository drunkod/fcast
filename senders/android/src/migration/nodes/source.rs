use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct SourceNode {
    pub id: String,
    pub uri: String,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_consumer_slot_ids: BTreeSet<String>,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
}

impl SourceNode {
    pub fn new(id: String, uri: String, audio_enabled: bool, video_enabled: bool) -> Self {
        Self {
            id,
            uri,
            audio_enabled,
            video_enabled,
            audio_consumer_slot_ids: BTreeSet::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
        }
    }

    pub fn add_consumer_link(&mut self, link_id: &str, audio: bool, video: bool) {
        if audio {
            self.audio_consumer_slot_ids.insert(link_id.to_string());
        }
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn remove_consumer_link(&mut self, link_id: &str) {
        self.audio_consumer_slot_ids.remove(link_id);
        self.video_consumer_slot_ids.remove(link_id);
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
        NodeInfo::Source(SourceInfo {
            uri: self.uri.clone(),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: Some(self.audio_consumer_slot_ids.iter().cloned().collect()),
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }
}
