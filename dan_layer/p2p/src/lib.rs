//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

mod message;
mod outbound_service;
mod peer_service;

pub use message::*;
pub use outbound_service::*;
pub use peer_service::*;
use taiji_comms::protocol::ProtocolId;

pub static TAIJI_DAN_MSG_PROTOCOL_ID: ProtocolId = ProtocolId::from_static(b"t/msg/1");
pub static TAIJI_DAN_CONSENSUS_MSG_ID: ProtocolId = ProtocolId::from_static(b"t/msg-hs/1");
