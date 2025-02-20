
use crate::db::paste_db_operations::PasteDbOperations;
use crate::db::scylla_db_operations::{ScyllaDbOperations};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use crate::{RedisAppState};
use crate::utils::helpers::{allowed_hosts_display, number_text_to_uuid};
use crate::db::redis_operations::dequeue;
use crate::models::user::UserById;
use crate::models::user_vm::LoginQuery;
use tera::{Tera, Context};
use actix_session::{Session};
use uuid::Uuid;

#[get("/user")]
async fn new_user(
    req: HttpRequest,
    db: web::Data<ScyllaDbOperations>,
    redis_con: web::Data<RedisAppState>,
    tera: web::Data<Tera>
) -> impl Responder {
    // Get Unique ID
    let mut con = match redis_con.redis_client.get_connection() {
        Ok(conn) => conn,
        Err(e) => {
            println!("Error connecting to Redis: {}", e);
            let mut context = Context::new();
            context.insert("title", &"Redis connection error".to_string());
            context.insert("error", &"Redis connection error.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        }
    };
    let text_user_id_num = match dequeue(&mut con, "users_ids") {
        Ok(Some(id)) => id,
        Ok(None) => {
            let mut context = Context::new();
            context.insert("title", &"No IDs in queue".to_string());
            context.insert("error", &"No IDs in queue.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::NotFound().body(template)  
        },
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Failed to get a user id".to_string());
            context.insert("error", &"Failed to get a user id.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        }    
    };
    let user_id = number_text_to_uuid(text_user_id_num);
    let user = UserById {
        user_id: user_id,
        user_token: "x".to_string(),
    };
    match db.insert_user_by_id(&user).await {
        Ok(_) => {
        let host = req.connection_info().host().to_string();
        let mut allowed_host: Option<String> = None;
        let alhosts = allowed_hosts_display();

        for url in &alhosts {
            let stripped = url
                .trim_start_matches("http://")
                .trim_start_matches("https://");
    
            if stripped == host {
                allowed_host = Some(url.to_string());
                break;
            }
        }

        let allowed_host = allowed_host.unwrap_or_else(|| alhosts.first().cloned().unwrap_or_else(|| "https://default.url".to_string()));

        let mut context = Context::new();
        context.insert("user_id", &user_id.to_string());
        context.insert("url_base", &allowed_host);
        let template = tera.render("root/after_register.html", &context).expect("Error");
        HttpResponse::Ok().body(template)
    },
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Failed to insert into db".to_string());
            context.insert("error", &"Failed to insert into db.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        }
    }
}
#[get("/user/{user_id}")]
async fn user_login_link(
    db: web::Data<ScyllaDbOperations>,
    user_id: web::Path<String>,
    redis_con: web::Data<RedisAppState>,
    session: Session,
    tera: web::Data<Tera>
) -> impl Responder {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Incorrect credentials".to_string());
            context.insert("error", &"Incorrect credentials.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::NotFound().body(template)
        }
    };
    handle_user_login(db, user_uuid, redis_con, session, tera).await
}

#[get("/user/login")]
async fn user_login(
    db: web::Data<ScyllaDbOperations>,
    query: web::Query<LoginQuery>,
    redis_con: web::Data<RedisAppState>,
    session: Session,
    tera: web::Data<Tera>
) -> impl Responder {
    let user_uuid = match Uuid::parse_str(&query.user_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Incorrect credentials".to_string());
            context.insert("error", &"Incorrect credentials.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::NotFound().body(template)
        }
    };
    handle_user_login(db, user_uuid, redis_con, session, tera).await
}

async fn handle_user_login(
    db: web::Data<ScyllaDbOperations>,
    user_id: Uuid,
    redis_con: web::Data<RedisAppState>,
    session: Session,
    tera: web::Data<Tera>
) -> HttpResponse {
    let user_old_token = match db.get_user_by_id(user_id).await {
        Ok(user) => {
            if user.is_none(){
                let mut context = Context::new();
                context.insert("title", &"User not found.".to_string());
                context.insert("error", &"User not found.".to_string());
                let template = tera.render("base/error.html", &context).expect("Error");
                return HttpResponse::NotFound().body(template)  
            }
            user.unwrap().user_token
        }
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Failed fetching user by id.".to_string());
            context.insert("error", &"Failed fetching user by id.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        }
    };
    // Get New Token
    let mut con = match redis_con.redis_client.get_connection() {
        Ok(conn) => conn,
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Failed connecting to redis.".to_string());
            context.insert("error", &"Failed connecting to redis.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        }
    };
    let new_user_token = match dequeue(&mut con, "users_tokens") {
        Ok(Some(id)) => id,
        Ok(None) => {
            let mut context = Context::new();
            context.insert("title", &"No Ids in que.".to_string());
            context.insert("error", &"No Ids in que.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        },
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Failed to get token.".to_string());
            context.insert("error", &"Failed to get token.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)  
        },
    };
    db.execute_update_token_operations(user_old_token,new_user_token.clone(),&user_id).await.unwrap();
    session.insert("user_token", new_user_token.clone()).unwrap();
    HttpResponse::Found()
        .append_header(("LOCATION", "/"))
        .finish()
}
#[post("/user/logout")]
async fn user_logout(
    db: web::Data<ScyllaDbOperations>,
    session: Session
) -> impl Responder {
    let token = match session.get::<String>("user_token") {
        Ok(Some(token)) => token,
        Ok(None) => "".to_string(),
        Err(_) => "".to_string()
    };
    db.delete_user_token(token).await.unwrap();
    HttpResponse::Found()
        .append_header(("LOCATION", "/"))
        .finish()
}