use common_enums::Country;
use common_types::three_ds_decision_rule_engine::ThreeDSDecision;

// List of countries that are part of the PSD2 region
const PSD2_COUNTRIES: [Country; 27] = [
    Country::Austria,
    Country::Belgium,
    Country::Bulgaria,
    Country::Croatia,
    Country::Cyprus,
    Country::Czechia,
    Country::Denmark,
    Country::Estonia,
    Country::Finland,
    Country::France,
    Country::Germany,
    Country::Greece,
    Country::Hungary,
    Country::Ireland,
    Country::Italy,
    Country::Latvia,
    Country::Lithuania,
    Country::Luxembourg,
    Country::Malta,
    Country::Netherlands,
    Country::Poland,
    Country::Portugal,
    Country::Romania,
    Country::Slovakia,
    Country::Slovenia,
    Country::Spain,
    Country::Sweden,
];

// function to apply PSD2 validations to the decision
pub fn apply_psd2_validations_during_execute(
    decision: ThreeDSDecision,
    request: &api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> ThreeDSDecision {
    let issuer_in_psd2 = request
        .issuer
        .as_ref()
        .and_then(|issuer| issuer.country)
        .map(|country| PSD2_COUNTRIES.contains(&country))
        .unwrap_or(false);
    let acquirer_in_psd2 = request
        .acquirer
        .as_ref()
        .and_then(|acquirer| acquirer.country)
        .map(|country| PSD2_COUNTRIES.contains(&country))
        .unwrap_or(false);
    if issuer_in_psd2 && acquirer_in_psd2 {
        // If both issuer and acquirer are in PSD2 region
        match decision {
            // If the decision is to enforce no 3DS, override it to enforce 3DS
            ThreeDSDecision::NoThreeDs => ThreeDSDecision::ChallengeRequested,
            _ => decision,
        }
    } else {
        // If PSD2 doesn't apply, exemptions cannot be applied
        match decision {
            ThreeDSDecision::NoThreeDs => ThreeDSDecision::NoThreeDs,
            // For all other decisions (including exemptions), enforce challenge as exemptions are only valid in PSD2 regions
            _ => ThreeDSDecision::ChallengeRequested,
        }
    }
}
