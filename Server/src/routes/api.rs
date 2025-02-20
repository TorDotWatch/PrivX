use actix_web::{web};
use crate::handlers::auth_handlers::{new_user, user_login, user_login_link, user_logout};
use crate::handlers::paste_handlers::{create_paste, delete_paste, get_paste_get, get_paste_post, get_paste_with_secret, get_user_pastes, get_paste_with_secret_with_placeholder};
use crate::handlers::root::{index, login, help};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
        .service(index)       // GET /
        .service(login)       // GET /login
        .service(help)       // GET /login

        // User endpoints
        .service(new_user)        // GET /user
        .service(user_login)      // POST /user
        .service(user_login_link) // GET /user/login
        .service(user_logout)     // DELETE /user

        // Paste endpoints
        .service(get_user_pastes) // GET /paste
        .service(create_paste)    // POST /paste
        .service(delete_paste)    // POST /delete/paste

        .service(get_paste_post)       // POST /{paste_id}
        .service(web::resource(["/{paste_id}", "/{paste_id}/"]).to(get_paste_get))  // GET /{paste_id}(/)
        .service(web::resource(["/{paste_id}/{secret}", "/{paste_id}/{secret}/"]).to(get_paste_with_secret)) // GET /{paste_id}/{secret}(/)
        .service(web::resource(["/{paste_id}/{secret}/{placeholder}", "/{paste_id}/{secret}/{placeholder}/"]).to(get_paste_with_secret_with_placeholder)) // GET /{paste_id}/{secret}/{placeholder}{/}
    );    
}
