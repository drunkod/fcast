use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeSet;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoGeneratorStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct VideoGeneratorPipelineProfile {
    pub elements: Vec<String>,
    pub pattern: String,
    pub is_live: bool,
    pub flip: bool,
    pub stage: VideoGeneratorStage,
}

impl VideoGeneratorPipelineProfile {
    fn new() -> Self {
        Self {
            elements: vec![
                "videotestsrc".to_string(),
                "deinterlace".to_string(),
                "appsink".to_string(),
            ],
            pattern: "ball".to_string(),
            is_live: true,
            flip: true,
            stage: VideoGeneratorStage::Idle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoGeneratorNode {
    pub id: String,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_consumer_slot_ids: BTreeSet<String>,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub pipeline: VideoGeneratorPipelineProfile,
    pub last_error: Option<String>,
}

impl VideoGeneratorNode {
    pub fn new(id: String) -> Self {
        Self {
            id,
            audio_enabled: false,
            video_enabled: true,
            audio_consumer_slot_ids: BTreeSet::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            pipeline: VideoGeneratorPipelineProfile::new(),
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

    pub fn schedule(&mut self, cue_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) {
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;

        let now = Utc::now();
        self.state = match cue_time {
            Some(cue) => {
                let preroll_at = cue - Duration::seconds(PREROLL_LEAD_TIME_SECONDS);
                if now < preroll_at {
                    self.pipeline.stage = VideoGeneratorStage::Idle;
                    State::Initial
                } else if now < cue {
                    self.pipeline.stage = VideoGeneratorStage::Prerolling;
                    State::Starting
                } else {
                    self.pipeline.stage = VideoGeneratorStage::Playing;
                    State::Started
                }
            }
            None => {
                self.pipeline.stage = VideoGeneratorStage::Playing;
                State::Started
            }
        };
    }

    pub fn stop(&mut self) {
        self.pipeline.stage = VideoGeneratorStage::Idle;
        self.state = State::Stopped;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    // Old protocol has no dedicated VideoGenerator info variant.
    // We encode it as a synthetic SourceInfo for compatibility.
    pub fn as_compatible_source_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: format!("videogenerator://{}", self.id),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: Some(self.audio_consumer_slot_ids.iter().cloned().collect()),
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }
}
