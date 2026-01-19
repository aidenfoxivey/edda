use std::time::SystemTime;

use meshtastic::protobufs::NodeInfo;
use meshtastic::types::NodeId;

/// Events originating from the user interface and going to the Meshtastic thread.
pub enum UiEvent {
    Message { node_id: NodeId, message: String },
}

/// Events originating from the Meshtastic thread going to the user interface.
pub enum MeshEvent {
    NodeAvailable(NodeInfo),
    Message { node_id: NodeId, message: String },
}

#[derive(Debug)]
pub struct Message {
    to: NodeId,
    name: String,
    ts: SystemTime,
}

#[derive(PartialEq)]
pub enum AppState {
    Loading,
    Loaded,
}

/// The specific element of the UI that is currently focused.
#[derive(PartialEq, Copy, Clone)]
pub enum Focus {
    NodeList,
    Conversation,
    Input,
}
