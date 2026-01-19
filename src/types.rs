use meshtastic::protobufs::NodeInfo;
use meshtastic::types::NodeId;

/// Events originating from the user interface and going to the Meshtastic thread.
pub enum UiEvent {
    Message { node_id: NodeId, message: String },
}

/// Events originating from the Meshtastic thread going to the user interface.
pub enum MeshEvent {
    NodeAvailable(Box<NodeInfo>),
    Message { node_id: NodeId, message: String },
}
