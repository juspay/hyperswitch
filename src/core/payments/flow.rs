use bach::models::Payments;

use diesel::prelude::*;

pub fn show_payments(conn: &PgConnection) -> Vec<Payments> {
    use bach::schema::payments::dsl::*;

    payments.filter(status.eq("NEW"))
        .limit(1)
        .load::<Payments>(conn)
        .expect("Error fetching payments")
}
