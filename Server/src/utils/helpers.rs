use anyhow::Context;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use actix_session::Session;
use crate::db::paste_db_operations::PasteDbOperations;
use crate::db::scylla_db_operations::ScyllaDbOperations;
use rand::{distributions::Alphanumeric, Rng};
use std::env;

// When before sending id to user(in base 62), we append this value to make it longer
// when receiving id from url, we substract this value
pub const ID_ADD: u128 = 229340105584896;
pub const ID_MUL: u128 = 1000000;

pub fn number_text_to_uuid(number: String) -> Uuid {
    let id: u128 =
        number.parse()
        .context("Failed to parse ID as u128").expect("Can't Paste text to u128");
    Uuid::from_u128(id)
}
pub async fn extract_user_id(session: &Session, db: &ScyllaDbOperations) -> Option<Uuid> {
    let token = match session.get::<String>("user_token") {
        Ok(Some(token)) => token,
        Ok(None) => "".to_string(),
        Err(_) => "".to_string()
    };

    if let Ok(Some(user_id)) = db.get_userid_by_token(&token).await {
        return Some(user_id);
    }
    
    None
}

pub fn time_difference_in_seconds(date: Option<DateTime<Utc>>) -> Option<i64> {
    match date {
        Some(date) => {
            let now = Utc::now();
            let duration = date.signed_duration_since(now);
            if duration.num_seconds() >= 0 {
                Some(duration.num_seconds())
            } else {
                Some(0)
            }
        }
        None => None,
    }
}

pub fn convert_seconds_to_largest_unit(seconds: i64) -> String {
    let conversions = vec![
        ("year", 31_536_000),
        ("month", 2_628_000),
        ("day", 86_400),
        ("hour", 3_600),
        ("minute", 60),
        ("second", 1),
    ];

    for (unit, unit_seconds) in conversions {
        let value = seconds as f64 / unit_seconds as f64;
        if value >= 1.0 {
            let rounded_value = value.floor() as i64;
            let plural = if rounded_value > 1 { "s" } else { "" };
            return format!("{} {}{}", rounded_value, unit, plural);
        }
    }

    "0 seconds".to_string()
}

pub fn bigint_to_base62(mut num: u128) -> String {
    const BASE62_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut result = String::new();
    let base = 62;

    if num == 0 {
        return "0".to_string();
    }

    while num > 0 {
        let remainder = (num % base) as usize;
        result.insert(0, BASE62_ALPHABET.chars().nth(remainder).unwrap());
        num /= base;
    }

    result
}

pub fn number_text_to_uint_base62(number: String) -> String {
    let id: u128 =
        number.parse()
        .context("Failed to parse ID as u128").expect("Can't Paste text to u128");
    bigint_to_base62(id)
}

fn base62_to_u128(base62: &str) -> Result<u128, &'static str> {
    const BASE62_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

    let mut result: u128 = 0;
    let base = 62u128;

    for (_, c) in base62.chars().enumerate() {
        let value = BASE62_ALPHABET.find(c)
            .ok_or("Invalid character in Base62 string")? as u128;

        result = result.checked_mul(base)
            .ok_or("Overflow during multiplication")?;
        result = result.checked_add(value)
            .ok_or("Overflow during addition")?;
    }

    Ok(result)
}

pub fn short_paste_id_to_uuid(base62: &str) -> Result<Uuid, &'static str> {
    let id = base62_to_u128(base62)?;
    Ok(Uuid::from_u128((id - ID_ADD) / ID_MUL))
}

pub fn generate_random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn short_paste_id_from_paste_id(id: u128) -> String {
    let paste_id_with_add = (id * ID_MUL + ID_ADD).to_string();
    number_text_to_uint_base62(paste_id_with_add)
}

pub fn allowed_hosts_display() -> Vec<String> {
    let env_var = env::var("ALLOWED_URLS").expect("ALLOWED_URLS env var not set");
    env_var.split(',').map(|s| s.to_string()).collect()
}