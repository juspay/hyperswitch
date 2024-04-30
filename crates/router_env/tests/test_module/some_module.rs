use logger::instrument;
use router_env as logger;

//#\[instrument\(skip_all)]
pub async fn fn_with_colon(val: i32) {
    let a = 13;
    let b = 31;

    logger::log!(
        logger::Level::WARN,
        ?a,
        ?b,
        tag = ?logger::Tag::ApiIncomingRequest,
        category = ?logger::Category::Api,
        flow = "some_flow",
        session_id = "some_session",
        answer = 13,
        message2 = "yyy",
        message = "Experiment",
        val,
    );

    fn_without_colon(131).await;
}

//#\[instrument\(fields(val3 = "abc"), skip_all)]
pub async fn fn_without_colon(val: i32) {
    let a = 13;
    let b = 31;

    // trace_macros!(true);
    logger::log!(
        logger::Level::INFO,
        ?a,
        ?b,
        tag = ?logger::Tag::ApiIncomingRequest,
        category = ?logger::Category::Api,
        flow = "some_flow",
        session_id = "some_session",
        answer = 13,
        message2 = "yyy",
        message = "Experiment",
        val,
    );
    // trace_macros!(false);
}
