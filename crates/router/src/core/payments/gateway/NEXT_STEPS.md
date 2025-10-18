# Gateway Abstraction Layer - Next Steps

## 🎯 Current Status

### ✅ What's Working
- **DirectGateway**: Fully functional, wraps `execute_connector_processing_step`
- **Trait Architecture**: Properly located in `hyperswitch_interfaces` crate
- **Type Safety**: Generic over State, ConnectorData, MerchantConnectorAccount
- **Factory Pattern**: Creates appropriate gateway based on execution path
- **Documentation**: Comprehensive docs explaining architecture and limitations

### ⚠️ What's Incomplete
- **UCS Gateway**: Marked as `todo!()` - needs additional context
- **Cutover Decision**: Factory always returns Direct path
- **Shadow Mode**: Not implemented yet

---

## 📋 Recommended Implementation Plan

### Phase 1: Use Gateway for Direct Path Only (CURRENT)

**Goal**: Get value from gateway abstraction for Direct path while UCS support is being built

**Implementation**:
```rust
// In payment flows (e.g., authorize_flow.rs)
pub async fn call_connector_service(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: &RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    payment_data: &PaymentData<api::Authorize>,
    merchant_connector_account: &MerchantConnectorAccountType,
    call_connector_action: CallConnectorAction,
) -> RouterResult<RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>> {
    // Check if we should use UCS
    let execution_path = should_call_unified_connector_service(
        state,
        merchant_context,
        router_data,
        Some(payment_data),
    ).await?;

    match execution_path {
        ExecutionPath::UnifiedConnectorService | ExecutionPath::ShadowUnifiedConnectorService => {
            // Use existing UCS code path
            call_unified_connector_service_authorize(
                router_data,
                state,
                header_payload,
                lineage_ids,
                merchant_connector_account,
                merchant_context,
                execution_mode,
            ).await
        }
        ExecutionPath::Direct => {
            // Use new gateway abstraction
            let gateway = GatewayFactory::create_authorize_gateway(
                state,
                connector,
                router_data,
                Some(payment_data),
            ).await?;

            gateway.execute(
                state,
                router_data.clone(),
                connector,
                merchant_connector_account,
                call_connector_action,
            ).await
        }
    }
}
```

**Benefits**:
- ✅ Immediate value for Direct path
- ✅ No breaking changes
- ✅ Incremental improvement
- ✅ UCS continues to work as before

**Timeline**: Can be implemented immediately

---

### Phase 2: Extend Trait for UCS Support

**Goal**: Enable UCS gateway implementation by providing necessary context

#### Step 1: Add Context Object to Interfaces Crate

**File**: `crates/hyperswitch_interfaces/src/api/gateway.rs`

```rust
/// Execution context for gateway operations
///
/// Provides additional context needed for UCS and other advanced gateway features.
/// All fields are optional to maintain backward compatibility.
pub struct GatewayExecutionContext<'a, F, PaymentData> {
    /// Merchant context for decision making
    pub merchant_context: Option<&'a MerchantContext>,
    
    /// Payment data for transformations
    pub payment_data: Option<&'a PaymentData>,
    
    /// Header payload for UCS headers
    pub header_payload: Option<&'a HeaderPayload>,
    
    /// Lineage IDs for tracing
    pub lineage_ids: Option<LineageIds>,
    
    /// Execution mode (Primary vs Shadow)
    pub execution_mode: ExecutionMode,
    
    /// Flow type marker
    _flow: PhantomData<F>,
}

impl<'a, F, PaymentData> GatewayExecutionContext<'a, F, PaymentData> {
    /// Create a minimal context for Direct path
    pub fn direct() -> Self {
        Self {
            merchant_context: None,
            payment_data: None,
            header_payload: None,
            lineage_ids: None,
            execution_mode: ExecutionMode::Primary,
            _flow: PhantomData,
        }
    }
    
    /// Create a full context for UCS path
    pub fn ucs(
        merchant_context: &'a MerchantContext,
        payment_data: &'a PaymentData,
        header_payload: &'a HeaderPayload,
        lineage_ids: LineageIds,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            merchant_context: Some(merchant_context),
            payment_data: Some(payment_data),
            header_payload: Some(header_payload),
            lineage_ids: Some(lineage_ids),
            execution_mode,
            _flow: PhantomData,
        }
    }
}
```

#### Step 2: Update PaymentGateway Trait

**File**: `crates/hyperswitch_interfaces/src/api/gateway.rs`

```rust
#[async_trait]
pub trait PaymentGateway<State, ConnectorData, MerchantConnectorAccount, F, Req, Resp, PaymentData>:
    Send + Sync
{
    async fn execute(
        self,
        state: &State,
        router_data: RouterData<F, Req, Resp>,
        connector: &ConnectorData,
        merchant_connector_account: &MerchantConnectorAccount,
        call_connector_action: CallConnectorAction,
        context: GatewayExecutionContext<'_, F, PaymentData>, // Add context
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>;
}
```

#### Step 3: Update DirectGateway

**File**: `crates/router/src/core/payments/gateway/direct.rs`

```rust
#[async_trait]
impl<F, ResourceCommonData, Req, Resp>
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        F,
        Req,
        Resp,
        PaymentData<F>, // Add PaymentData type
    > for DirectGateway<F, ResourceCommonData, Req, Resp>
where
    F: Clone + Send + Sync + std::fmt::Debug + 'static,
    ResourceCommonData: Clone + Send + Sync + 'static + connector_integration_interface::RouterDataConversion<F, Req, Resp>,
    Req: Clone + Send + Sync + std::fmt::Debug + 'static,
    Resp: Clone + Send + Sync + std::fmt::Debug + 'static,
{
    async fn execute(
        self,
        state: &SessionState,
        router_data: RouterData<F, Req, Resp>,
        _connector: &api::ConnectorData,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _call_connector_action: CallConnectorAction,
        _context: gateway_interface::GatewayExecutionContext<'_, F, PaymentData<F>>, // Accept context but don't use it
    ) -> CustomResult<RouterData<F, Req, Resp>, hyperswitch_interfaces::errors::ConnectorError> {
        // DirectGateway doesn't need context - just delegate to execute_connector_processing_step
        services::execute_connector_processing_step(
            state,
            self.connector_integration,
            &router_data,
            CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
    }
}
```

#### Step 4: Implement UCS Gateway

**File**: `crates/router/src/core/payments/gateway/ucs.rs`

```rust
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
        PaymentData<api::Authorize>, // Add PaymentData type
    > for UnifiedConnectorServiceGateway<api::Authorize>
{
    async fn execute(
        self,
        state: &SessionState,
        mut router_data: RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        _call_connector_action: CallConnectorAction,
        context: gateway_interface::GatewayExecutionContext<'_, api::Authorize, PaymentData<api::Authorize>>,
    ) -> CustomResult<
        RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    > {
        // Get UCS client
        let client = state
            .grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("UCS client not available")?;

        // Check if this is MIT (recurring) or CIT (first payment)
        let is_mandate_payment = router_data.request.mandate_id.is_some();

        if is_mandate_payment {
            // MIT flow - use payment_repeat
            let grpc_request = ucs::transformers::PaymentServiceRepeatEverythingRequest::foreign_try_from(&router_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let auth_metadata = ucs::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &connector.connector_name,
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let headers = state
                .get_grpc_headers_ucs(context.execution_mode)
                .lineage_ids(context.lineage_ids.unwrap_or_default());

            let response = client
                .payment_repeat(grpc_request, auth_metadata, headers)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let (payments_response, attempt_status, http_status_code) =
                ucs::handle_unified_connector_service_response_for_payment_repeat(response.into_inner())
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

            router_data.response = payments_response;
            router_data.status = attempt_status;
            router_data.connector_http_status_code = Some(http_status_code);
        } else {
            // CIT flow - use payment_authorize
            let grpc_request = ucs::transformers::PaymentServiceAuthorizeRequest::foreign_try_from(&router_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let auth_metadata = ucs::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &connector.connector_name,
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let headers = state
                .get_grpc_headers_ucs(context.execution_mode)
                .lineage_ids(context.lineage_ids.unwrap_or_default());

            let response = client
                .payment_authorize(grpc_request, auth_metadata, headers)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

            let (payments_response, attempt_status, http_status_code) =
                ucs::handle_unified_connector_service_response_for_payment_authorize(response.into_inner())
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;

            router_data.response = payments_response;
            router_data.status = attempt_status;
            router_data.connector_http_status_code = Some(http_status_code);
        }

        Ok(router_data)
    }
}
```

#### Step 5: Update Factory with Full Decision Logic

**File**: `crates/router/src/core/payments/gateway/factory.rs`

```rust
impl GatewayFactory {
    pub async fn create_authorize_gateway(
        state: &SessionState,
        connector: &api::ConnectorData,
        router_data: &RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        payment_data: Option<&PaymentData<api::Authorize>>,
        merchant_context: &MerchantContext, // Add this parameter
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                SessionState,
                api::ConnectorData,
                MerchantConnectorAccountType,
                api::Authorize,
                PaymentsAuthorizeData,
                PaymentsResponseData,
                PaymentData<api::Authorize>,
            >,
        >,
    > {
        let merchant_connector_id = connector
            .merchant_connector_id
            .as_ref()
            .ok_or(crate::core::errors::ApiErrorResponse::InternalServerError)?;

        // Now we can call the real decision logic
        let execution_path = ucs::should_call_unified_connector_service(
            state,
            merchant_context,
            router_data,
            payment_data,
        )
        .await?;

        match execution_path {
            ExecutionPath::Direct => {
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
            ExecutionPath::UnifiedConnectorService => {
                Ok(Box::new(UnifiedConnectorServiceGateway::new()))
            }
            ExecutionPath::ShadowUnifiedConnectorService => {
                // TODO: Implement ShadowGateway
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
        }
    }
}
```

#### Step 6: Update Flow Usage

**File**: `crates/router/src/core/payments/flows/authorize_flow.rs`

```rust
pub async fn call_connector_service(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: &RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    payment_data: &PaymentData<api::Authorize>,
    merchant_connector_account: &MerchantConnectorAccountType,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    call_connector_action: CallConnectorAction,
) -> RouterResult<RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>> {
    // Create gateway (decision logic inside factory)
    let gateway = GatewayFactory::create_authorize_gateway(
        state,
        connector,
        router_data,
        Some(payment_data),
        merchant_context, // Now we pass this
    ).await?;

    // Create context based on gateway type
    let context = gateway_interface::GatewayExecutionContext::ucs(
        merchant_context,
        payment_data,
        header_payload,
        lineage_ids,
        ExecutionMode::Primary,
    );

    // Execute through gateway
    gateway.execute(
        state,
        router_data.clone(),
        connector,
        merchant_connector_account,
        call_connector_action,
        context,
    ).await
}
```

**Timeline**: 2-3 weeks

---

### Phase 3: Implement Shadow Gateway

**Goal**: Enable A/B testing between Direct and UCS paths

#### Implementation

**File**: `crates/router/src/core/payments/gateway/shadow.rs`

```rust
pub struct ShadowGateway<State, ConnectorData, MCA, F, Req, Resp, PaymentData> {
    primary: Box<dyn PaymentGateway<State, ConnectorData, MCA, F, Req, Resp, PaymentData>>,
    shadow: Box<dyn PaymentGateway<State, ConnectorData, MCA, F, Req, Resp, PaymentData>>,
    _phantom: PhantomData<(State, ConnectorData, MCA, F, Req, Resp, PaymentData)>,
}

#[async_trait]
impl<State, ConnectorData, MCA, F, Req, Resp, PaymentData>
    PaymentGateway<State, ConnectorData, MCA, F, Req, Resp, PaymentData>
    for ShadowGateway<State, ConnectorData, MCA, F, Req, Resp, PaymentData>
where
    State: Send + Sync,
    ConnectorData: Clone + Send + Sync,
    MCA: Clone + Send + Sync,
    F: Clone + Send + Sync + 'static,
    Req: Clone + Send + Sync + 'static,
    Resp: Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    async fn execute(
        self,
        state: &State,
        router_data: RouterData<F, Req, Resp>,
        connector: &ConnectorData,
        merchant_connector_account: &MCA,
        call_connector_action: CallConnectorAction,
        context: GatewayExecutionContext<'_, F, PaymentData>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
        // Execute primary path
        let primary_result = self.primary.execute(
            state,
            router_data.clone(),
            connector,
            merchant_connector_account,
            call_connector_action.clone(),
            context.clone(),
        ).await;

        // Execute shadow path in background (don't await)
        let shadow_router_data = router_data.clone();
        let shadow_connector = connector.clone();
        let shadow_mca = merchant_connector_account.clone();
        let shadow_context = context.clone();
        
        tokio::spawn(async move {
            let shadow_result = self.shadow.execute(
                state,
                shadow_router_data,
                &shadow_connector,
                &shadow_mca,
                call_connector_action,
                shadow_context,
            ).await;

            // Compare results and log differences
            compare_and_log_results(primary_result.clone(), shadow_result);
        });

        // Return primary result immediately
        primary_result
    }
}
```

**Timeline**: 1-2 weeks after Phase 2

---

### Phase 4: Full Migration

**Goal**: Migrate all payment flows to use gateway abstraction

#### Flows to Migrate (20+ total)

1. ✅ authorize_flow.rs (POC)
2. psync_flow.rs
3. setup_mandate_flow.rs
4. capture_flow.rs
5. cancel_flow.rs
6. complete_authorize_flow.rs
7. incremental_authorization_flow.rs
8. extend_authorization_flow.rs
9. session_flow.rs
10. approve_flow.rs
11. reject_flow.rs
12. ... (10+ more)

#### Migration Strategy

For each flow:
1. Add gateway factory call
2. Create execution context
3. Call gateway.execute()
4. Test thoroughly
5. Monitor metrics
6. Move to next flow

**Timeline**: 4-6 weeks (1-2 flows per week)

---

## 🎯 Success Metrics

### Technical Metrics
- [ ] Code reduction: 50+ lines → 2 lines per flow
- [ ] Test coverage: >80% for gateway module
- [ ] Performance overhead: <5ms per request
- [ ] Success rate: Same as before migration
- [ ] Latency: Within 10% of baseline

### Business Metrics
- [ ] Zero production incidents during rollout
- [ ] 100% of flows migrated
- [ ] Developer satisfaction: >4/5
- [ ] Reduced time to add new flows: 50% reduction

---

## 📊 Timeline Summary

| Phase | Duration | Status |
|-------|----------|--------|
| Phase 1: Direct Path Only | Immediate | ✅ Can start now |
| Phase 2: UCS Support | 2-3 weeks | 🚧 Needs context object |
| Phase 3: Shadow Gateway | 1-2 weeks | 🚧 After Phase 2 |
| Phase 4: Full Migration | 4-6 weeks | 🚧 After Phase 3 |
| **Total** | **8-12 weeks** | |

---

## 🚀 Immediate Action Items

### This Week
1. [ ] Review and approve Phase 1 approach
2. [ ] Implement gateway usage in authorize_flow.rs (Direct path only)
3. [ ] Add unit tests for DirectGateway
4. [ ] Add integration tests for authorize flow with gateway

### Next Week
2. [ ] Design GatewayExecutionContext structure
3. [ ] Update PaymentGateway trait with context parameter
4. [ ] Update DirectGateway to accept (and ignore) context
5. [ ] Start UCS gateway implementation

### Week 3-4
6. [ ] Complete UCS gateway implementation
7. [ ] Update factory with full decision logic
8. [ ] Test UCS path with gateway abstraction
9. [ ] Validate metrics and monitoring

---

## 📝 Questions to Answer

1. **Context Object Design**: Should we use Option<Context> or always require it?
   - **Recommendation**: Always require it, use `GatewayExecutionContext::direct()` for Direct path

2. **MerchantContext in Factory**: How to get MerchantContext in factory?
   - **Recommendation**: Add as parameter to factory methods

3. **Shadow Mode Priority**: Should we implement shadow mode before full migration?
   - **Recommendation**: Yes, it's valuable for validation during migration

4. **Backward Compatibility**: How to maintain compatibility during migration?
   - **Recommendation**: Keep old code paths until all flows migrated, then remove

---

## 🎓 Resources

- [CHANGES_ANALYSIS.md](./CHANGES_ANALYSIS.md) - Detailed analysis of changes made
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Architecture deep dive
- [USAGE_EXAMPLE.md](./USAGE_EXAMPLE.md) - Usage examples
- [TODO.md](./TODO.md) - Detailed checklist

---

**Status**: Ready to proceed with Phase 1 ✅
**Next Action**: Implement gateway usage in authorize_flow.rs for Direct path
**Timeline**: 8-12 weeks for full implementation