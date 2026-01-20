//! Handle communication with a Meshtastic device connected over serial.

use std::env;
use std::time::Duration;

use meshtastic::api::StreamApi;
use meshtastic::protobufs::{NodeInfo, User};
use meshtastic::types::NodeId;
use meshtastic::utils;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::router::Router;
use crate::types::{MeshEvent, UiEvent};

#[tokio::main]
pub async fn run_meshtastic(
    rx: mpsc::Receiver<UiEvent>,
    tx: mpsc::Sender<MeshEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2);
    let arg = args[1].clone();

    if arg == "mock" {
        run_mock_meshtastic(rx, tx).await
    } else {
        run_real_meshtastic(rx, tx, arg).await
    }
}

async fn run_real_meshtastic(
    mut rx: mpsc::Receiver<UiEvent>,
    tx: mpsc::Sender<MeshEvent>,
    port: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream_api = StreamApi::new();

    let serial_stream = utils::stream::build_serial_stream(port, None, None, None)?;
    let (mut pkt_receiver, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let _stream_api = stream_api.configure(config_id).await?;

    let mut router = Router::new(tx);

    loop {
        tokio::select! {
            Some(packet) = pkt_receiver.recv() => {
                router.handle_packet_from_radio(packet);
            }
            Some(ui_event) = rx.recv() => {
                router.handle_ui_event(ui_event);
            }
            else => {
                break;
            }
        }
    }

    Ok(())
}

async fn run_mock_meshtastic(
    mut rx: mpsc::Receiver<UiEvent>,
    tx: mpsc::Sender<MeshEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock user
    #[allow(deprecated)]
    let mock_user = User {
        id: String::from("!12345678"),
        long_name: String::from("Mock User"),
        short_name: String::from("MOCK"),
        macaddr: vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xAB],
        hw_model: 0,
        is_licensed: false,
        role: 0,
        public_key: vec![],
        is_unmessagable: Some(false),
    };

    let mock_node = NodeInfo {
        num: 0x12345678,
        user: Some(mock_user),
        position: None,
        snr: 10.5,
        last_heard: 0,
        device_metrics: None,
        channel: 0,
        via_mqtt: false,
        hops_away: Some(0),
        is_favorite: false,
        is_ignored: false,
        is_key_manually_verified: false,
    };

    // Send the mock node immediately
    if let Err(e) = tx
        .send(MeshEvent::NodeAvailable(Box::new(mock_node.clone())))
        .await
    {
        log::error!("Failed to send mock node: {}", e);
        return Err(e.into());
    }

    // Set up a timer to send hello messages every 10 seconds
    let mut hello_interval = interval(Duration::from_secs(10));
    let node_id = NodeId::from(mock_node.num);

    loop {
        tokio::select! {
            _ = hello_interval.tick() => {
                let hello_message = MeshEvent::Message {
                    node_id,
                    message: String::from("Hello from mock user!"),
                };

                if let Err(e) = tx.send(hello_message).await {
                    log::error!("Failed to send mock message: {}", e);
                    break;
                }
            }
            Some(ui_event) = rx.recv() => {
                // Handle UI events normally (though in mock mode we just echo them)
                match ui_event {
                    UiEvent::Message { node_id, message } => {
                        let echo_message = MeshEvent::Message { node_id, message };
                        if let Err(e) = tx.send(echo_message).await {
                            log::error!("Failed to send mock echo: {}", e);
                        }
                    }
                }
            }
            else => {
                break;
            }
        }
    }

    Ok(())
}
