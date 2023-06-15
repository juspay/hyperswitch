// Add struct fields as necessary
#[allow(dead_code)]
#[derive(Debug)]
pub struct SchedulerConfig {
    raw_body: String,
    headers: Vec<(String, String)>,
    raw_headers: Vec<(String, String)>,
}

impl SchedulerConfig {
    // Add parameters to new as required
    pub fn new(headers: Vec<(String, String)>) -> Self {
        Self {
            raw_body: String::new(),
            headers,
            raw_headers: Vec::new(),
        }
    }
}
