//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_common_types::types::PrivateKey;
use taiji_dan_common_types::optional::Optional;
use taiji_dan_wallet_sdk::{
    models::TransactionStatus,
    storage::{WalletStore, WalletStoreReader, WalletStoreWriter},
};
use taiji_dan_wallet_storage_sqlite::SqliteWalletStore;
use taiji_transaction::{Transaction, TransactionId};

fn build_transaction() -> Transaction {
    Transaction::builder().sign(&PrivateKey::default()).build()
}

#[test]
fn get_and_insert_transaction() {
    let db = SqliteWalletStore::try_open(":memory:").unwrap();
    db.run_migrations().unwrap();
    let mut tx = db.create_write_tx().unwrap();
    let transaction = tx.transaction_get(TransactionId::default()).optional().unwrap();
    assert!(transaction.is_none());
    let transaction = build_transaction();
    let hash = *transaction.id();
    tx.transactions_insert(&transaction, false).unwrap();
    tx.commit().unwrap();

    let mut tx = db.create_read_tx().unwrap();
    let returned = tx.transaction_get(hash).unwrap();
    assert_eq!(transaction.id(), returned.transaction.id());
    assert_eq!(returned.status, TransactionStatus::default());
}
