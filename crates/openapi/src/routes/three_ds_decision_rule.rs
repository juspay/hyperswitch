/// 3DS Decision - Execute
#[utoipa::path(
    post,
    path = "/three_ds_decision/execute",
    request_body = ThreeDsDecisionRuleExecuteRequest,
    responses(
        (status = 200, description = "3DS Decision Rule Executed Successfully", body = ThreeDsDecisionRuleExecuteResponse),
        (status = 400, description = "Bad Request")
    ),
    tag = "3DS Decision Rule",
    operation_id = "Execute 3DS Decision Rule",
    security(("api_key" = []))
)]
pub fn three_ds_decision_rule_execute() {}
