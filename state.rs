use crate::{api, data::data};
use futures_util::StreamExt;
use gloo_net::http::Request;
use leptos::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wasm_bindgen::JsCast;

// ───────────── типы ─────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cfg {
    pub name: String,
    pub who: String,
    pub rules: String,
    pub len: String,
    pub you: String,
    pub theme: String,
    pub motion: bool,
    pub fs: String,
    pub keep: bool,
    pub glass: u32,
}
impl Default for Cfg {
    fn default() -> Self {
        Self {
            name: "Luna".into(),
            who: String::new(),
            rules: String::new(),
            len: "normal".into(),
            you: String::new(),
            theme: "auto".into(),
            motion: true,
            fs: "m".into(),
            keep: true,
            glass: 100,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Msg {
    pub role: String,
    pub content: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub msgs: Vec<Msg>,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct Me {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub plan: usize,
}

/// Строка в ленте. fx: 0 — обычный текст, 1 — «стоп», 2 — «лимит».
#[derive(Clone, PartialEq)]
pub struct Row {
    pub id: u64,
    pub user: bool,
    pub part: String,
    pub text: String,
    pub fx: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Main,
    Ai,
    Acc,
    Plan,
    About,
    Doc,
}

// ───────────── хранилище ─────────────

pub fn ls_get(k: &str) -> Option<String> {
    window().local_storage().ok().flatten()?.get_item(k).ok().flatten()
}
pub fn ls_set(k: &str, v: &str) {
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.set_item(k, v);
    }
}

pub fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| {
            let u = *c as u32;
            !((0x1F000..=0x1FAFF).contains(&u)
                || (0x2600..=0x27BF).contains(&u)
                || (0x2B00..=0x2BFF).contains(&u)
                || u == 0xFE0F
                || u == 0x200D)
        })
        .collect()
}

fn pick(a: &[String]) -> String {
    let i = (js_sys::Math::random() * a.len() as f64) as usize;
    a[i.min(a.len() - 1)].clone()
}

// ───────────── контекст ─────────────

#[derive(Clone, Copy)]
pub struct Ctx {
    pub cfg: RwSignal<Cfg>,
    pub pal: RwSignal<usize>,
    pub mi: RwSignal<usize>,
    pub busy: RwSignal<bool>,
    pub me: RwSignal<Option<Me>>,
    pub chats: RwSignal<Vec<Chat>>,
    pub cur: RwSignal<Option<i64>>,
    pub hist: RwSignal<Vec<Msg>>,
    pub log: RwSignal<Vec<Row>>,
    pub stream: RwSignal<Option<String>>,
    pub pal_open: RwSignal<bool>,
    pub msh_open: RwSignal<bool>,
    pub drawer: RwSignal<bool>,
    pub sheet: RwSignal<bool>,
    pub page: RwSignal<Page>,
    pub doc: RwSignal<String>,
    pub doc_from: RwSignal<Page>,
    pub toast: RwSignal<String>,
    pub toast_on: RwSignal<bool>,
    pub log_el: NodeRef<html::Main>,
    uid: StoredValue<u64>,
    run: StoredValue<u64>,
    abort: StoredValue<Option<web_sys::AbortController>>,
}

impl Ctx {
    pub fn new() -> Self {
        let cfg: Cfg = ls_get("luna-cfg").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let pal = ls_get("luna-pal3").and_then(|s| s.parse().ok()).unwrap_or(0usize).min(data().pal.len() - 1);
        let mi = ls_get("luna-model2").and_then(|s| s.parse().ok()).unwrap_or(0usize).min(2);
        Self {
            cfg: create_rw_signal(cfg),
            pal: create_rw_signal(pal),
            mi: create_rw_signal(mi),
            busy: create_rw_signal(false),
            me: create_rw_signal(None),
            chats: create_rw_signal(vec![]),
            cur: create_rw_signal(None),
            hist: create_rw_signal(vec![]),
            log: create_rw_signal(vec![]),
            stream: create_rw_signal(None),
            pal_open: create_rw_signal(false),
            msh_open: create_rw_signal(false),
            drawer: create_rw_signal(false),
            sheet: create_rw_signal(false),
            page: create_rw_signal(Page::Main),
            doc: create_rw_signal(String::new()),
            doc_from: create_rw_signal(Page::Main),
            toast: create_rw_signal(String::new()),
            toast_on: create_rw_signal(false),
            log_el: create_node_ref(),
            uid: store_value(0),
            run: store_value(0),
            abort: store_value(None),
        }
    }

    /// Эффекты: тема, палитра, сохранение настроек, загрузка сессии.
    pub fn wire(self) {
        let root: web_sys::HtmlElement = document().document_element().unwrap().unchecked_into();

        let r = root.clone();
        create_effect(move |_| {
            let c = self.cfg.get();
            match c.theme.as_str() {
                "auto" => {
                    let _ = r.remove_attribute("data-theme");
                }
                t => {
                    let _ = r.set_attribute("data-theme", t);
                }
            }
            let _ = r.class_list().toggle_with_force("calm", !c.motion);
            let st = r.style();
            let _ = st.set_property("--gl", &format!("{}", c.glass as f64 / 100.0));
            let fs = match c.fs.as_str() {
                "s" => "14px",
                "l" => "18.5px",
                _ => "16px",
            };
            let _ = st.set_property("--fs", fs);
            document().set_title(if c.name.is_empty() { "Luna" } else { &c.name });
        });

        let r = root.clone();
        create_effect(move |_| {
            let p = &data().pal[self.pal.get()];
            let st = r.style();
            let _ = st.set_property("--a", &p.a);
            let _ = st.set_property("--b", &p.b);
            let _ = st.set_property("--c", &p.c);
        });

        // сохранение настроек с задержкой (не на каждую букву)
        let gen = store_value(0u32);
        create_effect(move |prev: Option<()>| {
            let c = self.cfg.get();
            if prev.is_none() {
                return;
            }
            gen.update_value(|x| *x += 1);
            let me = gen.get_value();
            set_timeout(
                move || {
                    if gen.get_value() == me {
                        ls_set("luna-cfg", &serde_json::to_string(&c).unwrap_or_default());
                    }
                },
                Duration::from_millis(300),
            );
        });
        window_event_listener(ev::pagehide, move |_| {
            ls_set("luna-cfg", &serde_json::to_string(&self.cfg.get_untracked()).unwrap_or_default());
        });

        spawn_local(async move {
            if let Some(me) = api::me().await {
                self.finish_login(me);
            }
        });
    }

    pub fn next_uid(self) -> u64 {
        let v = self.uid.get_value() + 1;
        self.uid.set_value(v);
        v
    }

    pub fn say(self, t: &str) {
        self.toast.set(t.into());
        self.toast_on.set(true);
        set_timeout(move || self.toast_on.set(false), Duration::from_millis(2300));
    }

    pub fn scroll(self, smooth: bool) {
        request_animation_frame(move || {
            if let Some(el) = self.log_el.get_untracked() {
                let o = web_sys::ScrollToOptions::new();
                o.set_top(el.scroll_height() as f64);
                o.set_behavior(if smooth { web_sys::ScrollBehavior::Smooth } else { web_sys::ScrollBehavior::Auto });
                el.scroll_to_with_scroll_to_options(&o);
            }
        });
    }

    pub fn set_pal(self, i: usize) {
        self.pal.set(i);
        ls_set("luna-pal3", &i.to_string());
    }

    pub fn set_model(self, i: usize) {
        self.mi.set(i);
        ls_set("luna-model2", &i.to_string());
        if self.me.get_untracked().is_some() {
            self.me.update(|m| {
                if let Some(m) = m {
                    m.plan = i;
                }
            });
            spawn_local(api::set_plan(i));
        }
    }

    pub fn open_sheet(self, page: Page) {
        self.drawer.set(false);
        self.pal_open.set(false);
        self.msh_open.set(false);
        self.open_page(page);
    }

    pub fn open_page(self, page: Page) {
        self.drawer.set(false);
        self.page.set(page);
        self.sheet.set(true);
    }

    pub fn close_all(self) {
        self.drawer.set(false);
        self.sheet.set(false);
    }

    pub fn open_doc(self, key: &str, from: Page) {
        self.doc.set(key.into());
        self.doc_from.set(from);
        self.page.set(Page::Doc);
    }

    // ───── аккаунт ─────

    pub fn finish_login(self, me: Me) {
        self.mi.set(me.plan.min(2));
        ls_set("luna-model2", &self.mi.get_untracked().to_string());
        if self.cfg.get_untracked().you.is_empty() {
            let n = me.name.clone();
            self.cfg.update(|c| c.you = n);
        }
        self.me.set(Some(me));
        spawn_local(async move {
            self.chats.set(api::chats().await);
        });
    }

    pub fn logout(self) {
        spawn_local(api::logout());
        self.me.set(None);
        self.chats.set(vec![]);
        self.reset_view();
    }

    // ───── чаты ─────

    fn reset_view(self) {
        self.cur.set(None);
        self.hist.set(vec![]);
        self.log.set(vec![]);
        self.stream.set(None);
    }

    pub fn new_chat(self) {
        if self.busy.get_untracked() {
            return;
        }
        self.reset_view();
        self.drawer.set(false);
    }

    pub fn open_chat(self, id: i64) {
        if self.busy.get_untracked() {
            return;
        }
        let Some(c) = self.chats.get_untracked().into_iter().find(|c| c.id == id) else { return };
        self.cur.set(Some(id));
        let rows: Vec<Row> = c
            .msgs
            .iter()
            .map(|m| Row { id: self.next_uid(), user: m.role == "user", part: String::new(), text: m.content.clone(), fx: 0 })
            .collect();
        self.hist.set(c.msgs);
        self.log.set(rows);
        self.drawer.set(false);
        self.scroll(false);
    }

    pub fn del_chat(self, id: i64) {
        self.chats.update(|c| c.retain(|x| x.id != id));
        spawn_local(api::del_chat(id));
        if self.cur.get_untracked() == Some(id) && !self.busy.get_untracked() {
            self.reset_view();
        }
    }

    pub fn clear_hist(self) {
        self.chats.set(vec![]);
        if self.me.get_untracked().is_some() {
            spawn_local(api::del_all());
        }
        if !self.busy.get_untracked() {
            self.reset_view();
        }
    }

    pub fn export_chats(self) {
        let json = serde_json::to_string_pretty(&self.chats.get_untracked()).unwrap_or_default();
        let url = format!("data:application/json;charset=utf-8,{}", js_sys::encode_uri_component(&json));
        if let Ok(el) = document().create_element("a") {
            let a: web_sys::HtmlAnchorElement = el.unchecked_into();
            a.set_href(&url);
            a.set_download("luna-chats.json");
            a.click();
        }
    }

    fn persist(self) {
        let Some(id) = self.cur.get_untracked() else { return };
        if !self.cfg.get_untracked().keep || self.me.get_untracked().is_none() {
            return;
        }
        let h = self.hist.get_untracked();
        if h.is_empty() {
            self.chats.update(|c| c.retain(|x| x.id != id));
            self.cur.set(None);
            spawn_local(api::del_chat(id));
            return;
        }
        let chat = Chat { id, title: h[0].content.chars().take(40).collect(), msgs: h };
        self.chats.update(|c| {
            c.retain(|x| x.id != id);
            c.insert(0, chat.clone());
            c.truncate(50);
        });
        spawn_local(async move { api::put_chat(&chat).await });
    }

    // ───── отправка и поток ─────

    fn push_ai(self, part: &str, text: String, fx: u8) {
        let id = self.next_uid();
        self.log.update(|l| l.push(Row { id, user: false, part: part.into(), text, fx }));
        self.stream.set(None);
        self.busy.set(false);
        self.scroll(true);
    }

    /// Ошибка или лимит: ответ не получен, сообщение пользователя убираем из истории.
    fn fail(self, text: String, fx: u8) {
        self.hist.update(|h| {
            h.pop();
        });
        self.push_ai("", text, fx);
        self.persist();
    }

    pub fn send(self, text: String) {
        let t = text.trim().to_string();
        if t.is_empty() || self.busy.get_untracked() {
            return;
        }
        if self.me.get_untracked().is_none() {
            self.say("Войдите в аккаунт, чтобы писать");
            self.open_sheet(Page::Acc);
            return;
        }
        if self.cur.get_untracked().is_none() {
            self.cur.set(Some(js_sys::Date::now() as i64));
        }
        let rid = self.next_uid();
        self.log.update(|l| l.push(Row { id: rid, user: true, part: String::new(), text: t.clone(), fx: 0 }));
        self.hist.update(|h| h.push(Msg { role: "user".into(), content: t }));
        self.busy.set(true);
        self.stream.set(Some(String::new()));
        self.scroll(true);

        let my = self.run.get_value() + 1;
        self.run.set_value(my);
        let ac = web_sys::AbortController::new().ok();
        let sig = ac.as_ref().map(|a| a.signal());
        self.abort.set_value(ac);

        let cfg = self.cfg.get_untracked();
        let h = self.hist.get_untracked();
        let skip = h.len().saturating_sub(20);
        let body = serde_json::json!({
            "msgs": &h[skip..], "name": cfg.name, "who": cfg.who,
            "rules": cfg.rules, "len": cfg.len, "you": cfg.you
        });
        const ERR: &str = "Не удалось получить ответ. Отправьте сообщение ещё раз.";

        spawn_local(async move {
            let mut rb = Request::post("/api/chat");
            if let Some(s) = sig.as_ref() {
                rb = rb.abort_signal(Some(s));
            }
            let sent = match rb.json(&body) {
                Ok(r) => r.send().await,
                Err(_) => {
                    self.fail(ERR.into(), 0);
                    return;
                }
            };
            if self.run.get_value() != my {
                return;
            }
            let resp = match sent {
                Ok(r) => r,
                Err(_) => {
                    self.fail(ERR.into(), 0);
                    return;
                }
            };
            match resp.status() {
                429 => return self.fail(pick(&data().tpl_limit), 2),
                401 => {
                    self.me.set(None);
                    self.fail("Сессия закончилась. Войдите в аккаунт ещё раз.".into(), 0);
                    self.open_sheet(Page::Acc);
                    return;
                }
                _ => {}
            }
            let Some(raw) = resp.body().filter(|_| resp.ok()) else {
                return self.fail(ERR.into(), 0);
            };
            let mut stream = wasm_streams::ReadableStream::from_raw(raw.unchecked_into()).into_stream();
            let mut pending: Vec<u8> = Vec::new();
            let mut answer = String::new();
            let mut finished = false;
            while let Some(chunk) = stream.next().await {
                if self.run.get_value() != my {
                    return;
                }
                let Ok(v) = chunk else { break };
                pending.extend(js_sys::Uint8Array::new(&v).to_vec());
                while let Some(pos) = pending.iter().position(|b| *b == b'\\n') {
                    let mut line: Vec<u8> = pending.drain(..=pos).collect();
                    while matches!(line.last(), Some(b'\\n' | b'\\r')) { line.pop(); }
                    let line = String::from_utf8_lossy(&line);
                    let Some(data) = line.strip_prefix("data:") else { continue };
                    let data = data.trim();
                    if data == "[DONE]" {
                        finished = true;
                        break;
                    }
                    let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else { continue };
                    if let Some(piece) = event["choices"][0]["delta"]["content"].as_str() {
                        answer.push_str(piece);
                        self.stream.set(Some(clean(&answer)));
                        self.scroll(false);
                    }
                }
                if finished { break; }
            }
            if self.run.get_value() != my {
                return;
            }
            let out = clean(answer.trim());
            if out.is_empty() {
                return self.fail(ERR.into(), 0);
            }
            self.hist.update(|h| h.push(Msg { role: "assistant".into(), content: out.clone() }));
            self.push_ai("", out, 0);
            self.persist();
        });
    }

    pub fn stop(self) {
        if !self.busy.get_untracked() {
            return;
        }
        self.run.update_value(|r| *r += 1);
        if let Some(a) = self.abort.get_value() {
            a.abort();
        }
        let part = self.stream.get_untracked().unwrap_or_default().trim().to_string();
        let w = pick(&data().tpl_stop);
        let content = if part.is_empty() { w.clone() } else { format!("{part}\n\n{w}") };
        self.hist.update(|h| h.push(Msg { role: "assistant".into(), content }));
        self.push_ai(&part, w, 1);
        self.persist();
    }
}
