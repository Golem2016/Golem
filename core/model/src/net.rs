use serde::{Deserialize, Serialize};
use ya_service_bus::RpcMessage;

pub type NodeID = String; /* TODO: proper NodeID */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageAddress {
    BroadcastAddress { distance: u32 },
    Node(NodeID),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageType {
    Request,
    Reply,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub destination: MessageAddress,
    pub module: String,
    pub method: String,
    pub reply_to: NodeID,
    pub request_id: u64,
    pub message_type: MessageType,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SendMessage {
    pub message: Message,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SendMessageError(pub String);

impl RpcMessage for SendMessage {
    const ID: &'static str = "send-message";
    type Item = (); /* TODO what should we use for replies? */
    type Error = SendMessageError;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetMessages(pub NodeID);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetMessagesError(pub String);

impl RpcMessage for GetMessages {
    const ID: &'static str = "get-messages";
    type Item = Vec<Message>;
    type Error = SendMessageError;
}

#[derive(Serialize, Deserialize, Clone)]
pub enum NetworkStatus {
    ConnectedToCentralizedServer,
    ConnectedToP2PNetwork,
    NotConnected,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetNetworkStatus {}

impl RpcMessage for GetNetworkStatus {
    const ID: &'static str = "get-network-status";
    type Item = NetworkStatus;
    type Error = ();
}