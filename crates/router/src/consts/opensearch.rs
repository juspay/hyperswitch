use api_models::analytics::search::SearchIndex;

pub const fn get_search_indexes() -> [SearchIndex; 9] {
    [
        SearchIndex::PaymentAttempts,
        SearchIndex::PaymentIntents,
        SearchIndex::Refunds,
        SearchIndex::Disputes,
        SearchIndex::Payouts,
        SearchIndex::SessionizerPaymentAttempts,
        SearchIndex::SessionizerPaymentIntents,
        SearchIndex::SessionizerRefunds,
        SearchIndex::SessionizerDisputes,
    ]
}

pub const SEARCH_INDEXES: [SearchIndex; 9] = get_search_indexes();
