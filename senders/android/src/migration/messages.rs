use crate::migration::protocol::{Command, CommandResult, ControlPoint, NodeInfo, State};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct StartMessage {
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct StopMessage;

#[derive(Debug, Clone)]
pub struct ScheduleMessage {
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AddControlPointMessage {
    pub property: String,
    pub control_point: ControlPoint,
}

#[derive(Debug, Clone)]
pub struct RemoveControlPointMessage {
    pub controller_id: String,
    pub property: String,
}

#[derive(Debug, Clone, Copy)]
pub struct GetNodeInfoMessage;

#[derive(Debug, Clone)]
pub struct StoppedMessage {
    pub id: String,
    pub has_video_producer: bool,
    pub has_audio_producer: bool,
}

#[derive(Debug, Clone)]
pub enum NodeStatusMessage {
    State { id: String, state: State },
    Error { id: String, message: String },
}

#[derive(Debug, Clone)]
pub enum ConsumerMessage {
    Connect {
        link_id: String,
        has_video: bool,
        has_audio: bool,
        config: Option<std::collections::HashMap<String, serde_json::Value>>,
    },
    Disconnect {
        slot_id: String,
    },
    AddControlPoint {
        slot_id: String,
        property: String,
        control_point: ControlPoint,
    },
    RemoveControlPoint {
        controller_id: String,
        slot_id: String,
        property: String,
    },
}

#[derive(Debug, Clone)]
pub struct CommandMessage {
    pub command: Command,
}

#[derive(Debug, Clone)]
pub struct RegisterListenerMessage {
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum MessageResult {
    Command(CommandResult),
    NodeInfo(NodeInfo),
    Empty,
}
