use seekwel::model;

#[model]
pub struct App {
    pub id: u64,
    pub name: String,
    pub bundle_identifier: String,
}
