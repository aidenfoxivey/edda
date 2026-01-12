use crate::types::{MeshEvent, UiEvent};
use std::time::Duration;
/// Handle communication with a Meshtastic device connected over serial.
use tokio::sync::mpsc;
use tokio::time::sleep;

#[tokio::main]
pub async fn run_meshtastic(
    _: mpsc::Receiver<UiEvent>,
    tx: mpsc::Sender<MeshEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let test_node_id = meshtastic::types::NodeId::from(999);
    let test_node_info = meshtastic::protobufs::NodeInfo {
        num: 999,
        user: Some(meshtastic::protobufs::User {
            long_name: "PingBot".to_string(),
            short_name: "PB".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    tx.send(MeshEvent::NodeAvailable(test_node_info.clone()))
        .await?;

    loop {
        let msg = MeshEvent::Message {
            node_id: test_node_id,
            message: "Ping!".to_string(),
        };
        if tx.send(msg).await.is_err() {
            break; // channel closed, exit
        }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())

    // let args: Vec<String> = env::args().collect();
    // assert_eq!(args.len(), 2);
    // let port = args[1].clone();

    // let stream_api = StreamApi::new();

    // let serial_stream = utils::stream::build_serial_stream(port, None, None, None)?;
    // let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;

    // let config_id = utils::generate_rand_id();
    // let _stream_api = stream_api.configure(config_id).await?;

    // while let Some(decoded) = decoded_listener.recv().await {
    //     if decoded.payload_variant.is_none() {
    //         break;
    //     }

    //     match decoded.payload_variant.unwrap() {
    //         PayloadVariant::Packet(_) => {}
    //         PayloadVariant::MyInfo(_) => {}
    //         PayloadVariant::NodeInfo(node_info) => {
    //             tx.send(MeshEvent::NodeAvailable(node_info)).await?;
    //         }
    //         PayloadVariant::Config(_) => {}
    //         PayloadVariant::LogRecord(_) => {}
    //         PayloadVariant::ConfigCompleteId(_) => {}
    //         PayloadVariant::Rebooted(_) => {}
    //         PayloadVariant::ModuleConfig(_) => {}
    //         PayloadVariant::Channel(_) => {}
    //         PayloadVariant::QueueStatus(_) => {}
    //         PayloadVariant::XmodemPacket(_) => {}
    //         PayloadVariant::Metadata(_) => {}
    //         PayloadVariant::MqttClientProxyMessage(_) => {}
    //         PayloadVariant::FileInfo(_) => {}
    //         PayloadVariant::ClientNotification(_) => {}
    //         PayloadVariant::DeviceuiConfig(_) => {}
    //     }
    // }

    // Ok(())
}
