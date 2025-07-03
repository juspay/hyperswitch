# Large Files Report

This report identifies Markdown files in the Memory Bank that exceed or approach size thresholds.
Generated on 2025-05-27 21:14:37

## Files Exceeding Thresholds

Files that exceed the primary threshold criteria (300 lines or 15000 bytes):

| File Path | Line Count | Size (KB) | Status |
|-----------|------------|-----------|--------|
| crateIndex.md |      693 | 16.84 | Exceeds threshold |
| thematic/crates/hyperswitch_connectors/connector_implementation_guide.md |      702 | 23.01 | Exceeds threshold |
| thematic/crates/hyperswitch_connectors/connector_testing_guide.md |      497 | 15.51 | Exceeds threshold |
| thematic/crates/hyperswitch_connectors/connector_configuration_guide.md |      391 | 12.17 | Exceeds threshold |
| thematic/crates/hyperswitch_interfaces/webhook_handling.md |      391 | 17.44 | Exceeds threshold |
| thematic/crates/hyperswitch_interfaces/connector_integration.md |      316 | 10.11 | Exceeds threshold |
| thematic/crates/router/configuration/router_configuration.md |      344 | 10.55 | Exceeds threshold |
| thematic/crates/router/configuration/routing_strategies.md |      306 | 10.29 | Exceeds threshold |
| thematic/crates/router/architecture/dependencies.md |      320 | 9.28 | Exceeds threshold |
| thematic/crates/router/flows/payment_flows.md |      617 | 25.13 | Exceeds threshold |
| thematic/crates/router/flows/refund_flows.md |      440 | 18.85 | Exceeds threshold |
| thematic/crates/router/flows/webhook_flows.md |      421 | 17.67 | Exceeds threshold |
| thematic/documentation_process/review_process/05_feedback_incorporation.md |      301 | 8.77 | Exceeds threshold |

## Files Approaching Thresholds

Files that are approaching the threshold (270-300 lines) and may require monitoring:

| File Path | Line Count | Size (KB) | Status |
|-----------|------------|-----------|--------|
| thematic/crates/common_utils/overview.md |      280 | 10.95 | Approaching threshold |
| thematic/crates/hyperswitch_connectors/connector_interface_guide.md |      280 | 10.77 | Approaching threshold |
| thematic/crates/hyperswitch_constraint_graph/overview.md |      279 | 9.22 | Approaching threshold |
| thematic/crates/euclid_wasm/overview.md |      285 | 8.90 | Approaching threshold |
| thematic/crates/redis_interface/overview.md |      280 | 9.38 | Approaching threshold |
| thematic/crates/external_services/overview.md |      289 | 8.73 | Approaching threshold |
| thematic/crates/masking/overview.md |      271 | 9.38 | Approaching threshold |
| thematic/crates/hyperswitch_interfaces/additional_components.md |      298 | 10.36 | Approaching threshold |
| thematic/documentation_process/review_process/03_review_checklists.md |      273 | 10.35 | Approaching threshold |
| thematic/documentation_process/templates/implementation_guide_template.md |      280 | 6.19 | Approaching threshold |

## Analysis Notes

Files exceeding thresholds should be evaluated for splitting according to the criteria in [File Identification Criteria](file_identification_criteria.md).

## Next Steps

1. Review each file exceeding thresholds for logical split points
2. Determine the appropriate splitting strategy for each file
3. Implement splits for files that meet the decision criteria
4. Update this report with the actions taken

## Reference

- [File Identification Criteria](file_identification_criteria.md)
- [File Size Management Guide](../file_size_management_guide.md)
