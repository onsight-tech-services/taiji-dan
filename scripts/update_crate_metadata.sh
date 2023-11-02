#!/usr/bin/env bash

VERSION=$1
if [ "x$VERSION" == "x" ]; then
  echo "USAGE: update_crate_metadata version"
  exit 1
fi

function update_versions {
    packages=${@:-'
   applications/validator_node_rpc
   applications/taiji_dan_app_utilities
   applications/taiji_dan_wallet_cli
   applications/taiji_dan_wallet_daemon
   applications/taiji_indexer
   applications/taiji_validator_node
   applications/taiji_validator_node_cli
   clients/taiji_indexer_client
   clients/validator_node_client
   clients/validator_node_grpc_client
   clients/wallet_daemon_client
   comms/taiji_comms_logging
   dan_layer/common_types
   dan_layer/core
   dan_layer/engine
   dan_layer/engine_types
   dan_layer/integration_tests
   dan_layer/storage
   dan_layer/storage_lmdb
   dan_layer/storage_sqlite
   dan_layer/taiji_bor
   dan_layer/template_abi
   dan_layer/template_builtin
   dan_layer/template_lib
   dan_layer/template_macros
   dan_layer/template_test_tooling
   dan_layer/transaction
   dan_layer/transaction_manifest
   dan_layer/wallet/sdk
   dan_layer/wallet/storage_sqlite
'}

  p_arr=($packages)
    for p in "${p_arr[@]}"; do
      echo "Updating $path/$p version"
      update_version ./${p}/Cargo.toml $VERSION
    done
}

function update_version {
    CARGO=$1
    VERSION=$2
    #SCRIPT='s/version\s*=\s*"\d+\.\d+\.\d+"/version = "'"$VERSION"'"/'
    SCRIPT='s/^version = "0.*$/version = "'"$VERSION"'"/'
    echo "$SCRIPT" "$CARGO"
    sed -i.bak -e "$SCRIPT" "$CARGO"
    rm $CARGO.bak
}



update_versions ${p_arr[@]}
