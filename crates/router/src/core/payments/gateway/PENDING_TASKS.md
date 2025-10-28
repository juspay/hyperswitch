# Gateway Implementation - Pending Tasks

**Last Updated**: 2025-10-25  
**Overall Status**: 85% Complete (BLOCKED - See Critical Blocker)  
**Estimated Remaining Work**: 14-19 hours (PENDING UCS team response)

---

## 📊 Current Status Summary

### ✅ Completed (85%)
- Gateway abstraction infrastructure
- Authorize main flow with GRPC execution
- PSync flow with GRPC execution  
- SetupMandate flow with GRPC execution (v1 and v2)
- Main authorize flow integrated with gateway context
- Backward compatibility maintained

### ❌ Pending (15%) - ⛔ BLOCKED
- 4 authorize sub-flows implementation (BLOCKED - GRPC endpoints missing)
- Sub-flow integration into authorize_flow.rs (BLOCKED - depends on sub-flows)
- Unit and integration tests (BLOCKED - depends on sub-flows)
- Shadow mode implementation (optional)

### 🔴 CRITICAL BLOCKER DISCOVERED
**Issue**: UCS GRPC client does not have endpoints for the 4 sub-flows
**Impact**: Cannot implement AuthorizeSessionToken, PreProcessing, PostProcessing, CreateOrder
**Action Required**: Contact UCS team immediately (see Blockers & Risks section)

---

## 🔴 CRITICAL - Phase 1: Implement Authorize Sub-flows (6-8 hours)

### Task 1.1: Implement AuthorizeSessionToken Flow (2 hours)
**File**: `crates/router/src/core/payments/gateway/authorize.rs`  
**Lines**: 378 (PaymentGateway), 503 (FlowGateway)

**Steps**:
1. Research GRPC endpoint for session tokens in UCS API docs
2. Implement context extraction (follow PSync pattern at psync.rs:82-86)
3. Replace `todo!()` at line 378 with GRPC execution
4. Replace `todo!()` at line 503 with flow gateway logic
5. Add GRPC execution function (similar to execute_payment_authorize)
6. Add error handling and logging
7. Test with mock GRPC client

**Pattern to Follow**:
```rust
// Context extraction (like psync.rs:82-86)
let merchant_context = context.merchant_context;
let header_payload = context.header_payload;
let lineage_ids = context.lineage_ids;
let merchant_connector_account = context.merchant_connector_account;

// GRPC execution (like psync.rs:145-220)
execute_authorize_session_token(
    state,
    merchant_connector_account,
    header_payload,
    lineage_ids,
    merchant_context,
    payment_data,
).await
```

---

### Task 1.2: Implement PreProcessing Flow (2 hours)
**File**: `crates/router/src/core/payments/gateway/authorize.rs`  
**Lines**: 428 (PaymentGateway), 535 (FlowGateway)

**Steps**:
1. Research GRPC endpoint for preprocessing in UCS API docs
2. Implement context extraction
3. Replace `todo!()` at line 428 with GRPC execution
4. Replace `todo!()` at line 535 with flow gateway logic
5. Add GRPC execution function
6. Add error handling and logging
7. Test with mock GRPC client

---

### Task 1.3: Implement PostProcessing Flow (2 hours)
**File**: `crates/router/src/core/payments/gateway/authorize.rs`  
**Lines**: 459 (PaymentGateway), 564 (FlowGateway)

**Steps**:
1. Research GRPC endpoint for postprocessing in UCS API docs
2. Implement context extraction
3. Replace `todo!()` at line 459 with GRPC execution
4. Replace `todo!()` at line 564 with flow gateway logic
5. Add GRPC execution function
6. Add error handling and logging
7. Test with mock GRPC client

---

### Task 1.4: Implement CreateOrder Flow (2 hours)
**File**: `crates/router/src/core/payments/gateway/authorize.rs`  
**Lines**: 587 (PaymentGateway), 627 (FlowGateway)

**Steps**:
1. Research GRPC endpoint for order creation in UCS API docs
2. Implement context extraction
3. Replace `todo!()` at line 587 with GRPC execution
4. Replace `todo!()` at line 627 with flow gateway logic
5. Add GRPC execution function
6. Add error handling and logging
7. Test with mock GRPC client

---

## 🟡 MEDIUM - Phase 2: Integrate Sub-flows (2-3 hours)

### Task 2.1: Integrate add_session_token Sub-flow (30 min)
**File**: `crates/router/src/core/payments/flows/authorize_flow.rs`  
**Line**: 304

**Steps**:
1. Identify where to get context fields (merchant_context, header_payload, etc.)
2. Create `RouterGatewayContext` for AuthorizeSessionToken
3. Replace `None::<RouterGatewayContext>` with `Some(context)` at line 304
4. Test integration

---

### Task 2.2: Integrate create_order_at_connector Sub-flow (30 min)
**File**: `crates/router/src/core/payments/flows/authorize_flow.rs`  
**Line**: 437

**Steps**:
1. Create `RouterGatewayContext` for CreateOrder
2. Replace `None::<RouterGatewayContext>` with `Some(context)` at line 437
3. Test integration

---

### Task 2.3: Integrate preprocessing_steps Sub-flow (30 min)
**File**: `crates/router/src/core/payments/flows/authorize_flow.rs`  
**Line**: 569

**Steps**:
1. Create `RouterGatewayContext` for PreProcessing
2. Replace `None::<RouterGatewayContext>` with `Some(context)` at line 569
3. Test integration

---

### Task 2.4: Integrate postprocessing_steps Sub-flow (30 min)
**File**: `crates/router/src/core/payments/flows/authorize_flow.rs`  
**Line**: 621

**Steps**:
1. Create `RouterGatewayContext` for PostProcessing
2. Replace `None::<RouterGatewayContext>` with `Some(context)` at line 621
3. Test integration

---

## 🟡 MEDIUM - Phase 3: Testing & Validation (4-6 hours)

### Task 3.1: Add Unit Tests for All Flows (3 hours)
**Files**: All gateway implementation files

**Required Tests**:
1. **PSync gateway tests** (psync.rs)
   - Test GRPC request building
   - Test response parsing
   - Test error handling
   - Test timeout scenarios

2. **SetupMandate gateway tests** (setup_mandate.rs)
   - Test GRPC request building (v1 and v2)
   - Test response parsing
   - Test error handling

3. **Authorize main flow tests** (authorize.rs)
   - Test execute_payment_authorize
   - Test execute_payment_repeat
   - Test error handling

4. **All 4 sub-flow tests** (authorize.rs)
   - AuthorizeSessionToken tests
   - PreProcessing tests
   - PostProcessing tests
   - CreateOrder tests

**Test Template**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_flow_gateway_execution() {
        // Mock GRPC client
        // Create test context
        // Execute gateway
        // Assert response
    }
    
    #[tokio::test]
    async fn test_flow_error_handling() {
        // Test GRPC errors
        // Test timeout
        // Test invalid response
    }
}
```

---

### Task 3.2: Add Integration Tests (2 hours)
**File**: `crates/router/tests/gateway_integration_tests.rs` (NEW)

**Required Tests**:
1. Test gateway routing logic (Direct vs UCS)
2. Test backward compatibility (None context)
3. Test all flows end-to-end
4. Test error scenarios
5. Mock GRPC service for testing

**Test Template**:
```rust
#[tokio::test]
async fn test_authorize_flow_with_gateway() {
    // Create real payment data
    // Create RouterGatewayContext
    // Execute flow
    // Verify UCS was called
    // Verify response is correct
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Pass None for context
    // Verify Direct path is used
    // Verify flow still works
}
```

---

### Task 3.3: End-to-End Testing (1 hour)
**Environment**: Staging UCS service

**Test Scenarios**:
1. Test with staging UCS service
2. Verify all flows work correctly
3. Performance testing (compare Direct vs UCS)
4. Load testing
5. Error scenario testing

---

## 🟢 LOW - Phase 4: Shadow Mode (6-8 hours) - OPTIONAL

### Task 4.1: Implement Shadow Mode Execution (2 hours)
**File**: `crates/hyperswitch_interfaces/src/api/gateway.rs`  
**Line**: 193

**Steps**:
1. Replace `todo!()` at line 193
2. Execute both Direct and UCS paths in parallel
3. Compare results
4. Log differences
5. Return Direct path result (UCS is shadow only)

---

### Task 4.2: Add Shadow Mode Metrics (2 hours)
**File**: `crates/router/src/core/payments/gateway/helpers.rs`

**Metrics to Track**:
- Success/failure rates (Direct vs UCS)
- Response time differences
- Response data differences
- Error rate comparison

---

### Task 4.3: Add Shadow Mode Configuration (1 hour)
**File**: `crates/router/src/configs/`

**Configuration**:
- Feature flag for shadow mode
- Sampling rate (e.g., 1%, 10%, 100%)
- Connector-specific shadow mode settings
- Timeout configuration

---

### Task 4.4: Add Shadow Mode Alerting (3 hours)
**File**: `crates/router/src/core/payments/gateway/`

**Alerts**:
- Alert on high difference rates (>5%)
- Alert on UCS failures (>1%)
- Dashboard for shadow mode metrics
- Slack/email notifications

---

## 🎯 Recommended Execution Order

### Week 1: Sub-flow Implementation (6-8 hours)
**Priority**: 🔴 CRITICAL

1. Research GRPC endpoints for all 4 sub-flows (1 hour)
2. Implement AuthorizeSessionToken (2 hours)
3. Implement PreProcessing (2 hours)
4. Implement PostProcessing (2 hours)
5. Implement CreateOrder (2 hours)

**Deliverable**: All 4 sub-flows compile without `todo!()`

---

### Week 2: Integration & Testing (6-9 hours)
**Priority**: 🟡 MEDIUM

1. Integrate all 4 sub-flows into authorize_flow.rs (2-3 hours)
2. Add unit tests for all flows (3 hours)
3. Add integration tests (2 hours)
4. End-to-end testing (1 hour)

**Deliverable**: All flows integrated and tested

---

### Week 3: Shadow Mode (Optional, 6-8 hours)
**Priority**: 🟢 LOW

1. Implement shadow mode execution (2 hours)
2. Add metrics (2 hours)
3. Add configuration (1 hour)
4. Add alerting (3 hours)

**Deliverable**: Shadow mode ready for gradual rollout

---

## 📋 Checklist

### Phase 1: Sub-flows
- [ ] Research GRPC endpoints
- [ ] Implement AuthorizeSessionToken
- [ ] Implement PreProcessing
- [ ] Implement PostProcessing
- [ ] Implement CreateOrder
- [ ] All sub-flows compile without errors

### Phase 2: Integration
- [ ] Integrate add_session_token
- [ ] Integrate create_order_at_connector
- [ ] Integrate preprocessing_steps
- [ ] Integrate postprocessing_steps
- [ ] No sub-flows pass `None` for context

### Phase 3: Testing
- [ ] Unit tests for PSync
- [ ] Unit tests for SetupMandate
- [ ] Unit tests for Authorize main flow
- [ ] Unit tests for all 4 sub-flows
- [ ] Integration tests
- [ ] End-to-end tests
- [ ] All tests pass

### Phase 4: Shadow Mode (Optional)
- [ ] Shadow mode execution implemented
- [ ] Metrics tracking added
- [ ] Configuration added
- [ ] Alerting added
- [ ] Shadow mode tested at 1% traffic

---

## 🚨 Blockers & Risks

### 🔴 CRITICAL BLOCKER: GRPC Endpoints Do Not Exist

**Status**: ⛔ BLOCKED - Cannot proceed with sub-flow implementation

**Critical Finding**: The UCS GRPC client only provides **5 methods**, and **NONE** match the 4 required sub-flows.

#### Available UCS GRPC Methods (from unified_connector_service.rs)
1. ✅ `payment_authorize` - Used for main authorize flow
2. ✅ `payment_get` - Used for payment sync (PSync)
3. ✅ `payment_setup_mandate` - Used for setup mandate
4. ✅ `payment_repeat` - Used for MIT payments
5. ✅ `transform_incoming_webhook` - Used for webhook handling

#### Missing GRPC Methods for Sub-flows
1. ❌ **AuthorizeSessionToken** - No `payment_session_token` or similar method
2. ❌ **PreProcessing** - No `payment_preprocess` or similar method
3. ❌ **PostProcessing** - No `payment_postprocess` or similar method
4. ❌ **CreateOrder** - No `payment_create_order` or similar method

#### Evidence
- **File**: `crates/external_services/src/grpc_client/unified_connector_service.rs`
- **UCS Dependency**: `unified-connector-service-client` from https://github.com/juspay/connector-service
- **Revision**: `f719688943adf7bc17bb93dcb43f27485c17a96e`
- **Package**: `rust-grpc-client`

### 🚨 Required Actions (URGENT)

#### Action 1: Contact UCS Team (Priority: 🔴 CRITICAL)
**Questions to Ask**:
1. Do GRPC methods exist for session_token, preprocess, postprocess, create_order?
2. If yes, what are the exact method names and proto definitions?
3. If no, should these flows use the existing `payment_authorize` method?
4. If not supported, what is the timeline for UCS to add these endpoints?
5. Are these flows even supported in UCS, or should they remain Direct-only?

**Owner**: TBD  
**ETA**: 1-2 days  
**Blocking**: All Phase 1 tasks

#### Action 2: Check External Repository (Priority: 🔴 CRITICAL)
**Repository**: https://github.com/juspay/connector-service  
**Revision**: f719688943adf7bc17bb93dcb43f27485c17a96e  
**Action**: Examine proto service definitions for any undocumented methods  
**Owner**: TBD  
**ETA**: 1 hour

#### Action 3: Determine Implementation Strategy (Priority: 🔴 CRITICAL)
**Decision Required**: Choose one of the following options:

**Option A: Use Existing `payment_authorize` Method**
- Pros: Can implement immediately, no UCS changes needed
- Cons: May not be semantically correct, might confuse UCS service
- Implementation: Use `payment_authorize` with different request data for each sub-flow

**Option B: Wait for UCS Team to Add New Endpoints**
- Pros: Semantically correct, proper separation of concerns
- Cons: Blocks implementation, unknown timeline
- Implementation: Keep `todo!()` placeholders until UCS adds support

**Option C: Keep Direct Connector Path (Bypass UCS)**
- Pros: Unblocks implementation, maintains current functionality
- Cons: Sub-flows won't benefit from UCS features
- Implementation: Remove gateway integration for these 4 flows

**Decision Maker**: Product/Engineering team  
**ETA**: After UCS team response

### Current Blockers
1. ⛔ **GRPC Endpoints Missing** - Cannot implement sub-flows without UCS support
2. ⏳ **Waiting on UCS Team** - Need confirmation on endpoint availability
3. ⏳ **Strategy Decision Pending** - Need to choose Option A, B, or C

### Risks
1. **GRPC Endpoints Do Not Exist** (🔴 CRITICAL - CONFIRMED)
   - Sub-flow GRPC endpoints **DO NOT EXIST** in current UCS client
   - **Impact**: Cannot implement 4 sub-flows until resolved
   - **Mitigation**: Contact UCS team immediately
   - **Fallback Options**: See Action 3 above

2. **GRPC Service Compatibility** (MEDIUM)
   - New GRPC calls may not work with current UCS service
   - **Mitigation**: Test with staging UCS early
   - **Fallback**: Update request/response types

3. **Performance Impact** (LOW)
   - Gateway abstraction adds overhead
   - **Mitigation**: Benchmark Direct vs UCS paths
   - **Fallback**: Optimize hot paths

---

## 📞 URGENT Questions for Product/Engineering

### 🔴 Critical Questions (Blocking Implementation)
1. **UCS Support**: Are these 4 sub-flows even supported in UCS?
   - AuthorizeSessionToken
   - PreProcessing
   - PostProcessing
   - CreateOrder

2. **Implementation Strategy**: Which option should we choose?
   - Option A: Use existing `payment_authorize` method
   - Option B: Wait for UCS to add new endpoints
   - Option C: Keep Direct connector path (bypass UCS)

3. **Priority**: If not all supported, which sub-flows are most critical?

### 🟡 Important Questions
4. **Timeline**: What's the target date for complete UCS migration?
5. **Testing**: Do we have a staging UCS environment for testing?
6. **Shadow Mode**: Is shadow mode required before production rollout?
7. **Monitoring**: What metrics should we track for UCS vs Direct comparison?

### 📋 Additional Context Needed
8. **UCS Team Contact**: Who should we contact about missing GRPC endpoints?
9. **Proto Repository**: Do we have access to the full proto definitions?
10. **Fallback Plan**: If UCS doesn't support these flows, is Direct path acceptable?

---

## 📈 Success Criteria

### Minimum Viable Product (MVP)
- ✅ All 4 sub-flows implemented
- ✅ All sub-flows integrated
- ✅ Basic unit tests pass
- ✅ Integration tests pass
- ✅ No `todo!()` macros in production code

### Production Ready
- ✅ MVP criteria met
- ✅ 100% unit test coverage
- ✅ End-to-end tests pass
- ✅ Performance within 5% of Direct path
- ✅ Shadow mode tested at 10% traffic (optional)
- ✅ Documentation complete

---

## 📝 Notes

- **Pattern Established**: PSync and SetupMandate provide proven implementation pattern ✅
- **Architecture Solid**: Gateway abstraction works correctly for existing flows ✅
- **BLOCKER IDENTIFIED**: UCS GRPC client missing 4 sub-flow endpoints ⛔
- **Decision Required**: Choose implementation strategy (see Blockers section)
- **Testing Critical**: Don't skip testing - it's 30% of remaining work but prevents production issues

**Next Immediate Action**: 
1. ⛔ Contact UCS team about missing GRPC endpoints (URGENT)
2. ⏳ Check external proto repository for undocumented methods
3. ⏳ Decide on implementation strategy (Option A, B, or C)
4. ⏸️ Implementation BLOCKED until strategy decided

## 🎯 Revised Timeline

### Scenario A: UCS Adds Endpoints (Best Case)
- **Week 1**: Wait for UCS team response (1-2 days)
- **Week 2**: UCS implements new endpoints (1-2 weeks)
- **Week 3**: Implement sub-flows (6-8 hours)
- **Week 4**: Integration & testing (6-9 hours)
- **Total**: 3-4 weeks

### Scenario B: Use Existing Authorize Method (Medium Case)
- **Week 1**: Get UCS team approval (1-2 days)
- **Week 2**: Implement using `payment_authorize` (6-8 hours)
- **Week 3**: Integration & testing (6-9 hours)
- **Total**: 2-3 weeks

### Scenario C: Keep Direct Path (Fallback)
- **Week 1**: Get product approval (1-2 days)
- **Week 2**: Remove gateway integration for sub-flows (2-3 hours)
- **Week 3**: Update documentation (1-2 hours)
- **Total**: 1-2 weeks