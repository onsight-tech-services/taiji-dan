//   Copyright 2023 The Tari Project
//   SPDX-License-Identifier: BSD-3-Clause

use std::{
    cmp,
    collections::{BTreeSet, HashMap, HashSet},
    mem,
};

use indexmap::IndexMap;
use log::*;
use tari_common_types::types::PublicKey;
use tari_crypto::tari_utilities::ByteArray;
use tari_dan_common_types::{optional::Optional, Epoch};
use tari_engine_types::{
    bucket::Bucket,
    component::ComponentHeader,
    events::Event,
    fee_claim::{FeeClaim, FeeClaimAddress},
    fees::FeeReceipt,
    indexed_value::{IndexedValue, IndexedWellKnownTypes},
    lock::LockFlag,
    logs::LogEntry,
    non_fungible::NonFungibleContainer,
    non_fungible_index::NonFungibleIndex,
    proof::{ContainerRef, LockedResource, Proof},
    resource::Resource,
    resource_container::ResourceContainer,
    substate::{Substate, SubstateAddress, SubstateDiff, SubstateValue},
    transaction_receipt::TransactionReceipt,
    vault::Vault,
    virtual_substate::{VirtualSubstate, VirtualSubstateAddress},
    TemplateAddress,
};
use tari_template_lib::{
    args::MintArg,
    constants::CONFIDENTIAL_TARI_RESOURCE_ADDRESS,
    models::{
        Amount,
        BucketId,
        ComponentAddress,
        NonFungibleAddress,
        NonFungibleIndexAddress,
        ProofId,
        UnclaimedConfidentialOutputAddress,
        VaultId,
    },
    prelude::PUBLIC_IDENTITY_RESOURCE_ADDRESS,
    Hash,
};

use super::workspace::Workspace;
use crate::{
    runtime::{
        fee_state::FeeState,
        locking::LockedSubstate,
        scope::{CallFrame, CallScope},
        state_store::WorkingStateStore,
        tracker_auth::Authorization,
        ActionIdent,
        AuthorizationScope,
        RuntimeError,
        TransactionCommitError,
        VirtualSubstates,
    },
    state_store::memory::MemoryStateStore,
};

const LOG_TARGET: &str = "dan::engine::runtime::working_state";

#[derive(Debug, Clone)]
pub(super) struct WorkingState {
    events: Vec<Event>,
    logs: Vec<LogEntry>,
    buckets: HashMap<BucketId, Bucket>,
    proofs: HashMap<ProofId, Proof>,

    store: WorkingStateStore,

    claimed_confidential_outputs: Vec<UnclaimedConfidentialOutputAddress>,
    new_fee_claims: HashMap<FeeClaimAddress, FeeClaim>,
    virtual_substates: VirtualSubstates,

    last_instruction_output: Option<IndexedValue>,
    workspace: Workspace,
    call_frames: Vec<CallFrame>,
    base_call_scope: CallScope,

    fee_state: FeeState,
}

impl WorkingState {
    pub fn new(
        state_store: MemoryStateStore,
        virtual_substates: VirtualSubstates,
        initial_auth_scope: AuthorizationScope,
    ) -> Self {
        let mut base_call_scope = CallScope::new();
        base_call_scope.set_auth_scope(initial_auth_scope);

        Self {
            events: Vec::new(),
            logs: Vec::new(),
            buckets: HashMap::new(),
            proofs: HashMap::new(),

            store: WorkingStateStore::new(state_store),

            claimed_confidential_outputs: Vec::new(),
            last_instruction_output: None,

            workspace: Workspace::default(),
            virtual_substates,
            new_fee_claims: HashMap::default(),
            call_frames: Vec::new(),
            base_call_scope,
            fee_state: FeeState::new(),
        }
    }

    pub fn substate_exists(&self, address: &SubstateAddress) -> Result<bool, RuntimeError> {
        // All public identity resources exist
        if address
            .as_non_fungible_address()
            .map(|a| *a.resource_address() == PUBLIC_IDENTITY_RESOURCE_ADDRESS)
            .unwrap_or(false)
        {
            return Ok(true);
        }

        self.store.exists(address)
    }

    pub fn new_substate<K: Into<SubstateAddress>, V: Into<SubstateValue>>(
        &mut self,
        address: K,
        value: V,
    ) -> Result<(), RuntimeError> {
        let address = address.into();
        self.current_call_scope_mut()?.add_substate_to_scope(address.clone())?;
        self.store.insert(address, value.into())?;
        Ok(())
    }

    pub fn lock_substate(
        &mut self,
        addr: &SubstateAddress,
        lock_flag: LockFlag,
    ) -> Result<LockedSubstate, RuntimeError> {
        let lock_id = self.store.try_lock(addr, lock_flag)?;
        Ok(LockedSubstate::new(addr.clone(), lock_id, lock_flag))
    }

    pub fn unlock_substate(&mut self, lock: LockedSubstate) -> Result<(), RuntimeError> {
        self.store.try_unlock(lock.lock_id())?;
        Ok(())
    }

    pub fn get_component(&self, locked: &LockedSubstate) -> Result<&ComponentHeader, RuntimeError> {
        let (address, substate) = self.store.get_locked_substate(locked.lock_id())?;
        let component = substate.component().ok_or_else(|| RuntimeError::LockSubstateMismatch {
            lock_id: locked.lock_id(),
            address,
            expected_type: "Component",
        })?;
        Ok(component)
    }

    pub fn modify_component_with<R, F: FnOnce(&mut ComponentHeader) -> R>(
        &mut self,
        locked: &LockedSubstate,
        f: F,
    ) -> Result<R, RuntimeError> {
        let (address, substate_mut) = self.store.get_locked_substate_mut(locked.lock_id())?;
        let component_mut = substate_mut
            .component_mut()
            .ok_or_else(|| RuntimeError::LockSubstateMismatch {
                lock_id: locked.lock_id(),
                address,
                expected_type: "Component",
            })?;
        let before = IndexedWellKnownTypes::from_value(component_mut.state())?;
        let ret = f(component_mut);

        let indexed = IndexedWellKnownTypes::from_value(component_mut.state())?;

        for existing_vault in before.vault_ids() {
            // Vaults can never be removed from components
            if !indexed.vault_ids().contains(existing_vault) {
                return Err(RuntimeError::OrphanedSubstate {
                    address: (*existing_vault).into(),
                });
            }
        }
        self.validate_component_state(&indexed, false)?;

        Ok(ret)
    }

    pub fn get_resource(&self, locked: &LockedSubstate) -> Result<&Resource, RuntimeError> {
        let (addr, substate) = self.store.get_locked_substate(locked.lock_id())?;

        let resource = substate
            .as_resource()
            .ok_or_else(|| RuntimeError::LockSubstateMismatch {
                lock_id: locked.lock_id(),
                address: addr,
                expected_type: "Resource",
            })?;

        Ok(resource)
    }

    pub fn get_non_fungible(&self, locked: &LockedSubstate) -> Result<&NonFungibleContainer, RuntimeError> {
        let (address, value) = self.store.get_locked_substate(locked.lock_id())?;
        let non_fungible = value
            .as_non_fungible()
            .ok_or_else(|| RuntimeError::LockSubstateMismatch {
                lock_id: 0,
                address: address.clone(),
                expected_type: "NonFungible",
            })?;
        Ok(non_fungible)
    }

    pub fn get_non_fungible_mut(&mut self, locked: &LockedSubstate) -> Result<&mut NonFungibleContainer, RuntimeError> {
        let (address, value) = self.store.get_locked_substate_mut(locked.lock_id())?;
        let non_fungible = value
            .as_non_fungible_mut()
            .ok_or_else(|| RuntimeError::LockSubstateMismatch {
                lock_id: 0,
                address: address.clone(),
                expected_type: "NonFungible",
            })?;
        Ok(non_fungible)
    }

    pub fn claim_confidential_output(&mut self, addr: &UnclaimedConfidentialOutputAddress) -> Result<(), RuntimeError> {
        if self.claimed_confidential_outputs.contains(addr) {
            return Err(RuntimeError::ConfidentialOutputAlreadyClaimed { address: *addr });
        }
        self.claimed_confidential_outputs.push(*addr);
        Ok(())
    }

    pub fn get_locked_substate(&self, lock: &LockedSubstate) -> Result<&SubstateValue, RuntimeError> {
        let (_, substate) = self.store.get_locked_substate(lock.lock_id())?;
        Ok(substate)
    }

    pub fn get_vault(&self, locked: &LockedSubstate) -> Result<&Vault, RuntimeError> {
        let (addr, substate) = self.store.get_locked_substate(locked.lock_id())?;

        let vault = substate.as_vault().ok_or_else(|| RuntimeError::LockSubstateMismatch {
            lock_id: locked.lock_id(),
            address: addr,
            expected_type: "Vault",
        })?;

        Ok(vault)
    }

    pub fn get_vault_mut(&mut self, locked: &LockedSubstate) -> Result<&mut Vault, RuntimeError> {
        let (addr, substate) = self.store.get_locked_substate_mut(locked.lock_id())?;

        let vault_mut = substate
            .as_vault_mut()
            .ok_or_else(|| RuntimeError::LockSubstateMismatch {
                lock_id: locked.lock_id(),
                address: addr,
                expected_type: "Vault",
            })?;

        Ok(vault_mut)
    }

    pub fn get_resource_mut(&mut self, locked: &LockedSubstate) -> Result<&mut Resource, RuntimeError> {
        let (addr, substate) = self.store.get_locked_substate_mut(locked.lock_id())?;

        let resource_mut = substate
            .as_resource_mut()
            .ok_or_else(|| RuntimeError::LockSubstateMismatch {
                lock_id: locked.lock_id(),
                address: addr,
                expected_type: "Resource",
            })?;

        Ok(resource_mut)
    }

    pub fn get_current_epoch(&self) -> Result<Epoch, RuntimeError> {
        let address = VirtualSubstateAddress::CurrentEpoch;
        let current_epoch =
            self.virtual_substates
                .get(&address)
                .ok_or_else(|| RuntimeError::VirtualSubstateNotFound {
                    address: address.clone(),
                })?;
        let VirtualSubstate::CurrentEpoch(epoch) = current_epoch else {
            return Err(RuntimeError::VirtualSubstateNotFound { address });
        };
        Ok(Epoch(*epoch))
    }

    pub(super) fn validate_finalized(&self) -> Result<(), RuntimeError> {
        if !self.buckets.is_empty() {
            return Err(TransactionCommitError::DanglingBuckets {
                count: self.buckets.len(),
            }
            .into());
        }

        if !self.proofs.is_empty() {
            return Err(TransactionCommitError::DanglingProofs {
                count: self.proofs.len(),
            }
            .into());
        }

        for (_, vault) in self.store.new_vaults() {
            if !vault.locked_balance().is_zero() {
                return Err(TransactionCommitError::DanglingLockedValueInVault {
                    vault_id: *vault.vault_id(),
                    locked_amount: vault.locked_balance(),
                }
                .into());
            }
        }

        if self.call_frame_depth() != 0 {
            return Err(RuntimeError::CallFrameRemainingOnStack {
                remaining: self.call_frame_depth(),
            });
        }
        // Final call frame can be none if there are no instructions (due to either fee instructions or instructions
        // being empty)
        let call_scope = self.base_call_scope();
        if !call_scope.orphans().is_empty() {
            return Err(TransactionCommitError::OrphanedSubstates {
                substates: call_scope.orphans().iter().map(ToString::to_string).collect(),
            }
            .into());
        }

        Ok(())
    }

    pub fn get_proof(&self, proof_id: ProofId) -> Result<&Proof, RuntimeError> {
        self.proofs
            .get(&proof_id)
            .ok_or(RuntimeError::ProofNotFound { proof_id })
    }

    pub fn proof_exists(&self, proof_id: ProofId) -> bool {
        self.proofs.contains_key(&proof_id)
    }

    pub fn get_bucket(&self, bucket_id: BucketId) -> Result<&Bucket, RuntimeError> {
        if !self.current_call_scope()?.is_bucket_in_scope(bucket_id) {
            return Err(RuntimeError::BucketNotFound { bucket_id });
        }
        self.buckets
            .get(&bucket_id)
            .ok_or(RuntimeError::BucketNotFound { bucket_id })
    }

    pub fn get_bucket_mut(&mut self, bucket_id: BucketId) -> Result<&mut Bucket, RuntimeError> {
        if !self.current_call_scope()?.is_bucket_in_scope(bucket_id) {
            return Err(RuntimeError::BucketNotFound { bucket_id });
        }
        self.buckets
            .get_mut(&bucket_id)
            .ok_or(RuntimeError::BucketNotFound { bucket_id })
    }

    pub fn take_bucket(&mut self, bucket_id: BucketId) -> Result<Bucket, RuntimeError> {
        if !self.current_call_scope()?.is_bucket_in_scope(bucket_id) {
            return Err(RuntimeError::BucketNotFound { bucket_id });
        }
        self.current_call_scope_mut()?.remove_bucket_from_scope(bucket_id);
        self.buckets
            .remove(&bucket_id)
            .ok_or(RuntimeError::BucketNotFound { bucket_id })
    }

    pub fn burn_bucket(&mut self, bucket: Bucket) -> Result<(), RuntimeError> {
        if bucket.amount().is_zero() {
            return Ok(());
        }
        let resource_address = *bucket.resource_address();
        // Burn Non-fungibles (if resource is nf). Fungibles are burnt by removing the bucket from the tracker state
        // and not depositing it.
        for token_id in bucket.into_non_fungible_ids().into_iter().flatten() {
            let address = NonFungibleAddress::new(resource_address, token_id);
            let locked_nft = self.lock_substate(&SubstateAddress::NonFungible(address.clone()), LockFlag::Write)?;
            let nft = self.get_non_fungible_mut(&locked_nft)?;

            if nft.is_burnt() {
                return Err(RuntimeError::InvalidOpNonFungibleBurnt {
                    op: "burn_bucket",
                    resource_address,
                    nf_id: address.id().clone(),
                });
            }
            nft.burn();
            self.unlock_substate(locked_nft)?;
        }

        Ok(())
    }

    pub fn drop_proof(&mut self, proof_id: ProofId) -> Result<(), RuntimeError> {
        // Remove it from the auth scope if is in scope
        let call_frame_mut = self.current_call_scope_mut()?;
        if !call_frame_mut.is_proof_in_scope(proof_id) {
            return Err(RuntimeError::ProofNotFound { proof_id });
        }
        call_frame_mut.auth_scope_mut().remove_proof(&proof_id);

        // Fetch the proof
        let proof = self
            .proofs
            .remove(&proof_id)
            .ok_or(RuntimeError::ProofNotFound { proof_id })?;

        // Unlock funds
        match *proof.container() {
            ContainerRef::Bucket(bucket_id) => {
                self.buckets
                    .get_mut(&bucket_id)
                    .ok_or(RuntimeError::BucketNotFound { bucket_id })?
                    .unlock(proof)?;
            },
            ContainerRef::Vault(vault_id) => {
                let vault_lock = self.lock_substate(&SubstateAddress::Vault(vault_id), LockFlag::Write)?;
                self.get_vault_mut(&vault_lock)?.unlock(proof)?;
                self.unlock_substate(vault_lock)?;
            },
        }

        Ok(())
    }

    pub fn mint_resource(
        &mut self,
        locked_resource: &LockedSubstate,
        mint_arg: MintArg,
    ) -> Result<ResourceContainer, RuntimeError> {
        let resource_address =
            locked_resource
                .address()
                .as_resource_address()
                .ok_or_else(|| RuntimeError::InvariantError {
                    function: "mint_resource",
                    details: "LockedSubstate address is not a ResourceAddress".to_string(),
                })?;

        let resource_container = match mint_arg {
            MintArg::Fungible { amount } => {
                if amount.is_negative() {
                    return Err(RuntimeError::InvalidAmount {
                        amount,
                        reason: "Amount must be positive".to_string(),
                    });
                }

                debug!(
                    target: LOG_TARGET,
                    "Minting {} fungible tokens on resource: {}", amount, resource_address
                );

                ResourceContainer::fungible(resource_address, amount)
            },
            MintArg::NonFungible { tokens } => {
                debug!(
                    target: LOG_TARGET,
                    "Minting {} NFT token(s) on resource: {}",
                    tokens.len(),
                    resource_address
                );
                let mut token_ids = BTreeSet::new();

                let resource = self.get_resource(locked_resource)?;
                // TODO: This isn't correct (assumes tokens are never burnt), we'll need to rethink this
                let mut index = resource
                    .total_supply()
                    .as_u64_checked()
                    .ok_or(RuntimeError::InvalidAmount {
                        amount: resource.total_supply(),
                        reason: "Could not convert to u64".to_owned(),
                    })?;

                for (id, (data, mut_data)) in tokens {
                    let nft_address = NonFungibleAddress::new(resource_address, id.clone());
                    let addr = SubstateAddress::NonFungible(nft_address.clone());
                    if self.substate_exists(&addr)? {
                        return Err(RuntimeError::DuplicateNonFungibleId {
                            token_id: nft_address.id().clone(),
                        });
                    }
                    self.new_substate(addr.clone(), NonFungibleContainer::new(data, mut_data))?;
                    if !token_ids.insert(id.clone()) {
                        return Err(RuntimeError::DuplicateNonFungibleId { token_id: id });
                    }

                    // for each new nft we also create an index to be allow resource scanning
                    let index_address = NonFungibleIndexAddress::new(resource_address, index);
                    index += 1;
                    let nft_index = NonFungibleIndex::new(nft_address);
                    self.new_substate(index_address.clone(), nft_index)?;
                }

                ResourceContainer::non_fungible(resource_address, token_ids)
            },
            MintArg::Confidential { proof } => {
                debug!(
                    target: LOG_TARGET,
                    "Minting confidential tokens on resource: {}", resource_address
                );
                ResourceContainer::validate_confidential_mint(resource_address, *proof)?
            },
        };

        // Increase the total supply, this also validates that the resource already exists.
        {
            let resource_mut = self.get_resource_mut(locked_resource)?;
            resource_mut.increase_total_supply(resource_container.amount());
        }

        Ok(resource_container)
    }

    pub fn new_bucket(&mut self, bucket_id: BucketId, resource: ResourceContainer) -> Result<(), RuntimeError> {
        debug!(
            target: LOG_TARGET,
            "New bucket {} for resource {} {:?}", bucket_id, resource.resource_address(), resource.resource_type()
        );

        // Mark Resource and NFT substates as owned since they are going into a bucket
        {
            let scope_mut = self.current_call_scope_mut()?;
            scope_mut.move_node_to_owned(&(*resource.resource_address()).into())?;
            for id in resource.non_fungible_token_ids() {
                scope_mut
                    .move_node_to_owned(&NonFungibleAddress::new(*resource.resource_address(), id.clone()).into())?;
            }
        }

        let bucket = Bucket::new(bucket_id, resource);
        if self.buckets.insert(bucket_id, bucket).is_some() {
            return Err(RuntimeError::DuplicateBucket { bucket_id });
        }
        self.current_call_scope_mut()?.add_bucket_to_scope(bucket_id);
        Ok(())
    }

    pub fn new_proof(&mut self, proof_id: ProofId, locked_funds: LockedResource) -> Result<(), RuntimeError> {
        debug!(target: LOG_TARGET, "New proof {}", proof_id);
        if self.proofs.insert(proof_id, Proof::new(locked_funds)).is_some() {
            return Err(RuntimeError::DuplicateProof { proof_id });
        }

        self.current_call_scope_mut()?.add_proof_to_scope(proof_id);
        Ok(())
    }

    pub fn pay_fee(&mut self, resource: ResourceContainer, return_vault: VaultId) -> Result<(), RuntimeError> {
        self.fee_state.fee_payments.push((resource, return_vault));
        Ok(())
    }

    pub fn take_fee_claim(&mut self, epoch: Epoch, validator_public_key: PublicKey) -> Result<FeeClaim, RuntimeError> {
        let substate = self
            .virtual_substates
            .remove(&VirtualSubstateAddress::UnclaimedValidatorFee {
                epoch: epoch.as_u64(),
                address: validator_public_key.clone(),
            })
            .ok_or(RuntimeError::FeeClaimNotPermitted {
                epoch,
                address: validator_public_key.clone(),
            })?;

        let VirtualSubstate::UnclaimedValidatorFee(fee_claim) = substate else {
            return Err(RuntimeError::FeeClaimNotPermitted {
                epoch,
                address: validator_public_key,
            });
        };

        Ok(fee_claim)
    }

    pub fn claim_fee(
        &mut self,
        epoch: Epoch,
        validator_public_key: PublicKey,
    ) -> Result<ResourceContainer, RuntimeError> {
        let fee_claim_addr = FeeClaimAddress::from_addr(epoch.as_u64(), validator_public_key.as_bytes());
        let claim = self.take_fee_claim(epoch, validator_public_key.clone())?;
        let amount = claim.amount;
        if self.new_fee_claims.insert(fee_claim_addr, claim).is_some() {
            return Err(RuntimeError::DoubleClaimedFee {
                address: validator_public_key,
                epoch,
            });
        }
        Ok(ResourceContainer::confidential(
            CONFIDENTIAL_TARI_RESOURCE_ADDRESS,
            None,
            amount,
        ))
    }

    pub fn validate_component_state(
        &mut self,
        indexed: &IndexedWellKnownTypes,
        require_in_scope: bool,
    ) -> Result<(), RuntimeError> {
        let mut dup_check = HashSet::with_capacity(indexed.vault_ids().len());
        for vault_id in indexed.vault_ids() {
            if !dup_check.insert(vault_id) {
                return Err(RuntimeError::DuplicateReference {
                    address: (*vault_id).into(),
                });
            }
        }
        // TODO: I think that we can clean this up a bit. We should always be checking the scope but there are edge
        //       cases and it was just easier to have this conditional
        if require_in_scope {
            self.check_all_substates_in_scope(indexed)?;
        } else {
            self.check_all_substates_known(indexed)?;
        }

        let scope_mut = self.current_call_scope_mut()?;
        for address in indexed.referenced_substates() {
            // Move orphaned objects to owned
            scope_mut.move_node_to_owned(&address)?
        }

        Ok(())
    }

    pub fn authorization(&self) -> Authorization {
        Authorization::new(self)
    }

    pub fn take_mutated_substates(&mut self) -> IndexMap<SubstateAddress, SubstateValue> {
        let mut up_states = self.store.take_mutated_substates();
        up_states.extend(
            self.new_fee_claims
                .drain()
                .map(|(addr, claim)| (addr.into(), claim.into())),
        );
        up_states
    }

    pub fn fee_state(&self) -> &FeeState {
        &self.fee_state
    }

    pub fn fee_state_mut(&mut self) -> &mut FeeState {
        &mut self.fee_state
    }

    pub fn set_last_instruction_output(&mut self, output: IndexedValue) {
        self.last_instruction_output = Some(output);
    }

    pub fn finalize_fees(
        &mut self,
        transaction_hash: Hash,
        substates_to_persist: &mut IndexMap<SubstateAddress, SubstateValue>,
    ) -> Result<TransactionReceipt, RuntimeError> {
        let total_fees = self
            .fee_state
            .fee_charges
            .iter()
            .map(|(_, fee)| Amount::try_from(*fee).expect("fee overflowed i64::MAX"))
            .sum::<Amount>();
        let total_fee_payment = self
            .fee_state
            .fee_payments
            .iter()
            .map(|(resx, _)| resx.amount())
            .sum::<Amount>();

        let mut fee_resource =
            ResourceContainer::confidential(CONFIDENTIAL_TARI_RESOURCE_ADDRESS, None, Amount::zero());

        // Collect the fee
        let mut remaining_fees = total_fees;
        for (resx, _) in &mut self.fee_state.fee_payments {
            if remaining_fees.is_zero() {
                break;
            }
            let amount_to_withdraw = cmp::min(resx.amount(), remaining_fees);
            remaining_fees -= amount_to_withdraw;
            fee_resource.deposit(resx.withdraw(amount_to_withdraw)?)?;
        }

        // Refund the remaining payments if any
        for (mut resx, refund_vault) in self.fee_state.fee_payments.drain(..) {
            if resx.amount().is_zero() {
                continue;
            }

            let vault_mut = substates_to_persist
                .get_mut(&SubstateAddress::Vault(refund_vault))
                .expect("invariant: vault that made fee payment not in changeset")
                .as_vault_mut()
                .expect("invariant: substate address for fee refund is not a vault");
            vault_mut.resource_container_mut().deposit(resx.withdraw_all()?)?;
        }

        Ok(TransactionReceipt {
            transaction_hash,
            events: self.events.clone(),
            logs: self.logs.clone(),
            fee_receipt: FeeReceipt {
                total_fee_payment,
                total_fees_paid: fee_resource.amount(),
                cost_breakdown: self.fee_state.fee_charges.drain(..).collect(),
            },
        })
    }

    pub(super) fn current_call_scope_mut(&mut self) -> Result<&mut CallScope, RuntimeError> {
        Ok(self
            .call_frames
            .last_mut()
            .map(|s| s.scope_mut())
            .unwrap_or(&mut self.base_call_scope))
    }

    pub fn current_call_scope(&self) -> Result<&CallScope, RuntimeError> {
        Ok(self
            .call_frames
            .last()
            .map(|f| f.scope())
            .unwrap_or(&self.base_call_scope))
    }

    pub fn call_frame_depth(&self) -> usize {
        self.call_frames.len()
    }

    /// Returns template address and module name
    pub fn current_template(&self) -> Result<(&TemplateAddress, &str), RuntimeError> {
        self.call_frames
            .last()
            .map(|frame| frame.current_template())
            .ok_or(RuntimeError::NoActiveCallScope)
    }

    pub fn push_frame(&mut self, mut new_frame: CallFrame, max_call_depth: usize) -> Result<(), RuntimeError> {
        if self.call_frame_depth() + 1 > max_call_depth {
            return Err(RuntimeError::MaxCallDepthExceeded {
                max_depth: max_call_depth,
            });
        }

        let current = self.current_call_scope()?;
        new_frame.scope_mut().update_from_parent(current);

        if self.call_frame_depth() == 0 {
            // If this is the first call frame, then we use the base auth scope (virtual proofs are carried from the
            // base to the first call scope)
            new_frame
                .scope_mut()
                .set_auth_scope(self.base_call_scope.auth_scope().clone());
        }

        self.call_frames.push(new_frame);
        Ok(())
    }

    pub fn pop_frame(&mut self) -> Result<(), RuntimeError> {
        let pop = self.call_frames.pop().ok_or(RuntimeError::NoActiveCallScope)?;

        let scope = pop.into_scope();
        // Unlock the component
        if let Some(component_lock) = scope.get_current_component_lock() {
            self.unlock_substate(component_lock.clone())?;
        }

        if !scope.lock_scope().is_empty() {
            return Err(RuntimeError::DanglingSubstateLocks {
                count: scope.lock_scope().len(),
            });
        }

        if !scope.orphans().is_empty() {
            return Err(RuntimeError::OrphanedSubstates {
                substates: scope.orphans().iter().map(ToString::to_string).collect(),
            });
        }

        // Update the parent call scope
        debug!(target: LOG_TARGET, "pop_frame:\n{}", scope);
        self.current_call_scope_mut()?.update_from_child_scope(scope);

        Ok(())
    }

    pub fn base_call_scope(&self) -> &CallScope {
        &self.base_call_scope
    }

    pub fn take_state(&mut self) -> Self {
        let new_state = WorkingState::new(
            self.store.state_store().clone(),
            Default::default(),
            AuthorizationScope::new(vec![]),
        );
        mem::replace(self, new_state)
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    pub fn take_last_instruction_output(&mut self) -> Option<IndexedValue> {
        self.last_instruction_output.take()
    }

    pub fn load_component(&mut self, component_address: &ComponentAddress) -> Result<ComponentHeader, RuntimeError> {
        self.store.load_component(component_address).cloned()
    }

    pub fn check_all_substates_known(&self, value: &IndexedWellKnownTypes) -> Result<(), RuntimeError> {
        for addr in value.referenced_substates() {
            if !self.substate_exists(&addr)? {
                return Err(RuntimeError::SubstateNotFound { address: addr.clone() });
            }
        }
        for bucket_id in value.bucket_ids() {
            if !self.buckets().contains_key(bucket_id) {
                return Err(RuntimeError::ValidationFailedBucketNotInScope { bucket_id: *bucket_id });
            }
        }
        for proof_id in value.proof_ids() {
            if !self.proofs().contains_key(proof_id) {
                return Err(RuntimeError::ValidationFailedProofNotInScope { proof_id: *proof_id });
            }
        }

        Ok(())
    }

    pub fn check_all_substates_in_scope(&self, value: &IndexedWellKnownTypes) -> Result<(), RuntimeError> {
        let scope = self.current_call_scope()?;
        for addr in value.referenced_substates() {
            // You are allowed to reference root substates
            if addr.is_root() {
                if !self.substate_exists(&addr)? {
                    return Err(RuntimeError::SubstateNotFound { address: addr.clone() });
                }
            } else if !scope.is_substate_in_scope(&addr) {
                return Err(RuntimeError::SubstateOutOfScope { address: addr.clone() });
            } else {
                // OK
            }
        }
        for bucket_id in value.bucket_ids() {
            if !scope.is_bucket_in_scope(*bucket_id) {
                return Err(RuntimeError::ValidationFailedBucketNotInScope { bucket_id: *bucket_id });
            }
        }
        for proof_id in value.proof_ids() {
            if !scope.is_proof_in_scope(*proof_id) {
                return Err(RuntimeError::ValidationFailedProofNotInScope { proof_id: *proof_id });
            }
        }

        Ok(())
    }

    pub fn buckets(&self) -> &HashMap<BucketId, Bucket> {
        &self.buckets
    }

    pub fn proofs(&self) -> &HashMap<ProofId, Proof> {
        &self.proofs
    }

    pub fn push_log(&mut self, log: LogEntry) {
        self.logs.push(log);
    }

    pub fn take_logs(&mut self) -> Vec<LogEntry> {
        mem::take(&mut self.logs)
    }

    pub fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        mem::take(&mut self.events)
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn logs(&self) -> &[LogEntry] {
        &self.logs
    }

    pub fn generate_substate_diff(
        &self,
        transaction_receipt: TransactionReceipt,
        substates_to_persist: IndexMap<SubstateAddress, SubstateValue>,
    ) -> Result<SubstateDiff, RuntimeError> {
        let mut substate_diff = SubstateDiff::new();

        for (address, substate) in substates_to_persist {
            let new_substate = match self.store.get_unmodified_substate(&address).optional()? {
                Some(existing_state) => {
                    substate_diff.down(address.clone(), existing_state.version());
                    Substate::new(existing_state.version() + 1, substate)
                },
                None => Substate::new(0, substate),
            };
            substate_diff.up(address, new_substate);
        }

        // Special case: unclaimed confidential outputs are downed without being upped if claimed
        for claimed in &self.claimed_confidential_outputs {
            substate_diff.down(SubstateAddress::UnclaimedConfidentialOutput(*claimed), 0);
        }

        substate_diff.up(
            SubstateAddress::TransactionReceipt(transaction_receipt.transaction_hash.into()),
            Substate::new(0, SubstateValue::TransactionReceipt(transaction_receipt)),
        );

        Ok(substate_diff)
    }

    pub fn store(&self) -> &WorkingStateStore {
        &self.store
    }

    pub fn check_component_scope<T: Into<ActionIdent>>(
        &self,
        address: &SubstateAddress,
        action: T,
    ) -> Result<(), RuntimeError> {
        // Since we dont propagate _owned_ substate references up the call stack, if the substate is in scope, then it
        // was created in this scope and therefore owned.
        if self.current_call_scope()?.is_substate_in_scope(address) {
            return Ok(());
        }

        let component_lock = self
            .current_call_scope()?
            .get_current_component_lock()
            .ok_or(RuntimeError::NotInComponentContext { action: action.into() })?;

        let component = self.get_component(component_lock)?;
        if !component.contains_substate(address)? {
            warn!(
                target: LOG_TARGET,
                "Component {} attempted access to {} that is does not own",
                component_lock.address(),
                address
            );
            return Err(RuntimeError::SubstateNotOwned {
                address: address.clone(),
                requested_owner: component_lock.address().clone(),
            });
        }

        Ok(())
    }
}
