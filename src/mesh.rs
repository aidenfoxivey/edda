//! Handle communication with a Meshtastic device connected over serial.

use std::env;

use meshtastic::api::StreamApi;
use meshtastic::packet::PacketDestination;
use meshtastic::utils;
use tokio::sync::mpsc;

use crate::router::Router;
use crate::types::{MeshEvent, UiEvent};

#[tokio::main]
pub async fn run_meshtastic(
    mut tui_receiver: mpsc::Receiver<UiEvent>,
    tx: mpsc::Sender<MeshEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2);
    let port = args[1].clone();

    let stream_api = StreamApi::new();

    let serial_stream = utils::stream::build_serial_stream(port, None, None, None)?;
    let (mut pkt_receiver, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let mut stream_api = stream_api.configure(config_id).await?;

    let mut router = Router::new(tx);

    loop {
        tokio::select! {
            Some(packet) = pkt_receiver.recv() => {
                router.handle_packet_from_radio(packet);
            }
            Some(_tui_event) = tui_receiver.recv() => {
                stream_api.send_text(&mut router, "Hello world!".to_string(), PacketDestination::Broadcast, true, 0.into()).await?;
            }
            else => {
                break;
            }
        }
    }

    Ok(())
}
