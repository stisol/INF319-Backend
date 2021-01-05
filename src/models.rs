use super::schema::*;
use rocket_contrib::databases::diesel::{Insertable, Queryable};

#[derive(Queryable, Clone)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: Vec<u8>,
    pub privilege: i32,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Privilege {
    User = 0,
    Administrator = 1,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a [u8],
}

#[derive(Queryable, Clone)]
pub struct Model {
    pub id: i32,
    pub filename: String,
}

#[derive(Queryable, Clone)]
pub struct LabelSet {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub model: i32,
}

#[derive(Insertable)]
#[table_name = "labelsets"]
pub struct NewLabelSet<'a> {
    pub uuid: &'a str,
    pub name: &'a str,
    pub model: i32,
}

#[derive(Queryable, Clone)]
pub struct Label {
    pub id: i32,
    pub labelset: i32,
    pub name: String,
    pub colour: String,
    pub vertices: Vec<u8>,
}

#[derive(Insertable)]
#[table_name = "labels"]
pub struct NewLabel<'a> {
    pub labelset: i32,
    pub name: &'a str,
    pub colour: &'a str,
    pub vertices: &'a [u8],
}