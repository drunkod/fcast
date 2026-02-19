use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

fn default_as_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ControlMode {
    Set,
    Interpolate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct ControlPoint {
    pub id: String,
    pub time: DateTime<Utc>,
    pub value: serde_json::Value,
    pub mode: ControlMode,
}

impl Ord for ControlPoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for ControlPoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    CreateVideoGenerator {
        id: String,
    },
    CreateSource {
        id: String,
        uri: String,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
    },
    CreateDestination {
        id: String,
        family: DestinationFamily,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
    },
    CreateMixer {
        id: String,
        config: Option<HashMap<String, serde_json::Value>>,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
    },
    Connect {
        link_id: String,
        src_id: String,
        sink_id: String,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
        config: Option<HashMap<String, serde_json::Value>>,
    },
    Start {
        id: String,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    },
    Reschedule {
        id: String,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    },
    Remove {
        id: String,
    },
    Disconnect {
        link_id: String,
    },
    GetInfo {
        id: Option<String>,
    },
    AddControlPoint {
        controllee_id: String,
        property: String,
        control_point: ControlPoint,
    },
    RemoveControlPoint {
        id: String,
        controllee_id: String,
        property: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ControllerMessage {
    pub id: uuid::Uuid,
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Initial,
    Starting,
    Started,
    Stopping,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestinationFamily {
    Rtmp {
        uri: String,
    },
    Udp {
        host: String,
    },
    LocalFile {
        base_name: String,
        max_size_time: Option<u32>,
    },
    LocalPlayback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct SourceInfo {
    pub uri: String,
    pub video_consumer_slot_ids: Option<Vec<String>>,
    pub audio_consumer_slot_ids: Option<Vec<String>>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct DestinationInfo {
    pub family: DestinationFamily,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct MixerSlotInfo {
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct MixerInfo {
    pub slots: HashMap<String, MixerSlotInfo>,
    pub video_consumer_slot_ids: Option<Vec<String>>,
    pub audio_consumer_slot_ids: Option<Vec<String>>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub settings: HashMap<String, serde_json::Value>,
    pub control_points: HashMap<String, Vec<ControlPoint>>,
    pub slot_settings: HashMap<String, HashMap<String, serde_json::Value>>,
    pub slot_control_points: HashMap<String, HashMap<String, Vec<ControlPoint>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeInfo {
    Source(SourceInfo),
    Destination(DestinationInfo),
    Mixer(MixerInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Info {
    pub nodes: HashMap<String, NodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandResult {
    Error(String),
    Success,
    Info(Info),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ServerMessage {
    pub id: Option<uuid::Uuid>,
    pub result: CommandResult,
}
