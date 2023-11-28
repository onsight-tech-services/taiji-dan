//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use std::sync::Arc;

use taiji_common_types::types::PublicKey;
use taiji_comms::{types::CommsPublicKey, NodeIdentity};
use taiji_comms_rpc_state_sync::CommsRpcStateSyncManager;
use taiji_consensus::{
    hotstuff::{ConsensusWorker, ConsensusWorkerContext, HotstuffWorker},
    messages::HotstuffMessage,
};
use taiji_dan_common_types::committee::Committee;
use taiji_dan_p2p::{Message, OutboundService};
use taiji_dan_storage::consensus_models::TransactionPool;
use taiji_epoch_manager::base_layer::EpochManagerHandle;
use taiji_shutdown::ShutdownSignal;
use taiji_state_store_sqlite::SqliteStateStore;
use taiji_transaction::{Transaction, TransactionId};
use taiji_validator_node_rpc::client::TaijiCommsValidatorNodeClientFactory;
use tokio::{
    sync::{broadcast, mpsc, watch},
    task::JoinHandle,
};

use crate::{
    consensus::{
        leader_selection::RoundRobinLeaderStrategy,
        signature_service::TaijiSignatureService,
        spec::TaijiConsensusSpec,
        state_manager::TaijiStateManager,
    },
    event_subscription::EventSubscription,
    p2p::services::messaging::OutboundMessaging,
};

mod handle;
mod leader_selection;
mod signature_service;
mod spec;
mod state_manager;

pub use handle::*;

pub async fn spawn(
    store: SqliteStateStore<PublicKey>,
    node_identity: Arc<NodeIdentity>,
    epoch_manager: EpochManagerHandle,
    rx_new_transactions: mpsc::Receiver<TransactionId>,
    rx_hs_message: mpsc::Receiver<(CommsPublicKey, HotstuffMessage<PublicKey>)>,
    outbound_messaging: OutboundMessaging,
    client_factory: TaijiCommsValidatorNodeClientFactory,
    shutdown_signal: ShutdownSignal,
) -> (
    JoinHandle<Result<(), anyhow::Error>>,
    ConsensusHandle,
    mpsc::UnboundedReceiver<Transaction>,
) {
    let (tx_broadcast, rx_broadcast) = mpsc::channel(10);
    let (tx_leader, rx_leader) = mpsc::channel(10);
    let (tx_mempool, rx_mempool) = mpsc::unbounded_channel();

    let validator_addr = node_identity.public_key().clone();
    let signing_service = TaijiSignatureService::new(node_identity);
    let leader_strategy = RoundRobinLeaderStrategy::new();
    let transaction_pool = TransactionPool::new();
    let state_manager = TaijiStateManager::new();
    let (tx_hotstuff_events, _) = broadcast::channel(100);

    let hotstuff_worker = HotstuffWorker::<TaijiConsensusSpec>::new(
        validator_addr,
        rx_new_transactions,
        rx_hs_message,
        store.clone(),
        epoch_manager.clone(),
        leader_strategy,
        signing_service,
        state_manager,
        transaction_pool,
        tx_broadcast,
        tx_leader,
        tx_hotstuff_events.clone(),
        tx_mempool,
        shutdown_signal.clone(),
    );

    let (tx_current_state, rx_current_state) = watch::channel(Default::default());
    let context = ConsensusWorkerContext {
        epoch_manager: epoch_manager.clone(),
        hotstuff: hotstuff_worker,
        state_sync: CommsRpcStateSyncManager::new(epoch_manager, store, client_factory),
        tx_current_state,
    };

    let handle = ConsensusWorker::new(shutdown_signal).spawn(context);

    ConsensusMessageWorker {
        rx_broadcast,
        rx_leader,
        outbound_messaging,
    }
    .spawn();

    (
        handle,
        ConsensusHandle::new(rx_current_state, EventSubscription::new(tx_hotstuff_events)),
        rx_mempool,
    )
}

struct ConsensusMessageWorker {
    rx_broadcast: mpsc::Receiver<(Committee<CommsPublicKey>, HotstuffMessage<CommsPublicKey>)>,
    rx_leader: mpsc::Receiver<(CommsPublicKey, HotstuffMessage<CommsPublicKey>)>,
    outbound_messaging: OutboundMessaging,
}

impl ConsensusMessageWorker {
    fn spawn(mut self) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some((committee, msg)) = self.rx_broadcast.recv() => {
                        self.outbound_messaging
                            .broadcast(committee.members(), Message::Consensus(msg))
                            .await
                            .ok();
                    },
                    Some((dest, msg)) = self.rx_leader.recv() => {
                        self.outbound_messaging
                            .send(dest, Message::Consensus(msg))
                            .await
                            .ok();
                    },

                    else => break,
                }
            }
        });
    }
}