#![allow(unused, clippy::expect_used)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use euclid::{
    backend::{inputs, EuclidBackend, InterpreterBackend, VirInterpreterBackend},
    enums,
    frontend::ast::{self, parser},
    types::DummyOutput,
};

fn get_program_data() -> (ast::Program<DummyOutput>, inputs::BackendInput) {
    let code1 = r#"
        default: ["stripe", "adyen", "checkout"]

        stripe_first: ["stripe", "aci"]
        {
            payment_method = card & amount = 40 {
                payment_method = (card, bank_redirect)
                amount = (40, 50)
            }
        }

        adyen_first: ["adyen", "checkout"]
        {
            payment_method = bank_redirect & amount > 60 {
                payment_method = (card, bank_redirect)
                amount = (40, 50)
            }
        }

        auth_first: ["authorizedotnet", "adyen"]
        {
            payment_method = wallet
        }
    "#;

    let inp = inputs::BackendInput {
        metadata: None,
        payment: inputs::PaymentInput {
            amount: 32,
            card_bin: None,
            currency: enums::Currency::USD,
            authentication_type: Some(enums::AuthenticationType::NoThreeDs),
            capture_method: Some(enums::CaptureMethod::Automatic),
            business_country: Some(enums::Country::UnitedStatesOfAmerica),
            billing_country: Some(enums::Country::France),
            business_label: None,
            setup_future_usage: None,
        },
        payment_method: inputs::PaymentMethodInput {
            payment_method: Some(enums::PaymentMethod::PayLater),
            payment_method_type: Some(enums::PaymentMethodType::Sofort),
            card_network: None,
        },
        mandate: inputs::MandateData {
            mandate_acceptance_type: None,
            mandate_type: None,
            payment_type: None,
        },
    };

    let (_, program) = parser::program(code1).expect("Parser");

    (program, inp)
}

fn interpreter_vs_jit_vs_vir_interpreter(c: &mut Criterion) {
    let (program, binputs) = get_program_data();

    let interp_b = InterpreterBackend::with_program(program.clone()).expect("Interpreter backend");

    let vir_interp_b =
        VirInterpreterBackend::with_program(program).expect("Vir Interpreter Backend");

    c.bench_function("Raw Interpreter Backend", |b| {
        b.iter(|| {
            interp_b
                .execute(binputs.clone())
                .expect("Interpreter EXECUTION");
        });
    });

    c.bench_function("Valued Interpreter Backend", |b| {
        b.iter(|| {
            vir_interp_b
                .execute(binputs.clone())
                .expect("Vir Interpreter execution");
        })
    });
}

criterion_group!(benches, interpreter_vs_jit_vs_vir_interpreter);
criterion_main!(benches);
