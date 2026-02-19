use crate::migration::protocol::{DestinationFamily, DestinationInfo, NodeInfo, State};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationPipelineStage {
    Idle,
    Scheduled,
    Playing,
}

#[derive(Debug, Clone)]
pub struct DestinationPipelineProfile {
    pub family: DestinationFamily,
    pub elements: Vec<String>,
    pub wait_for_eos_on_stop: bool,
    pub stage: DestinationPipelineStage,
}

impl DestinationPipelineProfile {
    fn from_family(family: &DestinationFamily, audio: bool, video: bool) -> Self {
        let mut elements = Vec::new();

        match family {
            DestinationFamily::Rtmp { .. } => {
                elements.extend([
                    "flvmux",
                    "queue",
                    "rtmp2sink",
                    "videoconvert",
                    "timecodestamper",
                    "timeoverlay",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
            DestinationFamily::Udp { .. } => {
                elements.extend([
                    "mpegtsmux",
                    "udpsink",
                    "videoconvert",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
            DestinationFamily::LocalFile { .. } => {
                elements.extend([
                    "splitmuxsink",
                    "multiqueue",
                    "videoconvert",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
            DestinationFamily::LocalPlayback => {
                elements.extend([
                    "autovideosink",
                    "autoaudiosink",
                    "videoconvert",
                    "audioconvert",
                    "audioresample",
                    "queue",
                ]);
            }
        }

        if !audio {
            elements.retain(|el| !el.contains("audio"));
        }
        if !video {
            elements.retain(|el| !el.contains("video") && !el.contains("h264"));
        }

        Self {
            family: family.clone(),
            elements: elements.into_iter().map(str::to_string).collect(),
            wait_for_eos_on_stop: true,
            stage: DestinationPipelineStage::Idle,
        }
    }
}

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
    pub pipeline: Option<DestinationPipelineProfile>,
    pub last_error: Option<String>,
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
            pipeline: None,
            last_error: None,
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

    fn ensure_start_ready(&self) -> Result<(), String> {
        if self.audio_enabled && self.audio_slot_id.is_none() {
            return Err(format!(
                "Destination {} must have its audio slot connected before starting",
                self.id
            ));
        }
        if self.video_enabled && self.video_slot_id.is_none() {
            return Err(format!(
                "Destination {} must have its video slot connected before starting",
                self.id
            ));
        }
        Ok(())
    }

    /// Mirrors old destination behavior:
    /// - validates required slots before scheduling
    /// - builds family-specific pipeline profile on activation
    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        self.ensure_start_ready()?;
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;

        let now = Utc::now();
        if cue_time.is_some_and(|cue| cue > now) {
            self.state = State::Starting;
            let mut pipeline =
                DestinationPipelineProfile::from_family(&self.family, self.audio_enabled, self.video_enabled);
            pipeline.stage = DestinationPipelineStage::Scheduled;
            self.pipeline = Some(pipeline);
        } else {
            self.state = State::Started;
            let mut pipeline =
                DestinationPipelineProfile::from_family(&self.family, self.audio_enabled, self.video_enabled);
            pipeline.stage = DestinationPipelineStage::Playing;
            self.pipeline = Some(pipeline);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.state = State::Stopped;
        if let Some(pipeline) = self.pipeline.as_mut() {
            pipeline.stage = DestinationPipelineStage::Idle;
        }
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
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
