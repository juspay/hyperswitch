pub struct OrganizationNew {
    pub org_id: String,
    pub org_name: Option<String>,
}

impl OrganizationNew {
        /// Creates a new instance of the struct, with the option to provide an organization name.
    pub fn new(org_name: Option<String>) -> Self {
        Self {
            org_id: common_utils::generate_id_with_default_len("org"),
            org_name,
        }
    }
}
