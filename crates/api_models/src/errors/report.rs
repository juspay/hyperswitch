use error_stack::Report;

pub trait ReportSwitchExt<T> {
    fn switch(self) -> T;
}

impl<T, U, V> ReportSwitchExt<Result<T, Report<U>>> for Result<T, Report<V>>
where
    V: ErrorSwitchExt<U> + error_stack::Context,
    U: error_stack::Context,
{
    fn switch(self) -> Result<T, Report<U>> {
        match self {
            Ok(i) => Ok(i),
            Err(er) => {
                let new_c = er.current_context().switch();
                Err(er.change_context(new_c))
            }
        }
    }
}

pub trait ErrorSwitchExt<T> {
    fn switch(&self) -> T;
}
