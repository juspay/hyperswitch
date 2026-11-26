use common_utils::events::ApiEventMetric;

use crate::offer_engine::{BrowseOffersRequest, BrowseOffersResponse};

impl ApiEventMetric for BrowseOffersRequest {}

impl ApiEventMetric for BrowseOffersResponse {}

impl ApiEventMetric for crate::offer_engine::OfferEngineDashboardProxyRequest {}
