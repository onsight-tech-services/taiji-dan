//   Copyright 2022. The Tari Project
//
//   Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//   following conditions are met:
//
//   1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//   disclaimer.
//
//   2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//   following disclaimer in the documentation and/or other materials provided with the distribution.
//
//   3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//   products derived from this software without specific prior written permission.
//
//   THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//   INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//   DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//   SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//   SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//   WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//   USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

mod dispatcher;
pub use dispatcher::MessageDispatcher;

mod error;
mod inbound;
mod outbound;

pub use error::MessagingError;
pub use inbound::InboundMessaging;
pub use outbound::OutboundMessaging;
// -----------------------
// Messaging impl
// -----------------------
use tari_comms::types::CommsPublicKey;
use tari_consensus::messages::HotstuffMessage;
use tari_dan_p2p::{NetworkAnnounce, NewTransactionMessage};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::comms::MessageChannels;

pub fn spawn(
    our_node_address: CommsPublicKey,
    channels: MessageChannels,
    message_senders: DanMessageSenders,
) -> (OutboundMessaging, JoinHandle<anyhow::Result<()>>) {
    let (outbound_msg_tx, inbound_msg_rx, outbound_hs_tx, inbound_hs_rx) = channels;
    let (loopback_sender, loopback_receiver) = mpsc::channel(100);
    let inbound = InboundMessaging::new(
        our_node_address.clone(),
        inbound_msg_rx,
        inbound_hs_rx,
        loopback_receiver,
    );
    let outbound = OutboundMessaging::new(our_node_address, outbound_msg_tx, outbound_hs_tx, loopback_sender);
    let dispatcher = MessageDispatcher::new(inbound, message_senders);
    let handle = dispatcher.spawn();
    (outbound, handle)
}

#[derive(Debug, Clone)]
pub struct DanMessageSenders {
    pub tx_consensus_message: mpsc::Sender<(CommsPublicKey, HotstuffMessage<CommsPublicKey>)>,
    pub tx_new_transaction_message: mpsc::Sender<(CommsPublicKey, NewTransactionMessage)>,
    pub tx_network_announce: mpsc::Sender<(CommsPublicKey, NetworkAnnounce<CommsPublicKey>)>,
}

#[derive(Debug)]
pub struct DanMessageReceivers {
    pub rx_consensus_message: mpsc::Receiver<(CommsPublicKey, HotstuffMessage<CommsPublicKey>)>,
    pub rx_new_transaction_message: mpsc::Receiver<(CommsPublicKey, NewTransactionMessage)>,
    pub rx_network_announce: mpsc::Receiver<(CommsPublicKey, NetworkAnnounce<CommsPublicKey>)>,
}

pub fn new_messaging_channel(size: usize) -> (DanMessageSenders, DanMessageReceivers) {
    let (tx_consensus_message, rx_consensus_message) = mpsc::channel(size);
    let (tx_new_transaction_message, rx_new_transaction_message) = mpsc::channel(size);
    let (tx_network_announce, rx_network_announce) = mpsc::channel(size);
    let senders = DanMessageSenders {
        tx_consensus_message,
        tx_new_transaction_message,
        tx_network_announce,
    };
    let receivers = DanMessageReceivers {
        rx_consensus_message,
        rx_new_transaction_message,
        rx_network_announce,
    };

    (senders, receivers)
}
