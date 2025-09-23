// @generated automatically by Diesel CLI.

diesel::table! {
    api_logs (id) {
        id -> Int8,
        #[max_length = 255]
        endpoint -> Varchar,
        #[max_length = 10]
        method -> Varchar,
        #[max_length = 100]
        user_address -> Nullable<Varchar>,
        #[max_length = 50]
        vault_id -> Nullable<Varchar>,
        response_time_ms -> Nullable<Int4>,
        status_code -> Nullable<Int4>,
        error_message -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    indexer_state (id) {
        id -> Int4,
        #[max_length = 50]
        vault_id -> Varchar,
        last_processed_block -> Int8,
        last_processed_timestamp -> Nullable<Timestamptz>,
        last_error -> Nullable<Text>,
        last_error_at -> Nullable<Timestamptz>,
        #[max_length = 20]
        status -> Nullable<Varchar>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    user_kpis (id) {
        id -> Int4,
        #[max_length = 100]
        user_address -> Varchar,
        #[max_length = 50]
        vault_id -> Varchar,
        all_time_pnl -> Nullable<Numeric>,
        unrealized_pnl -> Nullable<Numeric>,
        realized_pnl -> Nullable<Numeric>,
        max_drawdown_pct -> Nullable<Numeric>,
        sharpe_ratio -> Nullable<Numeric>,
        total_deposits -> Nullable<Numeric>,
        total_withdrawals -> Nullable<Numeric>,
        total_fees_paid -> Nullable<Numeric>,
        calculated_at -> Nullable<Timestamptz>,
        share_price_used -> Nullable<Numeric>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        sortino_ratio -> Nullable<Numeric>,
        share_balance -> Nullable<Numeric>,
    }
}

diesel::table! {
    user_positions (id) {
        id -> Int4,
        #[max_length = 100]
        user_address -> Varchar,
        #[max_length = 50]
        vault_id -> Varchar,
        share_balance -> Numeric,
        cost_basis -> Numeric,
        first_deposit_at -> Nullable<Timestamptz>,
        last_activity_at -> Nullable<Timestamptz>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    user_transactions (id) {
        id -> Int4,
        #[max_length = 100]
        tx_hash -> Varchar,
        block_number -> Int8,
        block_timestamp -> Timestamptz,
        #[max_length = 100]
        user_address -> Varchar,
        #[max_length = 50]
        vault_id -> Varchar,
        #[sql_name = "type"]
        #[max_length = 20]
        type_ -> Varchar,
        #[max_length = 20]
        status -> Varchar,
        amount -> Numeric,
        shares_amount -> Nullable<Numeric>,
        share_price -> Nullable<Numeric>,
        gas_fee -> Nullable<Numeric>,
        metadata -> Nullable<Jsonb>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (address) {
        #[max_length = 100]
        address -> Varchar,
        #[max_length = 50]
        chain -> Varchar,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    vaults (id) {
        #[max_length = 50]
        id -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        #[max_length = 50]
        chain -> Varchar,
        #[max_length = 50]
        chain_id -> Nullable<Varchar>,
        #[max_length = 20]
        symbol -> Varchar,
        #[max_length = 20]
        base_asset -> Varchar,
        #[max_length = 20]
        status -> Varchar,
        inception_date -> Nullable<Date>,
        #[max_length = 100]
        contract_address -> Varchar,
        mgmt_fee_bps -> Nullable<Int4>,
        perf_fee_bps -> Int4,
        strategy_brief -> Nullable<Text>,
        #[max_length = 500]
        docs_url -> Nullable<Varchar>,
        min_deposit -> Nullable<Numeric>,
        max_deposit -> Nullable<Numeric>,
        deposit_paused -> Nullable<Bool>,
        instant_liquidity -> Nullable<Bool>,
        instant_slippage_max_bps -> Nullable<Int4>,
        redeem_24h_threshold_pct_of_aum -> Nullable<Numeric>,
        redeem_48h_above_threshold -> Nullable<Bool>,
        #[max_length = 500]
        icon_light_url -> Nullable<Varchar>,
        #[max_length = 500]
        icon_dark_url -> Nullable<Varchar>,
        #[max_length = 500]
        api_endpoint -> Varchar,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        start_block -> Int8,
    }
}

diesel::joinable!(indexer_state -> vaults (vault_id));
diesel::joinable!(user_kpis -> users (user_address));
diesel::joinable!(user_kpis -> vaults (vault_id));
diesel::joinable!(user_positions -> users (user_address));
diesel::joinable!(user_positions -> vaults (vault_id));
diesel::joinable!(user_transactions -> users (user_address));
diesel::joinable!(user_transactions -> vaults (vault_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_logs,
    indexer_state,
    user_kpis,
    user_positions,
    user_transactions,
    users,
    vaults,
);
