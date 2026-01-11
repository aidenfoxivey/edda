/// Handle communication with a Meshtastic device connected over serial.

use tokio::sync::mpsc;
use std::env;
use meshtastic::api::StreamApi;
use meshtastic::utils;
use meshtastic::protobufs::{from_radio::PayloadVariant, NodeInfo};

#[tokio::main]
pub async fn run_meshtastic(tx: mpsc::Sender<NodeInfo>) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2);
    let port = args[1].clone();

    let stream_api = StreamApi::new();

    let serial_stream = utils::stream::build_serial_stream(port, None, None, None)?;
    let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let _stream_api = stream_api.configure(config_id).await?;

    while let Some(decoded) = decoded_listener.recv().await {
        if let Some(PayloadVariant::NodeInfo(node_info)) = decoded.payload_variant
            && tx.send(node_info.clone()).await.is_err() {
                break;
            }
    }

    Ok(())
}
