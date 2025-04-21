# CaptureMethod

Default value if not passed is set to 'automatic' which results in Auth and Capture in one single API request. Pass 'manual' or 'manual_multiple' in case you want do a separate Auth and Capture by first authorizing and placing a hold on your customer's funds so that you can use the Payments/Capture endpoint later to capture the authorized amount. Pass 'manual' if you want to only capture the amount later once or 'manual_multiple' if you want to capture the funds multiple times later. Both 'manual' and 'manual_multiple' are only supported by a specific list of processors

## Enum

* `AUTOMATIC` (value: `'automatic'`)

* `MANUAL` (value: `'manual'`)

* `MANUAL_MULTIPLE` (value: `'manual_multiple'`)

* `SCHEDULED` (value: `'scheduled'`)

* `SEQUENTIAL_AUTOMATIC` (value: `'sequential_automatic'`)

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


