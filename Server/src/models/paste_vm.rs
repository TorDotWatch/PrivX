use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
pub struct CreatePasteRequest {
    pub(crate) title: String,
    pub(crate) syntax: Option<String>,
    pub(crate) expire: Option<i32>,
    pub(crate) burn: Option<bool>,
    pub(crate) secret: String,
    pub(crate) ivkey: String,
    pub(crate) content: String,
    pub(crate) gen_random_secret: Option<bool>,
    pub(crate) gen_random_ivkey: Option<bool>,
}
#[derive(Deserialize)]
pub struct GetPasteRequest {
    pub(crate) secret: String,
    pub(crate) ivkey: String,
}
#[derive(Serialize)]
pub struct CreatedPasteResponse {
    pub(crate) id: Uuid,
}
#[derive(Serialize)]
pub struct PasteResponse {
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) signature: String,
    pub(crate) syntax: Option<String>,
    pub(crate) expire: Option<i64>,
    pub(crate) views: i64,
}
#[derive(Serialize)]
pub struct GetPasteGenInfo {
    pub(crate) id: String,
    pub(crate) burn: bool,
    pub(crate) expire: Option<String>,
    pub(crate) views: i64,
}
#[derive(Serialize, Deserialize)]
pub struct PasteIds {
    pub(crate) paste_ids: Vec<Uuid>,
}