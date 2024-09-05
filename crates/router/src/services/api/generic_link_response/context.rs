use rust_i18n::t;
use tera::Context;

pub fn insert_locales_in_context_for_payout_link(context: &mut Context, locale: &str) {
    let i18n_payout_link_title = t!("payout_link.initiate.title", locale = locale);
    let i18n_january = t!("months.january", locale = locale);
    let i18n_february = t!("months.february", locale = locale);
    let i18n_march = t!("months.march", locale = locale);
    let i18n_april = t!("months.april", locale = locale);
    let i18n_may = t!("months.may", locale = locale);
    let i18n_june = t!("months.june", locale = locale);
    let i18n_july = t!("months.july", locale = locale);
    let i18n_august = t!("months.august", locale = locale);
    let i18n_september = t!("months.september", locale = locale);
    let i18n_october = t!("months.october", locale = locale);
    let i18n_november = t!("months.november", locale = locale);
    let i18n_december = t!("months.december", locale = locale);
    let i18n_not_allowed = t!("payout_link.initiate.not_allowed", locale = locale);
    let i18n_am = t!("time.am", locale = locale);
    let i18n_pm = t!("time.pm", locale = locale);

    context.insert("i18n_payout_link_title", &i18n_payout_link_title);
    context.insert("i18n_january", &i18n_january);
    context.insert("i18n_february", &i18n_february);
    context.insert("i18n_march", &i18n_march);
    context.insert("i18n_april", &i18n_april);
    context.insert("i18n_may", &i18n_may);
    context.insert("i18n_june", &i18n_june);
    context.insert("i18n_july", &i18n_july);
    context.insert("i18n_august", &i18n_august);
    context.insert("i18n_september", &i18n_september);
    context.insert("i18n_october", &i18n_october);
    context.insert("i18n_november", &i18n_november);
    context.insert("i18n_december", &i18n_december);
    context.insert("i18n_not_allowed", &i18n_not_allowed);
    context.insert("i18n_am", &i18n_am);
    context.insert("i18n_pm", &i18n_pm);
}

pub fn insert_locales_in_context_for_payout_link_status(context: &mut Context, locale: &str) {
    let i18n_payout_link_status_title = t!("payout_link.status.title", locale = locale);
    let i18n_success_text = t!("payout_link.status.text.success", locale = locale);
    let i18n_success_message = t!("payout_link.status.message.success", locale = locale);
    let i18n_pending_text = t!("payout_link.status.text.processing", locale = locale);
    let i18n_pending_message = t!("payout_link.status.message.processing", locale = locale);
    let i18n_failed_text = t!("payout_link.status.text.failed", locale = locale);
    let i18n_failed_message = t!("payout_link.status.message.failed", locale = locale);
    let i18n_ref_id_text = t!("payout_link.status.info.ref_id", locale = locale);
    let i18n_error_code_text = t!("payout_link.status.info.error_code", locale = locale);
    let i18n_error_message = t!("payout_link.status.info.error_message", locale = locale);
    let i18n_redirecting_text = t!(
        "payout_link.status.redirection_text.redirecting",
        locale = locale
    );
    let i18n_redirecting_in_text = t!(
        "payout_link.status.redirection_text.redirecting_in",
        locale = locale
    );
    let i18n_seconds_text = t!(
        "payout_link.status.redirection_text.seconds",
        locale = locale
    );

    context.insert(
        "i18n_payout_link_status_title",
        &i18n_payout_link_status_title,
    );
    context.insert("i18n_success_text", &i18n_success_text);
    context.insert("i18n_success_message", &i18n_success_message);
    context.insert("i18n_pending_text", &i18n_pending_text);
    context.insert("i18n_pending_message", &i18n_pending_message);
    context.insert("i18n_failed_text", &i18n_failed_text);
    context.insert("i18n_failed_message", &i18n_failed_message);
    context.insert("i18n_ref_id_text", &i18n_ref_id_text);
    context.insert("i18n_error_code_text", &i18n_error_code_text);
    context.insert("i18n_error_message", &i18n_error_message);
    context.insert("i18n_redirecting_text", &i18n_redirecting_text);
    context.insert("i18n_redirecting_in_text", &i18n_redirecting_in_text);
    context.insert("i18n_seconds_text", &i18n_seconds_text);
}
