use crate::types::{MeshEvent, UiEvent};
use meshtastic::api::StreamApi;
use meshtastic::protobufs::from_radio::PayloadVariant;
use meshtastic::utils;
use std::env;
/// Handle communication with a Meshtastic device connected over serial.
use tokio::sync::mpsc;

#[tokio::main]
pub async fn run_meshtastic(
    rx: mpsc::Receiver<UiEvent>,
    tx: mpsc::Sender<MeshEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2);
    let port = args[1].clone();

    let stream_api = StreamApi::new();

    let serial_stream = utils::stream::build_serial_stream(port, None, None, None)?;
    let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let _stream_api = stream_api.configure(config_id).await?;

    while let Some(decoded) = decoded_listener.recv().await {
        if decoded.payload_variant.is_none() {
            break;
        }

        match decoded.payload_variant.unwrap() {
            PayloadVariant::Packet(_) => {}
            PayloadVariant::MyInfo(_) => {}
            PayloadVariant::NodeInfo(node_info) => {
                tx.send(MeshEvent::NodeAvailable(node_info)).await?;
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

    Ok(())
}
