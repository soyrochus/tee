#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Principal {
    pub subject: String,
    pub display_name: String,
    pub email: Option<String>,
    pub tenant_id: Option<String>,
    pub roles: Vec<String>,
}
