pub static PROFILER_GUARD: once_cell::sync::OnceCell<pprof::ProfilerGuard<'static>> =
    once_cell::sync::OnceCell::new();
