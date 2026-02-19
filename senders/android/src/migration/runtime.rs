use crate::migration::{
    node_manager::NodeManager,
    protocol::{Command, CommandResult, ControllerMessage, ServerMessage},
};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::Deserialize;
use tracing::error;

lazy_static::lazy_static! {
    static ref GRAPH_NODE_MANAGER: Mutex<NodeManager> = Mutex::new(NodeManager::default());
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InboundCommand {
    Controller(ControllerMessage),
    Command(Command),
}

pub fn start_graph_runtime() -> Result<()> {
    let mut manager = GRAPH_NODE_MANAGER.lock();
    manager.start();
    Ok(())
}

pub fn shutdown_graph_runtime() -> Result<()> {
    let mut manager = GRAPH_NODE_MANAGER.lock();
    manager.shutdown();
    Ok(())
}

pub fn handle_command(command: Command) -> CommandResult {
    GRAPH_NODE_MANAGER.lock().dispatch(command)
}

pub fn handle_controller_message(message: ControllerMessage) -> ServerMessage {
    let result = handle_command(message.command);
    ServerMessage {
        id: Some(message.id),
        result,
    }
}

pub fn handle_command_json(payload: &str) -> Result<String> {
    let inbound: InboundCommand =
        serde_json::from_str(payload).context("Failed to parse command JSON payload")?;

    let response = match inbound {
        InboundCommand::Controller(msg) => handle_controller_message(msg),
        InboundCommand::Command(command) => ServerMessage {
            id: None,
            result: handle_command(command),
        },
    };

    serde_json::to_string(&response).context("Failed to serialize command response")
}

pub fn try_handle_command_json(payload: &str) -> String {
    match handle_command_json(payload) {
        Ok(response) => response,
        Err(err) => {
            error!(?err, "Failed to handle graph command JSON");
            serde_json::to_string(&ServerMessage {
                id: None,
                result: CommandResult::Error(format!("Invalid command payload: {err}")),
            })
            .unwrap_or_else(|ser_err| {
                format!(
                    "{{\"id\":null,\"result\":{{\"error\":\"Serialization failure: {}\"}}}}",
                    ser_err
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::protocol::{Command, DestinationFamily};

    #[test]
    fn json_command_roundtrip() {
        start_graph_runtime().unwrap();

        let payload = serde_json::to_string(&Command::CreateSource {
            id: "test-source".to_string(),
            uri: "https://example.com/video.mp4".to_string(),
            audio: true,
            video: true,
        })
        .unwrap();

        let response = handle_command_json(&payload).unwrap();
        assert!(response.contains("success"));

        let payload = serde_json::to_string(&Command::CreateDestination {
            id: "test-destination".to_string(),
            family: DestinationFamily::LocalPlayback,
            audio: true,
            video: true,
        })
        .unwrap();
        let response = handle_command_json(&payload).unwrap();
        assert!(response.contains("success"));

        let payload = serde_json::to_string(&Command::Connect {
            link_id: "test-link".to_string(),
            src_id: "test-source".to_string(),
            sink_id: "test-destination".to_string(),
            audio: true,
            video: true,
            config: None,
        })
        .unwrap();
        let response = handle_command_json(&payload).unwrap();
        assert!(response.contains("success"));

        shutdown_graph_runtime().unwrap();
    }
}
