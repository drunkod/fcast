use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeSet;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePipelineStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct SourcePipelineProfile {
    pub uri: String,
    pub manual_unblock: bool,
    pub immediate_fallback: bool,
    pub elements: Vec<String>,
    pub stage: SourcePipelineStage,
}

impl SourcePipelineProfile {
    fn new(uri: String) -> Self {
        Self {
            uri,
            manual_unblock: true,
            immediate_fallback: true,
            elements: vec![
                "fallbacksrc".to_string(),
                "deinterlace".to_string(),
                "audioconvert".to_string(),
                "level".to_string(),
                "appsink".to_string(),
            ],
            stage: SourcePipelineStage::Idle,
        }
    }
}

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
    pub pipeline: SourcePipelineProfile,
    pub last_error: Option<String>,
}

impl SourceNode {
    pub fn new(id: String, uri: String, audio_enabled: bool, video_enabled: bool) -> Self {
        Self {
            id,
            uri: uri.clone(),
            audio_enabled,
            video_enabled,
            audio_consumer_slot_ids: BTreeSet::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            pipeline: SourcePipelineProfile::new(uri),
            last_error: None,
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

    /// Mirrors old source scheduling semantics:
    /// - Initial until `cue - 10s`
    /// - Starting between `cue - 10s` and `cue` (preroll)
    /// - Started at/after `cue` (unblocked/playing)
    pub fn schedule(&mut self, cue_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) {
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;

        let now = Utc::now();
        self.state = match cue_time {
            Some(cue) => {
                let preroll_at = cue - Duration::seconds(PREROLL_LEAD_TIME_SECONDS);
                if now < preroll_at {
                    self.pipeline.stage = SourcePipelineStage::Idle;
                    State::Initial
                } else if now < cue {
                    self.pipeline.stage = SourcePipelineStage::Prerolling;
                    State::Starting
                } else {
                    self.pipeline.stage = SourcePipelineStage::Playing;
                    State::Started
                }
            }
            None => {
                self.pipeline.stage = SourcePipelineStage::Playing;
                State::Started
            }
        };
    }

    pub fn stop(&mut self) {
        self.pipeline.stage = SourcePipelineStage::Idle;
        self.state = State::Stopped;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
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
