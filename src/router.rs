//! A `Router` acts as middleware that can do work whenever a given message is sent or received.

use meshtastic::errors::Error;
use meshtastic::packet::PacketRouter;
use meshtastic::protobufs::{FromRadio, MeshPacket, User, from_radio::PayloadVariant};
use meshtastic::types::NodeId;
use tokio::sync::mpsc::Sender;

use crate::types::{MeshEvent};

pub struct Router {
    user: Option<User>,
    node_num: Option<NodeId>,
    ui_channel: Sender<MeshEvent>,
}

impl Router {
    pub fn new(ui_channel: Sender<MeshEvent>) -> Self {
        Router {
            user: None,
            node_num: None,
            ui_channel,
        }
    }

    pub fn handle_packet_from_radio(&mut self, packet: FromRadio) {
        match packet.payload_variant.as_ref() {
            // TODO(aidenfoxivey): This must be turned into a logger stmt instead.
            None => panic!("Unexpected packet from_radio"),
            Some(variant) => {
                match variant {
                    PayloadVariant::Packet(_) => {}
                    PayloadVariant::MyInfo(info) => {
                        // TODO(aidenfoxivey): I don't know that this case can happen, but want to be sure.
                        if self.node_num.is_some() {
                            panic!("Node number already set. Unexpected to set again.");
                        }
                        log::info!("Setting current node num to {}", info.my_node_num);
                        self.node_num = Some(NodeId::from(info.my_node_num));
                    }
                    PayloadVariant::NodeInfo(info) => {
                        if let Some(node_num) = self.node_num
                            && node_num == info.num
                        {
                            log::info!("Receiving current node user information");
                            self.user = info.user.clone();
                        }

                        if let Err(e) = self
                            .ui_channel
                            .try_send(MeshEvent::NodeAvailable(Box::new(info.clone())))
                        {
                            log::error!("Failed to send NodeAvailable event: {}", e);
                        }
                    }
                    PayloadVariant::Config(_) => {}
                    PayloadVariant::LogRecord(_) => {}
                    PayloadVariant::ConfigCompleteId(_) => {}
                    PayloadVariant::Rebooted(_) => {}
                    PayloadVariant::ModuleConfig(_) => {}
                    PayloadVariant::Channel(_) => {}
                    PayloadVariant::QueueStatus(_) => {}
                    PayloadVariant::XmodemPacket(_) => {}
                    PayloadVariant::Metadata(_) => {}
                    PayloadVariant::MqttClientProxyMessage(_) => {}
                    PayloadVariant::FileInfo(_) => {}
                    PayloadVariant::ClientNotification(_) => {}
                    PayloadVariant::DeviceuiConfig(_) => {}
                }
            }
        }
    }
}

impl PacketRouter<(), Error> for Router {
    fn handle_packet_from_radio(&mut self, packet: FromRadio) -> Result<(), Error> {
        self.handle_packet_from_radio(packet);
        Ok(())
    }

    fn handle_mesh_packet(&mut self, _packet: MeshPacket) -> Result<(), Error> {
        todo!()
    }

    fn source_node_id(&self) -> NodeId {
        self.node_num.unwrap_or(NodeId::new(0))
    }
}
