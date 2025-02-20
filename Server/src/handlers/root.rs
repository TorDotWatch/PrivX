
use tera::{Tera, Context};
use actix_web::{get, HttpResponse, Responder};
use actix_web::web;
use actix_session::Session;
use crate::utils::helpers::{extract_user_id};
use crate::db::scylla_db_operations::{ScyllaDbOperations};
use crate::models::paste::DEFAULT_EXPIRE;

#[get("/")]
async fn index(
    db: web::Data<ScyllaDbOperations>,
    tera: web::Data<Tera>,
    session: Session,
) -> impl Responder {
    let user_id = match extract_user_id(&session, &db).await {
        Some(id) => Some(id),
        None => None
    };

    let mut context = Context::new();
    context.insert("is_logged_in", &!user_id.is_none());
    context.insert("expire", &DEFAULT_EXPIRE);
    let template = tera.render("root/index.html", &context).expect("Error");
    HttpResponse::Ok().body(template)
}

#[get("/login")]
async fn login(
    tera: web::Data<Tera>
) -> impl Responder {
    let context = Context::new();
    let template = tera.render("root/login.html", &context).expect("Error");
    HttpResponse::Ok().body(template)
}


#[get("/help")]
async fn help(
    db: web::Data<ScyllaDbOperations>,
    tera: web::Data<Tera>,
    session: Session,
) -> impl Responder {
    let user_id = match extract_user_id(&session, &db).await {
        Some(id) => Some(id),
        None => None
    };

    let mut context = Context::new();
    context.insert("is_logged_in", &!user_id.is_none());
    context.insert("expire", &DEFAULT_EXPIRE);
    let template = tera.render("root/help.html", &context).expect("Error");
    HttpResponse::Ok().body(template)
}