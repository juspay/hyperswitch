use api_models::analytics::search::SearchIndex;

pub const fn get_search_indexes() -> [SearchIndex; 8] {
    [
        SearchIndex::PaymentAttempts,
        SearchIndex::PaymentIntents,
        SearchIndex::Refunds,
        SearchIndex::Disputes,
        SearchIndex::SessionizerPaymentAttempts,
        SearchIndex::SessionizerPaymentIntents,
        SearchIndex::SessionizerRefunds,
        SearchIndex::SessionizerDisputes,
    ]
}

pub const SEARCH_INDEXES: [SearchIndex; 8] = get_search_indexes();
