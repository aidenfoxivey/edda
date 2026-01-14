use crate::types::{MeshEvent, UiEvent};
use crate::router::Router;
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
    let (mut pkt_receiver, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let _stream_api = stream_api.configure(config_id).await?;

    let mut router = Router::new(tx);

    while let Some(packet) = pkt_receiver.recv().await {
        router.handle_packet_from_radio(packet);
    }

    Ok(())
}
