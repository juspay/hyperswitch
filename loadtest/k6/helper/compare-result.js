import { textSummary } from "./k6-summary.js";

function compute(baseline, result) {
    return ((1 - (result/baseline)) * -100).toFixed(2)
}

function buildResult(baseline, data) {
    return function(metrics, fields) {
        const result = {}
        metrics.forEach(metric => {
            result[metric] = {}
            fields.forEach(field => {
                let a = baseline.metrics[metric]["values"][field]
                let b = data.metrics[metric]["values"][field]
                result[metric][field] = compute(a, b) + "%"
            })
        })
        return result
    }
}

function compareResult(baseline, data) {
    // note: we can add more metrics. we grouped them based on the field they contain.
    // todo: add validation whether threshold/check are being matched or vus are the same as baseline run.
    const computeResult = buildResult(baseline, data)

    const group1 = ["http_req_duration"]
    const group1_fields = data.options.summaryTrendStats
    const group1_result = computeResult(group1, group1_fields)

    const group2 = ["http_reqs"]
    const group2_fields = ["rate"]
    const group2_result = computeResult(group2, group2_fields)

    return Object.assign(group1_result, group2_result)
}

export function readBaseline(scenario) {
    let baseline = null;
    if (__ENV.LOADTEST_RUN_NAME != "baseline") {
        baseline = JSON.parse(open(`./benchmark/baseline_${scenario}.json`, "r"))
    }
    return baseline
}

export function storeResult(scenario, baseline, data) {
    const file = `/scripts/benchmark/${__ENV.LOADTEST_RUN_NAME}_${scenario}`
    const summary = textSummary(data, { indent: ' ', enableColors: false })
    const result = {
        [`${file}.json`]: JSON.stringify(data, null, 2),
        [`${file}.summary`]: summary
    }
    if (baseline != null) {
        const diff = JSON.stringify(compareResult(baseline, data), null, 2)
        result[`${file}.diff`] = diff
        result["stdout"] = diff
    } else {
        result["stdout"] = summary
    }
    return result
}