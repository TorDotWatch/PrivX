use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct CreatedUserResponse {
    pub(crate) id: Uuid,
}
#[derive(Serialize)]
pub struct UserLoginResponse {
    pub(crate) token: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginQuery {
    pub(crate) user_id: String,
}