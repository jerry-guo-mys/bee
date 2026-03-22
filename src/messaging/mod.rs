//! 消息层：统一消息通道与消息类型

pub mod channels;
pub mod messages;

pub use channels::ChannelManager;
pub use messages::AppMessage;
