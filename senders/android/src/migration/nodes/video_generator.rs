use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
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

#[derive(Debug, Clone)]
pub struct LiveVideoGeneratorPipeline {
    pub pipeline: gst::Pipeline,
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
    pub live_pipeline: Option<LiveVideoGeneratorPipeline>,
    pub last_error: Option<String>,
}

impl VideoGeneratorNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

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
            live_pipeline: None,
            last_error: None,
        }
    }

    fn make_element(element: &str, name: &str) -> Result<gst::Element, String> {
        gst::ElementFactory::make(element)
            .name(name)
            .build()
            .map_err(|err| format!("Failed to create element `{element}`: {}", &*err.message))
    }

    fn build_live_pipeline(id: &str, profile: &VideoGeneratorPipelineProfile) -> Result<LiveVideoGeneratorPipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-videogen-{id}"));

        let src = Self::make_element("videotestsrc", &format!("videogen-src-{id}"))?;
        src.set_property("flip", profile.flip);
        src.set_property("is-live", profile.is_live);
        src.set_property_from_str("pattern", &profile.pattern);

        let deinterlace = Self::make_element("deinterlace", &format!("videogen-deinterlace-{id}"))?;
        let appsink = Self::make_element("appsink", &format!("videogen-appsink-{id}"))?;

        pipeline
            .add(&src)
            .map_err(|err| format!("Failed to add videotestsrc to video generator pipeline: {err:?}"))?;
        pipeline
            .add(&deinterlace)
            .map_err(|err| format!("Failed to add deinterlace to video generator pipeline: {err:?}"))?;
        pipeline
            .add(&appsink)
            .map_err(|err| format!("Failed to add appsink to video generator pipeline: {err:?}"))?;

        src.link(&deinterlace)
            .map_err(|err| format!("Failed to link videotestsrc->deinterlace: {err:?}"))?;
        deinterlace
            .link(&appsink)
            .map_err(|err| format!("Failed to link deinterlace->appsink: {err:?}"))?;

        Ok(LiveVideoGeneratorPipeline { pipeline })
    }

    fn teardown_live_pipeline(&mut self) {
        if let Some(live) = self.live_pipeline.take() {
            let _ = live.pipeline.set_state(gst::State::Null);
        }
    }

    fn ensure_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_some() {
            return Ok(());
        }

        self.live_pipeline = Some(Self::build_live_pipeline(&self.id, &self.pipeline)?);
        Ok(())
    }

    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        // Unit tests and host-only flows may call migration code before GStreamer init.
        if !Self::gst_initialized() {
            return Ok(());
        }

        match self.pipeline.stage {
            VideoGeneratorStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            VideoGeneratorStage::Prerolling | VideoGeneratorStage::Playing => {
                self.ensure_live_pipeline()?;
                let target_state = if self.pipeline.stage == VideoGeneratorStage::Prerolling {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    live.pipeline
                        .set_state(target_state)
                        .map_err(|err| format!("Failed to set video generator pipeline state to {target_state:?}: {err:?}"))?;
                }

                Ok(())
            }
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

    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
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

        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err.clone());
            self.pipeline.stage = VideoGeneratorStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.teardown_live_pipeline();
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
