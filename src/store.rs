use meshtastic::protobufs::NodeInfo;
use std::collections::HashMap;

pub trait Store {
    fn upsert_node(&mut self, node_info: NodeInfo);
    fn get_nodes(&self) -> HashMap<u32, NodeInfo>;
}

pub struct InMemoryStore {
    data: HashMap<u32, NodeInfo>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl Store for InMemoryStore {
    fn upsert_node(&mut self, node_info: NodeInfo) {
        self.data.insert(node_info.num, node_info);
    }

    fn get_nodes(&self) -> HashMap<u32, NodeInfo> {
        self.data.clone()
    }
}
