//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_dan_wallet_sdk::DanWalletSdk;
use taiji_dan_wallet_storage_sqlite::SqliteWalletStore;

use crate::{
    indexer_jrpc_impl::IndexerJsonRpcNetworkInterface,
    notify::Notify,
    services::{AccountMonitorHandle, WalletEvent},
};

#[derive(Debug, Clone)]
pub struct HandlerContext {
    wallet_sdk: DanWalletSdk<SqliteWalletStore, IndexerJsonRpcNetworkInterface>,
    notifier: Notify<WalletEvent>,
    account_monitor: AccountMonitorHandle,
}

impl HandlerContext {
    pub fn new(
        wallet_sdk: DanWalletSdk<SqliteWalletStore, IndexerJsonRpcNetworkInterface>,
        notifier: Notify<WalletEvent>,
        account_monitor: AccountMonitorHandle,
    ) -> Self {
        Self {
            wallet_sdk,
            notifier,
            account_monitor,
        }
    }

    pub fn notifier(&self) -> &Notify<WalletEvent> {
        &self.notifier
    }

    pub fn wallet_sdk(&self) -> &DanWalletSdk<SqliteWalletStore, IndexerJsonRpcNetworkInterface> {
        &self.wallet_sdk
    }

    pub fn account_monitor(&self) -> &AccountMonitorHandle {
        &self.account_monitor
    }
}
