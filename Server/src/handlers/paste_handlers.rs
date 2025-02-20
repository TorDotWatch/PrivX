
use crate::Config;
use crate::db::paste_db_operations::PasteDbOperations;
use crate::db::scylla_db_operations::{ScyllaDbOperations};
use crate::models::paste_vm::{CreatePasteRequest, GetPasteRequest, GetPasteGenInfo};
use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::{Duration, Utc};
use tera::{Tera, Context};
use uuid::Uuid;
use crate::{RedisAppState};
use crate::utils::helpers::{allowed_hosts_display, short_paste_id_from_paste_id, generate_random_string, short_paste_id_to_uuid, extract_user_id, number_text_to_uuid, time_difference_in_seconds, convert_seconds_to_largest_unit};
use crate::db::redis_operations::dequeue;
use crate::models::paste::{PasteById, DEFAULT_SIGNATURE};
use crate::models::crypto::{encrypt_data, decrypt_data};
use actix_session::Session;
use std::collections::HashMap;
use regex::Regex;

#[post("/{paste_id}")]
async fn get_paste_post(
    db: web::Data<ScyllaDbOperations>,
    session: Session,
    paste_id: web::Path<String>,
    paste_data: web::Form<GetPasteRequest>,
    tera: web::Data<Tera>
) -> impl Responder {
    handle_get_paste(db, session, paste_id, Some(paste_data), tera, true).await
}

pub async fn get_paste_get(
    db: web::Data<ScyllaDbOperations>,
    session: Session,
    paste_id: web::Path<String>,
    tera: web::Data<Tera>
) -> impl Responder {
    handle_get_paste(db, session, paste_id, None, tera, false).await
}

pub async fn get_paste_with_secret(
    db: web::Data<ScyllaDbOperations>,
    session: Session,
    params: web::Path<(String, String)>,
    tera: web::Data<Tera>
) -> impl Responder {
    let paste_data = web::Form(GetPasteRequest {
        ivkey: "".to_string(),
        secret: params.1.clone()
    });
    handle_get_paste(db, session, web::Path::from(params.0.clone()), Some(paste_data), tera, false).await
}

pub async fn get_paste_with_secret_with_placeholder(
    db: web::Data<ScyllaDbOperations>,
    session: Session,
    params: web::Path<(String, String, String)>,
    tera: web::Data<Tera>
) -> impl Responder {
    let paste_data = web::Form(GetPasteRequest {
        ivkey: "".to_string(),
        secret: params.1.clone()
    });
    handle_get_paste(db, session, web::Path::from(params.0.clone()), Some(paste_data), tera, false).await
}

async fn handle_get_paste(
    db: web::Data<ScyllaDbOperations>,
    session: Session,
    paste_id_base62: web::Path<String>,
    paste_d: Option<web::Form<GetPasteRequest>>, // Optional for GET
    tera: web::Data<Tera>,
    show_val_error: bool,
) -> impl Responder {
    let paste_id = match short_paste_id_to_uuid(&paste_id_base62) {
        Ok(uuid) => uuid,
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Invalid paste id".to_string());
            context.insert("error", &"Invalid paste id.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::NotFound().body(template) 
        },
    };

    let paste_data = paste_d.unwrap_or(web::Form(GetPasteRequest {
        ivkey: "".to_string(),
        secret: "".to_string()
    }));
    match db.get_paste_by_id(paste_id).await {
        Ok(Some(paste)) => {
            // Expiration
            if paste.expire.is_some(){
                if paste.expire.unwrap()<Utc::now(){
                    if paste.user_id != None {
                        db.delete_paste_by_user_id(&paste_id,&paste.user_id.unwrap()).await.expect("Can't Delete The Paste");
                    }else{
                        db.delete_paste_by_id(&paste_id).await.expect("Can't Delete The Paste");
                    }
                    let mut context = Context::new();
                    context.insert("title", &"Not found".to_string());
                    context.insert("error", &"The paste is not found.".to_string());
                    let template = tera.render("base/error.html", &context).expect("Error");
                    return HttpResponse::NotFound().body(template)                }
            }
            // Check signature
            let signature: &str = &decrypt_data(&paste_data.ivkey, &paste_data.secret, &paste.signature);
            let mut views = 0;
            if signature != DEFAULT_SIGNATURE {
                let mut context = Context::new();
                if show_val_error {
                    context.insert("error", &"Invalid secret/ivkey".to_string());
                }
                context.insert("paste_id", &paste_id_base62.to_string());
                context.insert("secret", &paste_data.secret);
                context.insert("ivkey", &paste_data.ivkey);
                let template = tera.render("paste/ask_secrets.html", &context).expect("Error");
                return HttpResponse::BadRequest().body(template)
            }
            // Burn
            if paste.burn {
                if paste.user_id != None {
                    db.delete_paste_by_user_id(&paste_id,&paste.user_id.unwrap()).await.expect("Can't Delete The Paste");
                }else{
                    db.delete_paste_by_id(&paste_id).await.expect("Can't Delete The Paste");
                }
            }else{
                // Increment
                db.increment_view_count_by_paste_id(paste_id).await.expect("Cant' Increment Views");
                // Views
                match db.get_view_count_by_paste_id(paste_id).await {
                    Ok(Some(v)) => {
                        views = v.0;
                    }
                    Ok(None) => {
                        let mut context = Context::new();
                        context.insert("title", &"Internal server error".to_string());
                        context.insert("error", &"Internal server error.".to_string());
                        let template = tera.render("base/error.html", &context).expect("Error");
                        return HttpResponse::InternalServerError().body(template)  
                    }
                    Err(_) => {
                        let mut context = Context::new();
                        context.insert("title", &"Internal server error".to_string());
                        context.insert("error", &"Internal server error.".to_string());
                        let template = tera.render("base/error.html", &context).expect("Error");
                        return HttpResponse::InternalServerError().body(template)
                    }
                }
            }
            let user_id = match extract_user_id(&session, &db).await {
                Some(id) => Some(id),
                None => None
            };
            let title = decrypt_data(&paste_data.ivkey, &paste_data.secret, &paste.title);
            let content = decrypt_data(&paste_data.ivkey, &paste_data.secret, &paste.content);
            let syntax = decrypt_data(&paste_data.ivkey, &paste_data.secret, &paste.syntax.clone().unwrap_or("".to_string()));
            let expire = match paste.expire {
                Some(expire) => expire.format("%H:%M %d %B %Y").to_string(),
                None => "Never".to_string()
            };
        
            let mut context = Context::new();
            context.insert("views", &views.to_string());
            context.insert("title", &title.to_string());
            context.insert("content", &content.to_string());
            context.insert("syntax", &syntax.to_string());
            context.insert("signature", &signature.to_string());
            context.insert("is_logged_in", &!user_id.is_none());
            context.insert("expire", &expire);
            let template = tera.render("paste/show.html", &context).expect("Error");
            HttpResponse::Ok().body(template)      
        }
        Ok(None) => {
            let mut context = Context::new();
            context.insert("title", &"Not found".to_string());
            context.insert("error", &"The paste is not found.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::NotFound().body(template)
        }, 
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Internal server error".to_string());
            context.insert("error", &"Internal server error.".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)
        }
    }
}

#[get("/paste")]
async fn get_user_pastes(
    session: Session,
    db: web::Data<ScyllaDbOperations>,
    tera: web::Data<Tera>
) -> impl Responder {
    // User Id extraction with error handling
    let user_id = match extract_user_id(&session, &db).await {
        Some(id) => id,
        None => {
            let mut context = Context::new();
            context.insert("title", &"Unauthorized: Invalid user credentials".to_string());
            context.insert("error", &"Unauthorized: Invalid user credentials".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::Unauthorized().body(template)
        }
    };

    // Fetch all paste IDs for the user
    let pastes = match db.get_pastes_by_userid(user_id).await {
        Ok(pastes_uuid) => pastes_uuid,
        Err(_) => {
            let mut context = Context::new();
            context.insert("title", &"Internal server error while retrieving pastes".to_string());
            context.insert("error", &"Internal server error while retrieving pastes".to_string());
            let template = tera.render("base/error.html", &context).expect("Error");
            return HttpResponse::InternalServerError().body(template)
        }
    };

    // Gather information for each paste, with improved error handling
    let mut user_pastes: Vec<GetPasteGenInfo> = Vec::new();
    for uuid in &pastes {
        let paste = match db.get_paste_info_by_id(uuid.clone()).await {
            Ok(Some(paste_info)) => paste_info,
            Ok(None) => {
                let mut context = Context::new();
                context.insert("title", &"Paste not found".to_string());
                context.insert("error", &"Paste not found.".to_string());
                let template = tera.render("base/error.html", &context).expect("Error");
                return HttpResponse::NotFound().body(template)  
            }
            Err(_) => {
                let mut context = Context::new();
                context.insert("title", &"Error retrieving paste information".to_string());
                context.insert("error", &"Error retrieving paste information.".to_string());
                let template = tera.render("base/error.html", &context).expect("Error");
                return HttpResponse::InternalServerError().body(template)
            }
        };

        let views = match db.get_view_count_by_paste_id(uuid.clone()).await {
            Ok(Some(views)) => views.0,
            Ok(None) => 0i64, // No views recorded, default to 0
            Err(_) => {
                let mut context = Context::new();
                context.insert("title", &"Error retrieving view count".to_string());
                context.insert("error", &"Error retrieving view count.".to_string());
                let template = tera.render("base/error.html", &context).expect("Error");
                return HttpResponse::InternalServerError().body(template)
            }
        };

        let diff_in_sec = match paste.expire {
            Some(expire) => time_difference_in_seconds(Some(expire)),
            None => Some(99999999)
        };

        let new_paste_info = GetPasteGenInfo {
            id: short_paste_id_from_paste_id(uuid.as_u128()),
            burn: paste.burn,
            expire: Some(convert_seconds_to_largest_unit(diff_in_sec.unwrap())),
            views,
        };
        user_pastes.push(new_paste_info);
    }
    let user_id = match extract_user_id(&session, &db).await {
        Some(id) => Some(id),
        None => None
    };

    let mut context = Context::new();
    context.insert("is_logged_in", &!user_id.is_none());
    context.insert("pastes", &user_pastes);
    let template = tera.render("paste/my_pastes.html", &context).expect("Error");
    HttpResponse::Ok().body(template)
}

#[post("/paste")]
async fn create_paste(
    session: Session,
    db: web::Data<ScyllaDbOperations>,
    redis_con: web::Data<RedisAppState>,
    config: web::Data<Config>,
    paste_data: web::Form<CreatePasteRequest>,
    tera: web::Data<Tera>,
) -> impl Responder {
    // UserID
    let user_id: Option<Uuid>;
    user_id = match extract_user_id(&session, &db).await {
        Some(id) => Option::from(id),
        _ => {None}
    };

    let mut should_render_gen = false;
    let mut secret = paste_data.secret.clone();
    let mut ivkey = paste_data.ivkey.clone();
    if paste_data.gen_random_secret.is_some() && paste_data.gen_random_secret.unwrap() {
        should_render_gen = true;
        secret = generate_random_string(32);
    }
    if paste_data.gen_random_ivkey.is_some() && paste_data.gen_random_ivkey.unwrap() {
        should_render_gen = true;
        ivkey = generate_random_string(16);
    }

    if should_render_gen {
        let burn = match paste_data.burn {
            Some(b) => b,
            None => false,
        };
        let expire = match paste_data.expire {
            Some(e) => e,
            None => 0,
        };
        let mut context = Context::new();
        context.insert("is_logged_in", &!user_id.is_none());
        context.insert("secret", &secret.to_string());
        context.insert("ivkey", &ivkey.to_string());
        context.insert("title", &paste_data.title.to_string());
        context.insert("content", &paste_data.content.to_string());
        context.insert("expire", &expire);
        context.insert("burn", &burn);
        let template = tera.render("root/index.html", &context).expect("Error");
        return HttpResponse::Ok().body(template);
    }
    
    // Title
    if paste_data.title.len() > config.max_title_length as usize {
        return render_paste_form_with_val_error(
            paste_data,
            tera,
            user_id, 
            format!("Title must not exceed {} bytes", config.max_title_length).to_string()).await
    }
    // Content Size
    if paste_data.content.len() > config.max_content_kb as usize || paste_data.content.len() < 3 {
        return render_paste_form_with_val_error(
            paste_data,
            tera,
            user_id, 
            format!("Content size must be between 3 and {} bytes", config.max_content_kb).to_string()).await
    }
    let mut duration = 0;
    // Expiration
    let expiration_time = match paste_data.expire {
        Some(seconds) => {
            if seconds == 0 {
                None
            } else if seconds >= config.min_paste_duration && seconds <= config.max_paste_duration {
                duration=seconds;
                Some(Utc::now() + Duration::seconds(seconds as i64))
            } else {
                return render_paste_form_with_val_error(
                    paste_data,
                    tera,
                    user_id, 
                    "Expiration time must be between 1 minute and 1 year".to_string()).await
            }
        }
        None => None,
    };

    // Syntax
    if paste_data.syntax.is_some() && paste_data.syntax.clone().unwrap_or("".to_string()).len() > config.max_syntax_length as usize {
        return render_paste_form_with_val_error(
            paste_data,
            tera,
            user_id, 
            format!("Syntax must not exceed {} bytes", config.max_syntax_length).to_string()).await
    }
    // Get Unique ID
    let mut con = match redis_con.redis_client.get_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return render_paste_form_with_val_error(
                paste_data,
                tera,
                user_id, 
                "Failed to get unique id".to_string()).await
        },
    };
    let text_paste_id_num = match dequeue(&mut con, "paste_ids") {
        Ok(Some(id)) => id,
        Ok(None) => {
            return render_paste_form_with_val_error(
                paste_data,
                tera,
                user_id, 
                "No IDs in queue".to_string()).await
        },
        Err(_) => {
            return render_paste_form_with_val_error(
                paste_data,
                tera,
                user_id, 
                "Failed to get ids from queue".to_string()).await
        },
    };
    let paste_id = number_text_to_uuid(text_paste_id_num.clone());
    // Generate signature    
    let enc_sig = encrypt_data(&paste_data.ivkey, &paste_data.secret, DEFAULT_SIGNATURE);
    let enc_content = encrypt_data(&paste_data.ivkey, &paste_data.secret, &paste_data.content.clone());
    let enc_title = encrypt_data(&paste_data.ivkey, &paste_data.secret, &paste_data.title.clone());
    let enc_syntax = encrypt_data(&paste_data.ivkey, &paste_data.secret, &paste_data.syntax.clone().unwrap_or("".to_string()));
    let enc_syntax_option: Option<String> = Some(enc_syntax.clone());

    // Save In DB
    let new_paste = PasteById {
        paste_id,
        title: enc_title.clone(),
        signature: enc_sig.clone(),
        content: enc_content.clone(),
        syntax: enc_syntax_option.clone(),
        expire: expiration_time,
        burn: paste_data.burn.unwrap_or(false),
        user_id,
    };
    match db.insert_paste(&new_paste, duration).await {
        Ok(_) => {
            if let Some(user_id) = new_paste.user_id {
                if let Err(e) = db.insert_paste_by_user_id(user_id, paste_id,duration).await {
                    eprintln!("Failed to associate paste with user: {:?}", e);
                }
            }
            let paste_id_u128: u128 = text_paste_id_num.parse()
            .expect("Can't Paste text to u128");
            let short_paste_id = short_paste_id_from_paste_id(paste_id_u128);
            let mut context = Context::new();
            context.insert("paste_id", &short_paste_id.to_string());
            context.insert("allowed_hosts", &allowed_hosts_display());
            context.insert("has_secret", &(paste_data.secret.len() >= 1));
            context.insert("secret", &paste_data.secret);
            context.insert("has_ivkey", &(paste_data.ivkey.len() >= 1));
            context.insert("ivkey", &paste_data.ivkey);
            let template = tera.render("paste/copy_link.html", &context).expect("Error");
            HttpResponse::Created().body(template)
        }
        Err(_) => {
            return render_paste_form_with_val_error(
                paste_data,
                tera,
                user_id, 
                "Failed to create paste".to_string()).await
        }
    }
}
async fn render_paste_form_with_val_error(
    paste_data: web::Form<CreatePasteRequest>,
    tera: web::Data<Tera>,
    user_id: Option<Uuid>,
    val_error: String
) -> HttpResponse {
    let burn = match paste_data.burn {
        Some(b) => b,
        None => false,
    };
    let expire = match paste_data.expire {
        Some(e) => e,
        None => 0,
    };

    let mut context = Context::new();
    context.insert("is_logged_in", &!user_id.is_none());
    context.insert("secret", &paste_data.secret.clone().to_string());
    context.insert("ivkey", &paste_data.ivkey.clone().to_string());
    context.insert("title", &paste_data.title.to_string());
    context.insert("content", &paste_data.content.to_string());
    context.insert("expire", &expire);
    context.insert("burn", &burn);
    context.insert("val_error", &val_error);
    let template = tera.render("root/index.html", &context).expect("Error");
    HttpResponse::Ok().body(template)
}
#[post("/delete/paste")]
async fn delete_paste(
    session: Session,
    db: web::Data<ScyllaDbOperations>,
    payload: web::Form<HashMap<String, String>>,
    tera: web::Data<Tera>
) -> impl Responder {
    let uuid_regex = Regex::new(r"paste_ids\[(.*?)\]").unwrap();
    for (key, value) in payload.iter() {
        if value != "yes" {
            continue;
        }
        if let Some(captures) = uuid_regex.captures(key) {
            // Extract the UUID from the capture group
            if let Some(uuid_short) = captures.get(1) {
                match short_paste_id_to_uuid(&uuid_short.as_str()) {
                    Ok(uuid) => {
                        // UserID
                        let user_id = match extract_user_id(&session, &db).await {
                            Some(id) => id,
                            None => {
                                let mut context = Context::new();
                                context.insert("title", &"Unauthorized".to_string());
                                context.insert("error", &"Unauthorized.".to_string());
                                let template = tera.render("base/error.html", &context).expect("Error");
                                return HttpResponse::Unauthorized().body(template)  
                            },
                        };
                        // Check + Delete
                        match db.check_paste_by_userid(&user_id, &uuid).await {
                            Ok(true) => match db.delete_paste_by_user_id(&uuid, &user_id).await {
                                Ok(_) => {
                                    println!("Paste deleted successfully, uuid: {:?}", uuid);
                                },
                                Err(_) => {
                                    println!("Failed to delete paste");
                                },
                            },
                            Ok(false) => {
                                println!("Paste not found");
                            },
                            Err(_) => {
                                println!("Failed to fetch paste");
                            },
                        }
                    },
                    Err(_) => {},
                };
            }
        }
    }

    HttpResponse::Found()
        .append_header(("LOCATION", "/paste"))
        .finish()
}
