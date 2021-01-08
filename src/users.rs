use crate::{
    authentication,
    diesel::BoolExpressionMethods,
    models::UserLabelSet,
    schema::{self, users::dsl::*},
    MainDbConn,
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use rocket::{
    delete, get,
    http::{Cookie, Cookies},
    post, put,
};
use rocket_contrib::{json::Json, uuid::Uuid};
use serde::Deserialize;
use sodiumoxide::crypto::pwhash::argon2id13;
use std::error::Error;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[post("/login", format = "json", data = "<data>")]
pub fn login(
    conn: MainDbConn,
    mut cookies: Cookies,
    data: Json<Login>,
) -> Result<(), Box<dyn Error>> {
    let results = users
        .filter(username.eq(&data.username))
        .load::<crate::models::User>(&*conn)?;
    let user = results.get(0).ok_or("Could not find user.")?;

    sodiumoxide::init().map_err(|_| "Failed to init sodiumoxide.")?;
    let hash = argon2id13::HashedPassword::from_slice(&user.password)
        .ok_or("Could not recover password hash")?;
    let password_matches = argon2id13::pwhash_verify(&hash, data.password.as_bytes());

    if !password_matches {
        return Err("Incorrect password.".into());
    }

    add_login_cookie(&mut cookies, user.id);
    Ok(())
}

#[post("/logout")]
pub fn logout(_user: &authentication::User, mut cookies: Cookies) -> Result<(), !> {
    remove_login_cookie(&mut cookies);
    Ok(())
}

#[put("/create", format = "json", data = "<data>")]
pub fn create(
    conn: MainDbConn,
    mut cookies: Cookies,
    data: Json<Login>,
) -> Result<(), Box<dyn Error>> {
    sodiumoxide::init().map_err(|_| "Failed to init sodiumoxide.")?;
    let hash = argon2id13::pwhash(
        data.password.as_bytes(),
        argon2id13::OPSLIMIT_INTERACTIVE,
        argon2id13::MEMLIMIT_INTERACTIVE,
    )
    .map_err(|_| "Failed to hash password.")?;

    let insert = super::models::NewUser {
        username: data.username.as_ref(),
        password: hash.as_ref(),
    };

    rocket_contrib::databases::diesel::insert_into(schema::users::table)
        .values(&insert)
        .execute(&*conn)?;
    let user = users
        .filter(username.eq(insert.username))
        .load::<crate::models::User>(&*conn)?
        .pop()
        .ok_or("Could not find user that was just inserted!")?;

    add_login_cookie(&mut cookies, user.id);
    Ok(())
}

#[post("/refresh", rank = 1)]
pub fn refresh_session_user(user: &authentication::User, mut cookies: Cookies) -> Result<(), !> {
    let user_id = user.0.id;
    remove_login_cookie(&mut cookies);
    add_login_cookie(&mut cookies, user_id);
    return Ok(());
}

#[post("/refresh", rank = 2)]
pub fn refresh_session_loggedout(
    mut cookies: Cookies,
) -> rocket::response::status::Unauthorized<()> {
    remove_login_cookie(&mut cookies);
    rocket::response::status::Unauthorized(None)
}

fn add_login_cookie(cookies: &mut Cookies, user_id: i32) {
    cookies.add_private(Cookie::new("user_id", user_id.to_string()));
}

fn remove_login_cookie(cookies: &mut Cookies) {
    if let Some(cookie) = cookies.get_private("user_id") {
        cookies.remove_private(cookie);
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonUserLabelSets {
    pub name: String,
    pub id: i32,
    pub uuid: String,
}

impl From<crate::models::LabelSet> for JsonUserLabelSets {
    fn from(set: crate::models::LabelSet) -> Self {
        Self {
            id: set.id,
            name: set.name,
            uuid: set.uuid,
        }
    }
}

#[put("/labelsets/<uuid>")]
pub fn add_labelset(
    conn: MainDbConn,
    uuid: Uuid,
    user: &authentication::User,
) -> Result<Option<()>, Box<dyn Error>> {
    let set = schema::labelsets::dsl::labelsets
        .filter(schema::labelsets::dsl::uuid.eq(&uuid.to_string()))
        .limit(1)
        .load::<crate::models::LabelSet>(&*conn)?
        .pop();
    if set.is_none() {
        return Ok(None);
    }
    let set = set.unwrap();

    let data = UserLabelSet {
        userid: user.0.id,
        labelset: set.id,
    };

    rocket_contrib::databases::diesel::insert_into(schema::userlabelsets::table)
        .values(&data)
        .execute(&*conn)?;

    Ok(Some(()))
}

#[delete("/labelsets/<uuid>")]
pub fn delete_labelset(
    conn: MainDbConn,
    uuid: Uuid,
    user: &authentication::User,
) -> Result<Option<()>, Box<dyn Error>> {
    use schema::userlabelsets::dsl::{labelset, userid};

    let set = schema::labelsets::dsl::labelsets
        .filter(schema::labelsets::dsl::uuid.eq(&uuid.to_string()))
        .limit(1)
        .load::<crate::models::LabelSet>(&*conn)?
        .pop();
    if set.is_none() {
        return Ok(None);
    }
    let set = set.unwrap();

    let filter1 = labelset.eq(&set.id);
    let filter2 = userid.eq(&user.0.id);
    let deleted = rocket_contrib::databases::diesel::delete(schema::userlabelsets::table)
        .filter(filter1.and(filter2))
        .execute(&*conn)?;

    match deleted {
        0 => Ok(None),
        1 => Ok(Some(())),
        n => Err(format!("Expected 1 deleted userset, but deleted {}!", n).into()),
    }
}

#[get("/labelsets")]
pub fn get_labelsets(
    conn: MainDbConn,
    user: &authentication::User,
) -> Result<Json<Vec<JsonUserLabelSets>>, Box<dyn Error>> {
    let set_ids: Vec<_> = schema::userlabelsets::dsl::userlabelsets
        .filter(schema::userlabelsets::dsl::userid.eq(&user.0.id))
        .load::<crate::models::UserLabelSet>(&*conn)?
        .into_iter()
        .map(|uls| uls.labelset)
        .collect();

    let result: Vec<_> = schema::labelsets::dsl::labelsets
        .filter(schema::labelsets::dsl::id.eq_any(&set_ids))
        .load::<crate::models::LabelSet>(&*conn)?
        .into_iter()
        .map(From::from)
        .collect();

    Ok(Json(result))
}
