use crate::osc::OscEvent::Message;
use log::{info, trace};
use rosc::{OscMessage, OscPacket};
use tokio::sync::mpsc::UnboundedSender;
use vrchat_osc::models::{OscNode, OscRootNode};
use vrchat_osc::{ServiceType, VRChatOSC};

pub struct OscService {}

pub enum OscEvent {
    Message(OscMessage),
}

fn debug_str_osc_node(node: &OscNode, key: &str, depth: u8, is_last: bool) -> String {
    let prefix: String = if depth == 0 {
        "".to_string()
    } else {
        "  ".repeat((depth as usize) - 1) + if is_last { "└" } else { "├" }
    };
    let mut lines = vec![prefix + key];
    let mut i = 0;
    node.contents.iter().for_each(|(key, content_node)| {
        lines.push(debug_str_osc_node(
            content_node,
            key,
            depth + 1,
            i + 1 == node.contents.len(),
        ));
        i = i + 1;
    });
    lines.join("\n").to_string()
}

impl OscService {
    pub async fn process_osc(
        sender: UnboundedSender<OscEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initialize VRChatOSC instance");
        let vrchat_osc = VRChatOSC::new().await?;

        let cloned_vrchat_osc = vrchat_osc.clone();
        vrchat_osc
            .on_connect(move |res| match res {
                ServiceType::Osc(name, addr) => {
                    info!("Connected to OSC server: {} at {}", name, addr);
                    let vrchat_osc = cloned_vrchat_osc.clone();
                    // Send a message to the OSC server
                    tokio::spawn(async move {
                        vrchat_osc
                            .send_to_addr(
                                OscPacket::Message(OscMessage {
                                    addr: "/avatar/parameters/VRChatOSC".to_string(),
                                    args: vec![rosc::OscType::String("Connected".to_string())],
                                }),
                                addr,
                            )
                            .await
                            .unwrap();
                        info!("Sent message to OSC server.");
                    });
                }
                ServiceType::OscQuery(name, addr) => {
                    info!("Connected to OSCQuery server: {} at {}", name, addr);
                    let vrchat_osc = cloned_vrchat_osc.clone();
                    // Get parameters from the OSCQuery server
                    tokio::spawn(async move {
                        // NOTE: When actually retrieving parameters, you should implement retry logic here.
                        // If VRChat has just started, it is possible that valid values may not be returned immediately.
                        let params = vrchat_osc
                            .get_parameter_from_addr("/avatar/parameters", addr)
                            .await
                            .unwrap();
                        info!(
                            "Received parameters: \n{}",
                            debug_str_osc_node(&params, "/avatar/parameters", 0, false)
                        );
                    });
                }
            })
            .await;

        // Register a test service
        let root_node = OscRootNode::new().with_avatar();
        let sender_ = sender.clone();
        vrchat_osc
            .register("osc_wardrobe", root_node, move |packet| {
                if let OscPacket::Message(msg) = packet {
                    sender_.send(Message(msg)).unwrap();
                }
            })
            .await?;
        info!("Service registered.");

        // Wait for the service to be registered
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Get parameters from the registered service
        let params = vrchat_osc
            .get_parameter("/avatar/parameters", "VRChat-Client-*")
            .await?;
        info!(
            "Received parameters: \n{}",
            params
                .iter()
                .map(|(name, node)| {
                    debug_str_osc_node(node, name.to_string().as_ref(), 0, false)
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        loop {
            tokio::task::yield_now().await;
        }
    }
}
