//!
//! Macros.
//!

pub use crate::log;

///
/// Make a new log Event.
///
/// The event macro is invoked with a Level and up to 32 key-value fields.
/// Optionally, a format string and arguments may follow the fields; this will be used to construct an implicit field named “message”.
/// See the top-level [documentation of tracing](https://docs.rs/tracing/latest/tracing/index.html#using-the-macros) for details on the syntax accepted by this macro.
///
/// # Example
/// ```rust
/// // FIXME: write
/// ```
///

#[macro_export]
macro_rules! log {

    // done

    (
        @MUNCH
        {
            level : { $level:ident },
            tag : { $tag:ident },
            category : { $category:ident },
            flow : { $flow:expr },
            // $( session_id : { $session_id:expr }, )?
            // $( payment_id : { $payment_id:expr }, )?
            // $( payment_attempt_id : { $payment_attempt_id:expr }, )?
            // $( merchant_id : { $merchant_id:expr }, )?
        },
        $( $tail:tt )*
    )
    =>
    (
        ::tracing::event!
        (
            ::router_env::Level::$level,
            level = ?::router_env::Level::$level,
            tag = ?::router_env::Tag::$tag,
            category = ?::router_env::Category::$category,
            flow = ?$flow,
            // $( session_id = $session_id, )?
            // $( payment_id = $payment_id, )?
            // $( payment_attempt_id = $payment_attempt_id, )?
            // $( merchant_id = $merchant_id, )?
            $( $tail )*
        );
    );

    // entry with colon

    (
        level : $level:ident,
        tag : $tag:ident,
        category : $category:ident,
        flow : $(?)?$flow:expr,
        $( $tail:tt )*
    )
    =>
    (
        $crate::log!
        {
            @MUNCH
            {
                level : { $level },
                tag : { $tag },
                category : { $category },
                flow : { $flow },
            },
            $( $tail )*
        }
    );

    // entry without colon

    (
        $level:ident,
        $tag:ident,
        $category:ident,
        $flow:expr,
        $( $tail:tt )*
    )
    =>
    (
        $crate::log!
        {
            @MUNCH
            {
                level : { $level },
                tag : { $tag },
                category : { $category },
                flow : { $flow },
            },
            $( $tail )*
        }
    );

}
