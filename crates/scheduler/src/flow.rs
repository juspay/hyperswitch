#[derive(Copy, Clone, Debug, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum SchedulerFlow {
    Producer,
    Consumer,
    Cleaner,
}
