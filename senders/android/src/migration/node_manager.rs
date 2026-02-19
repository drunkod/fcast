use crate::migration::{
    nodes::{DestinationNode, MixerNode, SourceNode, VideoGeneratorNode},
    protocol::{Command, CommandResult, ControlPoint, Info, NodeInfo, State},
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct LinkRecord {
    src_id: String,
    sink_id: String,
    audio: bool,
    video: bool,
    config: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
enum NodeRecord {
    Source(SourceNode),
    Destination(DestinationNode),
    Mixer(MixerNode),
    VideoGenerator(VideoGeneratorNode),
}

impl NodeRecord {
    fn can_output_audio(&self) -> bool {
        match self {
            Self::Source(node) => node.audio_enabled,
            Self::Mixer(node) => node.audio_enabled,
            Self::VideoGenerator(node) => node.audio_enabled,
            Self::Destination(_) => false,
        }
    }

    fn can_output_video(&self) -> bool {
        match self {
            Self::Source(node) => node.video_enabled,
            Self::Mixer(node) => node.video_enabled,
            Self::VideoGenerator(node) => node.video_enabled,
            Self::Destination(_) => false,
        }
    }

    fn can_input_audio(&self) -> bool {
        match self {
            Self::Destination(node) => node.audio_enabled,
            Self::Mixer(node) => node.audio_enabled,
            Self::Source(_) | Self::VideoGenerator(_) => false,
        }
    }

    fn can_input_video(&self) -> bool {
        match self {
            Self::Destination(node) => node.video_enabled,
            Self::Mixer(node) => node.video_enabled,
            Self::Source(_) | Self::VideoGenerator(_) => false,
        }
    }

    fn to_info(&self) -> NodeInfo {
        match self {
            Self::Source(node) => node.as_info(),
            Self::Destination(node) => node.as_info(),
            Self::Mixer(node) => node.as_info(),
            Self::VideoGenerator(node) => node.as_compatible_source_info(),
        }
    }

    fn set_schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        state: State,
    ) {
        match self {
            Self::Source(node) => node.set_schedule(cue_time, end_time, state),
            Self::Destination(node) => node.set_schedule(cue_time, end_time, state),
            Self::Mixer(node) => node.set_schedule(cue_time, end_time, state),
            Self::VideoGenerator(node) => node.set_schedule(cue_time, end_time, state),
        }
    }

    fn add_consumer_link(&mut self, link_id: &str, audio: bool, video: bool) {
        match self {
            Self::Source(node) => node.add_consumer_link(link_id, audio, video),
            Self::Mixer(node) => node.connect_output_consumer(link_id, audio, video),
            Self::VideoGenerator(node) => node.add_consumer_link(link_id, audio, video),
            Self::Destination(_) => {}
        }
    }

    fn remove_consumer_link(&mut self, link_id: &str) {
        match self {
            Self::Source(node) => node.remove_consumer_link(link_id),
            Self::Mixer(node) => node.disconnect_output_consumer(link_id),
            Self::VideoGenerator(node) => node.remove_consumer_link(link_id),
            Self::Destination(_) => {}
        }
    }
}

#[derive(Debug, Default)]
pub struct NodeManager {
    started: bool,
    nodes: HashMap<String, NodeRecord>,
    links: HashMap<String, LinkRecord>,
}

impl NodeManager {
    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn shutdown(&mut self) {
        self.started = false;
        self.nodes.clear();
        self.links.clear();
    }

    pub fn dispatch(&mut self, command: Command) -> CommandResult {
        if !self.started {
            self.started = true;
        }

        match command {
            Command::CreateVideoGenerator { id } => self.create_video_generator(id),
            Command::CreateSource {
                id,
                uri,
                audio,
                video,
            } => self.create_source(id, uri, audio, video),
            Command::CreateDestination {
                id,
                family,
                audio,
                video,
            } => self.create_destination(id, family, audio, video),
            Command::CreateMixer {
                id,
                config,
                audio,
                video,
            } => self.create_mixer(id, config, audio, video),
            Command::Connect {
                link_id,
                src_id,
                sink_id,
                audio,
                video,
                config,
            } => self.connect(link_id, src_id, sink_id, audio, video, config),
            Command::Disconnect { link_id } => self.disconnect(&link_id),
            Command::Start {
                id,
                cue_time,
                end_time,
            } => self.schedule_node(&id, cue_time, end_time),
            Command::Reschedule {
                id,
                cue_time,
                end_time,
            } => self.schedule_node(&id, cue_time, end_time),
            Command::Remove { id } => self.remove_node(&id),
            Command::GetInfo { id } => self.get_info(id.as_ref()),
            Command::AddControlPoint {
                controllee_id,
                property,
                control_point,
            } => self.add_control_point(&controllee_id, &property, control_point),
            Command::RemoveControlPoint {
                id,
                controllee_id,
                property,
            } => self.remove_control_point(&id, &controllee_id, &property),
        }
    }

    fn ensure_unique_id(&self, id: &str) -> Result<(), String> {
        if self.nodes.contains_key(id) {
            return Err(format!("A node already exists with id {id}"));
        }
        Ok(())
    }

    fn create_video_generator(&mut self, id: String) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::VideoGenerator(VideoGeneratorNode::new(id)),
        );
        CommandResult::Success
    }

    fn create_source(
        &mut self,
        id: String,
        uri: String,
        audio: bool,
        video: bool,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if !audio && !video {
            return CommandResult::Error(format!(
                "Source with id {id} must have either audio or video enabled"
            ));
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::Source(SourceNode::new(id, uri, audio, video)),
        );
        CommandResult::Success
    }

    fn create_destination(
        &mut self,
        id: String,
        family: crate::migration::protocol::DestinationFamily,
        audio: bool,
        video: bool,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if !audio && !video {
            return CommandResult::Error(format!(
                "Destination with id {id} must have either audio or video enabled"
            ));
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::Destination(DestinationNode::new(id, family, audio, video)),
        );
        CommandResult::Success
    }

    fn create_mixer(
        &mut self,
        id: String,
        config: Option<HashMap<String, Value>>,
        audio: bool,
        video: bool,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if !audio && !video {
            return CommandResult::Error(format!(
                "Mixer with id {id} must have either audio or video enabled"
            ));
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::Mixer(MixerNode::new(id, config, audio, video)),
        );
        CommandResult::Success
    }

    fn connect(
        &mut self,
        link_id: String,
        src_id: String,
        sink_id: String,
        audio: bool,
        video: bool,
        config: Option<HashMap<String, Value>>,
    ) -> CommandResult {
        if !audio && !video {
            return CommandResult::Error(format!(
                "Link with id {link_id} must have either audio or video enabled"
            ));
        }
        if self.links.contains_key(&link_id) {
            return CommandResult::Error(format!("A link already exists with id {link_id}"));
        }

        let Some(src_node) = self.nodes.get(&src_id) else {
            return CommandResult::Error(format!("No producer with id {src_id}"));
        };
        let Some(sink_node) = self.nodes.get(&sink_id) else {
            return CommandResult::Error(format!("No consumer with id {sink_id}"));
        };

        if audio && (!src_node.can_output_audio() || !sink_node.can_input_audio()) {
            return CommandResult::Error(format!(
                "Link {link_id} requested audio, but source/sink capabilities do not match"
            ));
        }
        if video && (!src_node.can_output_video() || !sink_node.can_input_video()) {
            return CommandResult::Error(format!(
                "Link {link_id} requested video, but source/sink capabilities do not match"
            ));
        }

        let sink_update = match self.nodes.get_mut(&sink_id) {
            Some(NodeRecord::Destination(dest)) => dest.connect_input(&link_id, audio, video),
            Some(NodeRecord::Mixer(mixer)) => {
                mixer.connect_input_slot(&link_id, audio, video, config.clone());
                Ok(())
            }
            Some(NodeRecord::Source(_)) | Some(NodeRecord::VideoGenerator(_)) => {
                Err(format!("Node {sink_id} is not a consumer"))
            }
            None => Err(format!("No consumer with id {sink_id}")),
        };

        if let Err(err) = sink_update {
            return CommandResult::Error(err);
        }

        if let Some(src) = self.nodes.get_mut(&src_id) {
            src.add_consumer_link(&link_id, audio, video);
        }

        self.links.insert(
            link_id,
            LinkRecord {
                src_id,
                sink_id,
                audio,
                video,
                config,
            },
        );

        CommandResult::Success
    }

    fn disconnect(&mut self, link_id: &str) -> CommandResult {
        let Some(link) = self.links.remove(link_id) else {
            return CommandResult::Error(format!("No link with id {link_id}"));
        };

        if let Some(src) = self.nodes.get_mut(&link.src_id) {
            src.remove_consumer_link(link_id);
        }
        if let Some(sink) = self.nodes.get_mut(&link.sink_id) {
            match sink {
                NodeRecord::Destination(dest) => dest.disconnect_input(link_id),
                NodeRecord::Mixer(mixer) => mixer.disconnect_input_slot(link_id),
                NodeRecord::Source(_) | NodeRecord::VideoGenerator(_) => {}
            }
        }

        CommandResult::Success
    }

    fn schedule_node(
        &mut self,
        id: &str,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> CommandResult {
        let Some(node) = self.nodes.get_mut(id) else {
            return CommandResult::Error(format!("No node with id {id}"));
        };

        let state = match cue_time {
            Some(cue) if cue > Utc::now() => State::Starting,
            _ => State::Started,
        };
        node.set_schedule(cue_time, end_time, state);
        CommandResult::Success
    }

    fn remove_node(&mut self, id: &str) -> CommandResult {
        if !self.nodes.contains_key(id) {
            return CommandResult::Error(format!("No node with id {id}"));
        }

        let link_ids = self
            .links
            .iter()
            .filter_map(|(link_id, link)| {
                if link.src_id == id || link.sink_id == id {
                    Some(link_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for link_id in link_ids {
            let _ = self.disconnect(&link_id);
        }

        self.nodes.remove(id);
        CommandResult::Success
    }

    fn get_info(&self, id: Option<&String>) -> CommandResult {
        let mut nodes = HashMap::new();
        match id {
            Some(id) => {
                let Some(node) = self.nodes.get(id) else {
                    return CommandResult::Error(format!("No node with id {id}"));
                };
                nodes.insert(id.clone(), node.to_info());
            }
            None => {
                for (id, node) in &self.nodes {
                    nodes.insert(id.clone(), node.to_info());
                }
            }
        }

        CommandResult::Info(Info { nodes })
    }

    fn add_control_point(
        &mut self,
        controllee_id: &str,
        property: &str,
        control_point: ControlPoint,
    ) -> CommandResult {
        if let Some(link) = self.links.get(controllee_id).cloned() {
            let Some(node) = self.nodes.get_mut(&link.sink_id) else {
                return CommandResult::Error(format!(
                    "No sink node with id {} for link {}",
                    link.sink_id, controllee_id
                ));
            };

            if let NodeRecord::Mixer(mixer) = node {
                mixer.add_slot_control_point(controllee_id, property, control_point);
                return CommandResult::Success;
            }

            return CommandResult::Error(format!(
                "Slot control points are only supported for mixer links; {} is not a mixer",
                link.sink_id
            ));
        }

        let Some(node) = self.nodes.get_mut(controllee_id) else {
            return CommandResult::Error(format!("No node or slot with id {controllee_id}"));
        };

        if let NodeRecord::Mixer(mixer) = node {
            mixer.add_control_point(property, control_point);
            return CommandResult::Success;
        }

        CommandResult::Error(format!(
            "Node control points are currently supported only for mixers; {controllee_id} is not a mixer"
        ))
    }

    fn remove_control_point(
        &mut self,
        controller_id: &str,
        controllee_id: &str,
        property: &str,
    ) -> CommandResult {
        if let Some(link) = self.links.get(controllee_id).cloned() {
            let Some(node) = self.nodes.get_mut(&link.sink_id) else {
                return CommandResult::Error(format!(
                    "No sink node with id {} for link {}",
                    link.sink_id, controllee_id
                ));
            };

            if let NodeRecord::Mixer(mixer) = node {
                mixer.remove_slot_control_point(controller_id, controllee_id, property);
                return CommandResult::Success;
            }

            return CommandResult::Error(format!(
                "Slot control points are only supported for mixer links; {} is not a mixer",
                link.sink_id
            ));
        }

        let Some(node) = self.nodes.get_mut(controllee_id) else {
            return CommandResult::Error(format!("No node or slot with id {controllee_id}"));
        };

        if let NodeRecord::Mixer(mixer) = node {
            mixer.remove_control_point(controller_id, property);
            return CommandResult::Success;
        }

        CommandResult::Error(format!(
            "Node control points are currently supported only for mixers; {controllee_id} is not a mixer"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::protocol::{Command, DestinationFamily};

    #[test]
    fn create_connect_and_get_info() {
        let mut manager = NodeManager::default();
        manager.start();

        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-1".to_string(),
                uri: "https://example.com/video.mp4".to_string(),
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "dest-1".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::Connect {
                link_id: "link-1".to_string(),
                src_id: "source-1".to_string(),
                sink_id: "dest-1".to_string(),
                audio: true,
                video: true,
                config: None,
            }),
            CommandResult::Success
        ));

        let result = manager.dispatch(Command::GetInfo { id: None });
        match result {
            CommandResult::Info(info) => {
                assert!(info.nodes.contains_key("source-1"));
                assert!(info.nodes.contains_key("dest-1"));
            }
            other => panic!("Expected info result, got {other:?}"),
        }
    }
}
