use crate::osc::OscEvent::Message;
use rosc::{OscMessage, OscPacket};
use tokio::sync::mpsc::UnboundedSender;
use vrchat_osc::models::OscRootNode;
use vrchat_osc::{ServiceType, VRChatOSC};

pub struct OscService {}

pub enum OscEvent {
    Message(OscMessage),
}

impl OscService {
    pub async fn process_osc(
        sender: UnboundedSender<OscEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize VRChatOSC instance
        let vrchat_osc = VRChatOSC::new().await?;

        let cloned_vrchat_osc = vrchat_osc.clone();
        vrchat_osc
            .on_connect(move |res| match res {
                ServiceType::Osc(name, addr) => {
                    println!("Connected to OSC server: {} at {}", name, addr);
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
                        println!("Sent message to OSC server.");
                    });
                }
                ServiceType::OscQuery(name, addr) => {
                    println!("Connected to OSCQuery server: {} at {}", name, addr);
                    let vrchat_osc = cloned_vrchat_osc.clone();
                    // Get parameters from the OSCQuery server
                    tokio::spawn(async move {
                        // NOTE: When actually retrieving parameters, you should implement retry logic here.
                        // If VRChat has just started, it is possible that valid values may not be returned immediately.
                        let params = vrchat_osc
                            .get_parameter_from_addr("/avatar/parameters", addr)
                            .await
                            .unwrap();
                        println!("Received parameters: {:?}", params);
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
                    println!("Received OSC message: {:?}", msg);
                    let _ = sender_.send(Message(msg));
                }
            })
            .await?;
        println!("Service registered.");

        // Wait for the service to be registered
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Get parameters from the registered service
        let params = vrchat_osc
            .get_parameter("/avatar/parameters", "VRChat-Client-*")
            .await?;
        println!("Received parameters: {:?}", params);

        loop {
            tokio::task::yield_now().await;
        }
    }
}
