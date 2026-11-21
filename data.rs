//! Статические данные (кролик, палитры, тарифы, документы), сгенерированные из исходной Luna.html.
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Deserialize)]
pub struct Pal {
    pub n: String,
    pub a: String,
    pub b: String,
    pub c: String,
}
#[derive(Deserialize)]
pub struct Model {
    pub n: String,
    pub d: String,
    pub pro: bool,
}
#[derive(Deserialize)]
pub struct Pre {
    pub n: String,
    pub who: String,
    pub rules: String,
}
#[derive(Deserialize)]
pub struct Doc {
    pub h: String,
    pub t: String,
    pub s: Vec<(String, String)>,
}
#[derive(Deserialize)]
pub struct Data {
    pub rb: Vec<String>,
    pub rb_stop: String,
    pub rb_lim: String,
    pub pal: Vec<Pal>,
    pub models: Vec<Model>,
    pub feat: Vec<Vec<String>>,
    pub pre: Vec<Pre>,
    pub tpl_stop: Vec<String>,
    pub tpl_limit: Vec<String>,
    pub icons: HashMap<String, String>,
    pub lg_date: String,
    pub docs: HashMap<String, Doc>,
}

pub fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("data.json")).expect("data.json"))
}
