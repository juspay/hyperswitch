use api_models::analytics::search::SearchIndex;

pub const fn get_search_indexes() -> [SearchIndex; 4] {
    [
        SearchIndex::PaymentAttempts,
        SearchIndex::PaymentIntents,
        SearchIndex::Refunds,
        SearchIndex::Disputes,
    ]
}

pub const SEARCH_INDEXES: [SearchIndex; 4] = get_search_indexes();
