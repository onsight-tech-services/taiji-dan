// @generated automatically by Diesel CLI.

diesel::table! {
    blocks (id) {
        id -> Integer,
        block_id -> Text,
        parent_block_id -> Text,
        height -> BigInt,
        epoch -> BigInt,
        proposed_by -> Text,
        qc_id -> Text,
        command_count -> BigInt,
        commands -> Text,
        total_leader_fee -> BigInt,
        is_committed -> Bool,
        is_processed -> Bool,
        is_dummy -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    high_qcs (id) {
        id -> Integer,
        block_id -> Text,
        block_height -> BigInt,
        qc_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    last_executed (id) {
        id -> Integer,
        block_id -> Text,
        height -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    last_proposed (id) {
        id -> Integer,
        block_id -> Text,
        height -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    last_sent_vote (id) {
        id -> Integer,
        epoch -> BigInt,
        block_id -> Text,
        block_height -> BigInt,
        decision -> Integer,
        signature -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    last_voted (id) {
        id -> Integer,
        block_id -> Text,
        height -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    leaf_blocks (id) {
        id -> Integer,
        block_id -> Text,
        block_height -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    locked_block (id) {
        id -> Integer,
        block_id -> Text,
        height -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    locked_outputs (id) {
        id -> Integer,
        block_id -> Text,
        transaction_id -> Text,
        shard_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    missing_transactions (id) {
        id -> Integer,
        block_id -> Text,
        block_height -> BigInt,
        transaction_id -> Text,
        is_awaiting_execution -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    parked_blocks (id) {
        id -> Integer,
        block_id -> Text,
        parent_block_id -> Text,
        height -> BigInt,
        epoch -> BigInt,
        proposed_by -> Text,
        justify -> Text,
        command_count -> BigInt,
        commands -> Text,
        total_leader_fee -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    quorum_certificates (id) {
        id -> Integer,
        qc_id -> Text,
        block_id -> Text,
        json -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    substates (id) {
        id -> Integer,
        shard_id -> Text,
        address -> Text,
        version -> Integer,
        data -> Text,
        state_hash -> Text,
        created_by_transaction -> Text,
        created_justify -> Text,
        created_block -> Text,
        created_height -> BigInt,
        destroyed_by_transaction -> Nullable<Text>,
        destroyed_justify -> Nullable<Text>,
        destroyed_by_block -> Nullable<Text>,
        created_at_epoch -> BigInt,
        destroyed_at_epoch -> Nullable<BigInt>,
        read_locks -> Integer,
        is_locked_w -> Bool,
        locked_by -> Nullable<Text>,
        created_at -> Timestamp,
        destroyed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    transaction_pool (id) {
        id -> Integer,
        transaction_id -> Text,
        original_decision -> Text,
        local_decision -> Nullable<Text>,
        remote_decision -> Nullable<Text>,
        evidence -> Text,
        remote_evidence -> Nullable<Text>,
        transaction_fee -> BigInt,
        leader_fee -> BigInt,
        stage -> Text,
        pending_stage -> Nullable<Text>,
        is_ready -> Bool,
        updated_at -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    transaction_pool_history (history_id) {
        history_id -> Nullable<Integer>,
        id -> Integer,
        transaction_id -> Text,
        original_decision -> Text,
        local_decision -> Nullable<Text>,
        remote_decision -> Nullable<Text>,
        evidence -> Text,
        transaction_fee -> BigInt,
        leader_fee -> BigInt,
        stage -> Text,
        new_stage -> Text,
        is_ready -> Bool,
        new_is_ready -> Bool,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        change_time -> Nullable<Timestamp>,
    }
}

diesel::table! {
    transaction_pool_state_updates (id) {
        id -> Integer,
        block_id -> Text,
        block_height -> BigInt,
        transaction_id -> Text,
        stage -> Text,
        evidence -> Text,
        is_ready -> Bool,
        local_decision -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    transactions (id) {
        id -> Integer,
        transaction_id -> Text,
        fee_instructions -> Text,
        instructions -> Text,
        signature -> Text,
        inputs -> Text,
        input_refs -> Text,
        filled_inputs -> Text,
        resulting_outputs -> Nullable<Text>,
        result -> Nullable<Text>,
        execution_time_ms -> Nullable<BigInt>,
        final_decision -> Nullable<Text>,
        abort_details -> Nullable<Text>,
        min_epoch -> Nullable<BigInt>,
        max_epoch -> Nullable<BigInt>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    votes (id) {
        id -> Integer,
        hash -> Text,
        epoch -> BigInt,
        block_id -> Text,
        decision -> Integer,
        sender_leaf_hash -> Text,
        signature -> Text,
        created_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    blocks,
    high_qcs,
    last_executed,
    last_proposed,
    last_sent_vote,
    last_voted,
    leaf_blocks,
    locked_block,
    locked_outputs,
    missing_transactions,
    parked_blocks,
    quorum_certificates,
    substates,
    transaction_pool,
    transaction_pool_history,
    transaction_pool_state_updates,
    transactions,
    votes,
);
