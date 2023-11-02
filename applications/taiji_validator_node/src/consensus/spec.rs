//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_comms::types::CommsPublicKey;
use taiji_comms_rpc_state_sync::CommsRpcStateSyncManager;
use taiji_consensus::traits::ConsensusSpec;
use taiji_epoch_manager::base_layer::EpochManagerHandle;
use taiji_state_store_sqlite::SqliteStateStore;

use crate::consensus::{
    leader_selection::RoundRobinLeaderStrategy,
    signature_service::TaijiSignatureService,
    state_manager::TaijiStateManager,
};

#[derive(Clone)]
pub struct TaijiConsensusSpec;

impl ConsensusSpec for TaijiConsensusSpec {
    type Addr = CommsPublicKey;
    type EpochManager = EpochManagerHandle;
    type LeaderStrategy = RoundRobinLeaderStrategy;
    type StateManager = TaijiStateManager;
    type StateStore = SqliteStateStore<Self::Addr>;
    type SyncManager = CommsRpcStateSyncManager<Self::EpochManager, Self::StateStore>;
    type VoteSignatureService = TaijiSignatureService;
}
