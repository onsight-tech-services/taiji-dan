//  Copyright 2022, The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::time::Duration;

use log::*;
use tari_base_node_client::{
    grpc::GrpcBaseNodeClient,
    types::{BaseLayerMetadata, BlockInfo},
    BaseNodeClient,
    BaseNodeClientError,
};
use tari_common_types::types::{Commitment, FixedHash, FixedHashSizeError, PublicKey};
use tari_core::transactions::transaction_components::{
    CodeTemplateRegistration,
    SideChainFeature,
    TransactionOutput,
    ValidatorNodeRegistration,
};
use tari_crypto::tari_utilities::ByteArray;
use tari_dan_common_types::{optional::Optional, Epoch, NodeHeight};
use tari_dan_storage::{
    consensus_models::{Block, SubstateRecord},
    global::{GlobalDb, MetadataKey},
    StateStore,
    StorageError,
};
use tari_dan_storage_sqlite::{error::SqliteStorageError, global::SqliteGlobalDbAdapter};
use tari_engine_types::{
    confidential::UnclaimedConfidentialOutput,
    substate::{SubstateAddress, SubstateValue},
};
use tari_epoch_manager::{base_layer::EpochManagerHandle, EpochManagerError};
use tari_shutdown::ShutdownSignal;
use tari_state_store_sqlite::SqliteStateStore;
use tari_template_lib::models::{EncryptedData, TemplateAddress, UnclaimedConfidentialOutputAddress};
use tokio::{task, task::JoinHandle, time};

use crate::{
    consensus_constants::ConsensusConstants,
    template_manager::interface::{TemplateManagerError, TemplateManagerHandle, TemplateRegistration},
};

const LOG_TARGET: &str = "tari::dan::base_layer_scanner";

pub fn spawn(
    global_db: GlobalDb<SqliteGlobalDbAdapter>,
    base_node_client: GrpcBaseNodeClient,
    epoch_manager: EpochManagerHandle,
    template_manager: TemplateManagerHandle,
    shutdown: ShutdownSignal,
    consensus_constants: ConsensusConstants,
    shard_store: SqliteStateStore<PublicKey>,
    scan_base_layer: bool,
    base_layer_scanning_interval: Duration,
) -> JoinHandle<anyhow::Result<()>> {
    task::spawn(async move {
        let base_layer_scanner = BaseLayerScanner::new(
            global_db,
            base_node_client,
            epoch_manager,
            template_manager,
            shutdown,
            consensus_constants,
            shard_store,
            scan_base_layer,
            base_layer_scanning_interval,
        );

        base_layer_scanner.start().await?;
        Ok(())
    })
}

pub struct BaseLayerScanner {
    global_db: GlobalDb<SqliteGlobalDbAdapter>,
    last_scanned_height: u64,
    last_scanned_tip: Option<FixedHash>,
    last_scanned_hash: Option<FixedHash>,
    next_block_hash: Option<FixedHash>,
    base_node_client: GrpcBaseNodeClient,
    epoch_manager: EpochManagerHandle,
    template_manager: TemplateManagerHandle,
    shutdown: ShutdownSignal,
    consensus_constants: ConsensusConstants,
    state_store: SqliteStateStore<PublicKey>,
    scan_base_layer: bool,
    base_layer_scanning_interval: Duration,
    has_attempted_scan: bool,
}

impl BaseLayerScanner {
    pub fn new(
        global_db: GlobalDb<SqliteGlobalDbAdapter>,
        base_node_client: GrpcBaseNodeClient,
        epoch_manager: EpochManagerHandle,
        template_manager: TemplateManagerHandle,
        shutdown: ShutdownSignal,
        consensus_constants: ConsensusConstants,
        state_store: SqliteStateStore<PublicKey>,
        scan_base_layer: bool,
        base_layer_scanning_interval: Duration,
    ) -> Self {
        Self {
            global_db,
            last_scanned_tip: None,
            last_scanned_height: 0,
            last_scanned_hash: None,
            next_block_hash: None,
            base_node_client,
            epoch_manager,
            template_manager,
            shutdown,
            consensus_constants,
            state_store,
            scan_base_layer,
            base_layer_scanning_interval,
            has_attempted_scan: false,
        }
    }

    pub async fn start(mut self) -> Result<(), BaseLayerScannerError> {
        if !self.scan_base_layer {
            info!(
                target: LOG_TARGET,
                "⚠️ scan_base_layer turned OFF. Base layer scanner is exiting."
            );
            return Ok(());
        }

        self.load_initial_state()?;
        // Scan on startup
        if let Err(err) = self.scan_blockchain().await {
            error!(target: LOG_TARGET, "Base layer scanner failed with error: {}", err);
        }

        loop {
            tokio::select! {
                _ = time::sleep(self.base_layer_scanning_interval) => {
                    if let Err(err) = self.scan_blockchain().await {
                        error!(target: LOG_TARGET, "Base layer scanner failed with error: {}", err);
                    }
                },
                _ = self.shutdown.wait() => break
            }
        }

        Ok(())
    }

    fn load_initial_state(&mut self) -> Result<(), BaseLayerScannerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut metadata = self.global_db.metadata(&mut tx);

        self.last_scanned_tip = metadata.get_metadata(MetadataKey::BaseLayerScannerLastScannedTip)?;
        self.last_scanned_hash = metadata.get_metadata(MetadataKey::BaseLayerScannerLastScannedBlockHash)?;
        self.last_scanned_height = metadata
            .get_metadata(MetadataKey::BaseLayerScannerLastScannedBlockHeight)?
            .unwrap_or(0);
        self.next_block_hash = metadata.get_metadata(MetadataKey::BaseLayerScannerNextBlockHash)?;
        Ok(())
    }

    async fn scan_blockchain(&mut self) -> Result<(), BaseLayerScannerError> {
        // fetch the new base layer info since the previous scan
        let tip = self.base_node_client.get_tip_info().await?;

        match self.get_blockchain_progression(&tip).await? {
            BlockchainProgression::Progressed => {
                info!(
                    target: LOG_TARGET,
                    "⛓️ Blockchain has progressed to height {}. We last scanned {}/{}. Scanning for new side-chain \
                     UTXOs.",
                    tip.height_of_longest_chain,
                    self.last_scanned_height,
                    tip.height_of_longest_chain
                        .saturating_sub(self.consensus_constants.base_layer_confirmations)
                );
                self.sync_blockchain().await?;
            },
            BlockchainProgression::Reorged => {
                error!(
                    target: LOG_TARGET,
                    "⚠️ Base layer reorg detected. Rescanning from genesis."
                );
                // TODO: we need to figure out where the fork happened, and delete data after the fork.
                self.last_scanned_hash = None;
                self.last_scanned_height = 0;
                self.sync_blockchain().await?;
            },
            BlockchainProgression::NoProgress => {
                trace!(target: LOG_TARGET, "No new blocks to scan.");
                // If no progress has been made since restarting, we still need to tell the epoch manager that scanning
                // is done
                if !self.has_attempted_scan {
                    self.epoch_manager.notify_scanning_complete().await?;
                }
            },
        }

        self.has_attempted_scan = false;

        Ok(())
    }

    async fn get_blockchain_progression(
        &mut self,
        tip: &BaseLayerMetadata,
    ) -> Result<BlockchainProgression, BaseLayerScannerError> {
        match self.last_scanned_tip {
            Some(hash) if hash == tip.tip_hash => Ok(BlockchainProgression::NoProgress),
            Some(hash) => {
                let header = self.base_node_client.get_header_by_hash(hash).await.optional()?;
                if header.is_some() {
                    Ok(BlockchainProgression::Progressed)
                } else {
                    Ok(BlockchainProgression::Reorged)
                }
            },
            None => Ok(BlockchainProgression::Progressed),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn sync_blockchain(&mut self) -> Result<(), BaseLayerScannerError> {
        let start_scan_height = self.last_scanned_height;
        let mut current_hash = self.last_scanned_hash;
        let tip = self.base_node_client.get_tip_info().await?;
        let end_height = match tip
            .height_of_longest_chain
            .checked_sub(self.consensus_constants.base_layer_confirmations)
        {
            None => {
                debug!(
                    target: LOG_TARGET,
                    "Base layer blockchain is not yet at the required height to start scanning it"
                );
                return Ok(());
            },
            Some(end_height) => end_height,
        };

        for current_height in start_scan_height..=end_height {
            let utxos = self
                .base_node_client
                .get_sidechain_utxos(current_hash, 1)
                .await?
                .pop()
                .ok_or_else(|| {
                    BaseLayerScannerError::InvalidSideChainUtxoResponse(format!(
                        "Base layer returned empty response for height {}",
                        current_height
                    ))
                })?;
            let block_info = utxos.block_info;
            // TODO: Because we don't know the next hash when we're done scanning to the tip, we need to load the
            //       previous scanned block again to get it.  This isn't ideal, but won't be an issue when we scan a few
            //       blocks back.
            if self.last_scanned_hash.map(|h| h == block_info.hash).unwrap_or(false) {
                if let Some(hash) = block_info.next_block_hash {
                    current_hash = Some(hash);
                    continue;
                }
                break;
            }
            info!(
                target: LOG_TARGET,
                "⛓️ Scanning base layer block {} of {}", block_info.height, end_height
            );

            for output in utxos.outputs {
                let output_hash = output.hash();
                let Some(sidechain_feature) = output.features.sidechain_feature.as_ref() else {
                    warn!(target: LOG_TARGET, "Base node returned invalid data: Sidechain utxo output must have sidechain features");
                    continue;
                };
                match sidechain_feature {
                    SideChainFeature::ValidatorNodeRegistration(reg) => {
                        self.register_validator_node_registration(current_height, reg.clone())
                            .await?;
                    },
                    SideChainFeature::CodeTemplateRegistration(reg) => {
                        self.register_code_template_registration(
                            reg.template_name.to_string(),
                            (*output_hash).into(),
                            reg.clone(),
                            &block_info,
                        )
                        .await?;
                    },
                    SideChainFeature::ConfidentialOutput(_data) => {
                        // Should be checked by the base layer
                        if !output.is_burned() {
                            warn!(
                                target: LOG_TARGET,
                                "Ignoring confidential output that is not burned: {} with commitment {}",
                                output_hash,
                                output.commitment.as_public_key()
                            );
                            continue;
                        }
                        info!(
                            target: LOG_TARGET,
                            "Found burned output: {} with commitment {}",
                            output_hash,
                            output.commitment.as_public_key()
                        );
                        self.register_burnt_utxo(&output).await?;
                    },
                }
            }

            // Once we have all the UTXO data, we "activate" the new epoch if applicable.
            self.epoch_manager
                .update_epoch(block_info.height, block_info.hash)
                .await?;

            self.set_last_scanned_block(tip.tip_hash, &block_info)?;

            match block_info.next_block_hash {
                Some(next_hash) => {
                    current_hash = Some(next_hash);
                },
                None => {
                    info!(
                        target: LOG_TARGET,
                        "⛓️ No more blocks to scan. Last scanned block height: {}", block_info.height
                    );
                    if block_info.height != end_height {
                        return Err(BaseLayerScannerError::InvalidSideChainUtxoResponse(format!(
                            "Expected to scan to height {}, but got to height {}",
                            end_height, block_info.height
                        )));
                    }
                    break;
                },
            }
        }

        self.epoch_manager.notify_scanning_complete().await?;

        Ok(())
    }

    async fn register_burnt_utxo(&mut self, output: &TransactionOutput) -> Result<(), BaseLayerScannerError> {
        let address = SubstateAddress::UnclaimedConfidentialOutput(
            UnclaimedConfidentialOutputAddress::try_from_commitment(output.commitment.as_bytes()).map_err(|e|
                // Technically impossible, but anyway
                BaseLayerScannerError::InvalidSideChainUtxoResponse(format!("Invalid commitment: {}", e)))?,
        );

        let substate = SubstateValue::UnclaimedConfidentialOutput(UnclaimedConfidentialOutput {
            commitment: output.commitment.clone(),
            encrypted_data: EncryptedData(output.encrypted_data.to_bytes()),
        });
        self.state_store
            .with_write_tx(|tx| {
                let genesis = Block::<PublicKey>::genesis();

                // TODO: This should be proposed in a block...
                SubstateRecord {
                    address,
                    version: 0,
                    substate_value: substate,
                    state_hash: Default::default(),
                    created_by_transaction: Default::default(),
                    created_justify: *genesis.justify().id(),
                    created_block: *genesis.id(),
                    created_height: NodeHeight::zero(),
                    created_at_epoch: Epoch(0),
                    destroyed: None,
                }
                .create(tx)
            })
            .map_err(|source| BaseLayerScannerError::CouldNotRegisterBurntUtxo {
                commitment: Box::new(output.commitment.clone()),
                source,
            })?;
        Ok(())
    }

    async fn register_validator_node_registration(
        &mut self,
        height: u64,
        registration: ValidatorNodeRegistration,
    ) -> Result<(), BaseLayerScannerError> {
        info!(
            target: LOG_TARGET,
            "⛓️ Validator node registration UTXO for {} found at height {}",
            registration.public_key(),
            height,
        );

        self.epoch_manager
            .add_validator_node_registration(height, registration)
            .await?;

        Ok(())
    }

    async fn register_code_template_registration(
        &mut self,
        template_name: String,
        template_address: TemplateAddress,
        registration: CodeTemplateRegistration,
        block_info: &BlockInfo,
    ) -> Result<(), BaseLayerScannerError> {
        info!(
            target: LOG_TARGET,
            "🌠 new template found with address {} at height {}", template_address, block_info.height
        );
        let template = TemplateRegistration {
            template_name,
            template_address,
            registration,
            mined_height: block_info.height,
            mined_hash: block_info.hash,
        };
        self.template_manager.add_template(template).await?;

        Ok(())
    }

    fn set_last_scanned_block(&mut self, tip: FixedHash, block_info: &BlockInfo) -> Result<(), BaseLayerScannerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut metadata = self.global_db.metadata(&mut tx);
        metadata.set_metadata(MetadataKey::BaseLayerScannerLastScannedTip, &tip)?;
        metadata.set_metadata(MetadataKey::BaseLayerScannerLastScannedBlockHash, &block_info.hash)?;
        metadata.set_metadata(MetadataKey::BaseLayerScannerNextBlockHash, &block_info.next_block_hash)?;
        metadata.set_metadata(MetadataKey::BaseLayerScannerLastScannedBlockHeight, &block_info.height)?;
        self.global_db.commit(tx)?;
        self.last_scanned_tip = Some(tip);
        self.last_scanned_hash = Some(block_info.hash);
        self.next_block_hash = block_info.next_block_hash;
        self.last_scanned_height = block_info.height;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BaseLayerScannerError {
    #[error(transparent)]
    FixedHashSizeError(#[from] FixedHashSizeError),
    #[error("Storage error: {0}")]
    SqliteStorageError(#[from] SqliteStorageError),
    #[error("Epoch manager error: {0}")]
    EpochManagerError(#[from] EpochManagerError),
    #[error("Template manager error: {0}")]
    TemplateManagerError(#[from] TemplateManagerError),
    #[error("Base node client error: {0}")]
    BaseNodeError(#[from] BaseNodeClientError),
    #[error("Invalid side chain utxo response: {0}")]
    InvalidSideChainUtxoResponse(String),
    #[error("Could not register burnt UTXO because {source}")]
    CouldNotRegisterBurntUtxo {
        commitment: Box<Commitment>,
        source: StorageError,
    },
}

enum BlockchainProgression {
    /// The blockchain has progressed since the last scan
    Progressed,
    /// Reorg was detected
    Reorged,
    /// The blockchain has not progressed since the last scan
    NoProgress,
}
