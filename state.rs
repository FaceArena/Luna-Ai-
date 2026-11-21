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
    pub lab: LabConfig,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LabConfig {
    pub personas: Vec<LabPersona>,
    pub presets: Vec<LabPreset>,
    pub lorebooks: Vec<LabLorebook>,
    pub active_preset: Option<String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LabPersona {
    pub id: String,
    pub title: String,
    pub name: String,
    pub who: String,
    pub rules: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LabPreset {
    pub id: String,
    pub name: String,
    pub len: String,
    pub instructions: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LabLorebook {
    pub id: String,
    pub name: String,
    pub keywords: Vec<String>,
    pub content: String,
    pub enabled: bool,
}
impl Default for Cfg {
    fn default() -> Self {
        Self {
            name: "Luna".into(),
            who: String::new(),
            rules: String::new(),
            len: "normal".into(),
            you: String::new(),
            theme: "dark".into(),
            motion: true,
            fs: "m".into(),
            keep: true,
            glass: 100,
            lab: LabConfig::default(),
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
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub plan: usize,
    #[serde(default)]
    pub channel_member: bool,
    #[serde(default = "default_prompt_limit")]
    pub prompt_limit: usize,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct PlanState {
    pub plan: usize,
    #[serde(default)]
    pub channel_member: Option<bool>,
    #[serde(default = "default_prompt_limit")]
    pub prompt_limit: usize,
}

fn default_prompt_limit() -> usize { 100 }

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub chat_id: Option<i64>,
    pub category: String,
    pub content: String,
    pub source: String,
    pub importance: i32,
    pub confidence: f64,
    pub is_pinned: bool,
    pub updated_at: String,
}


/// Строка в ленте. fx: 0 — обычный текст, 1 — «стоп», 2 — «лимит», 3 — «лимит промта».
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
    Memory,
    Lab,
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
    pub cfg_sync: RwSignal<bool>,
    pub pal: RwSignal<usize>,
    pub mi: RwSignal<usize>,
    pub model_preview: RwSignal<usize>,
    pub busy: RwSignal<bool>,
    pub plan_busy: RwSignal<bool>,
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
        // The active tariff is server-authoritative; never restore a premium plan from local storage.
        let mi = 0usize;
        Self {
            cfg: create_rw_signal(cfg),
            cfg_sync: create_rw_signal(false),
            pal: create_rw_signal(pal),
            mi: create_rw_signal(mi),
            model_preview: create_rw_signal(mi),
            busy: create_rw_signal(false),
            plan_busy: create_rw_signal(false),
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
            if c.theme == "auto" {
                self.cfg.update(|cfg| cfg.theme = "dark".into());
                return;
            }
            match c.theme.as_str() {
                // "auto" из старых сохранённых настроек мигрирует в тёмную тему.
                "auto" | "dark" => {
                    let _ = r.set_attribute("data-theme", "dark");
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
                        if self.cfg_sync.get_untracked() && self.me.get_untracked().is_some() {
                            let cfg = c.clone();
                            spawn_local(async move { let _ = api::put_personal_settings(&cfg).await; });
                        }
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

        // Re-check membership when the user returns to the app (for example,
        // after opening Telegram to leave or re-join the channel).
        let refresh_subscription = move || {
            if self.me.get_untracked().is_none() { return; }
            spawn_local(async move {
                if let Some(me) = api::me().await {
                    let plan = me.plan.min(2);
                    self.mi.set(plan);
                    self.model_preview.set(plan);
                    ls_set("luna-model2", &plan.to_string());
                    self.me.set(Some(me));
                }
            });
        };
        // iOS Safari/PWA may restore a page without firing focus consistently.
        // Reconcile the plan and channel lock state on both focus and pageshow.
        window_event_listener(ev::focus, move |_| refresh_subscription());
        window_event_listener(ev::pageshow, move |_| refresh_subscription());
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
        if i > 2 || self.mi.get_untracked() == i || self.plan_busy.get_untracked() {
            return;
        }
        let Some(_) = self.me.get_untracked() else {
            if i > 0 {
                self.say("Чтобы выбрать премиум-тариф, сначала войдите через Telegram.");
                self.open_sheet(Page::Acc);
            } else {
                self.mi.set(0);
                self.model_preview.set(0);
                ls_set("luna-model2", "0");
            }
            return;
        };

        // Optimistic UI: switch immediately, then reconcile with the server.
        // The plan endpoint returns the verified membership in the same request.
        let previous = self.mi.get_untracked();
        self.plan_busy.set(true);
        self.mi.set(i);
        self.model_preview.set(i);
        spawn_local(async move {
            match api::set_plan(i).await {
                Ok(result) => {
                    let verified_plan = result.plan.min(2);
                    self.mi.set(verified_plan);
                    self.model_preview.set(verified_plan);
                    ls_set("luna-model2", &verified_plan.to_string());
                    self.me.update(|current| {
                        if let Some(me) = current {
                            me.plan = verified_plan;
                            if let Some(channel_member) = result.channel_member {
                                me.channel_member = channel_member;
                            }
                            me.prompt_limit = result.prompt_limit;
                        }
                    });
                }
                Err(message) => {
                    self.mi.set(previous);
                    self.model_preview.set(previous);
                    ls_set("luna-model2", &previous.to_string());
                    let notice = if message.contains("403") || message.contains("подпишитесь") {
                        "Премиум доступен только подписчикам Telegram-канала Luna AI."
                    } else {
                        "Не удалось изменить тариф. Проверьте подключение и попробуйте снова."
                    };
                    self.say(notice);
                }
            }
            self.plan_busy.set(false);
        });
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
        self.model_preview.set(me.plan.min(2));
        ls_set("luna-model2", &self.mi.get_untracked().to_string());
        self.cfg_sync.set(false);
        self.me.set(Some(me.clone()));
        spawn_local(async move {
            match api::get_personal_settings().await {
                Ok(remote) => {
                    let is_new = remote.is_none();
                    let owner = ls_get("luna-cfg-owner");
                    let local = self.cfg.get_untracked();
                    let mut cfg = match remote {
                        Some(saved) => saved,
                        None if owner.as_deref().map_or(true, |id| id == me.id) => local,
                        None => Cfg::default(),
                    };
                    if cfg.you.is_empty() { cfg.you = me.name.clone(); }
                    self.cfg.set(cfg.clone());
                    ls_set("luna-cfg-owner", &me.id);
                    self.cfg_sync.set(true);
                    if is_new { let _ = api::put_personal_settings(&cfg).await; }
                }
                Err(_) => self.say("Не удалось загрузить персональные настройки. Они пока останутся на этом устройстве."),
            }
            self.chats.set(api::chats().await);
        });
    }

    pub fn logout(self) {
        spawn_local(api::logout());
        self.cfg_sync.set(false);
        self.mi.set(0);
        self.model_preview.set(0);
        ls_set("luna-model2", "0");
        self.me.set(None);
        self.cfg.set(Cfg::default());
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
        let recent_user_context = h.iter().rev()
            .filter(|message| message.role == "user")
            .take(3)
            .map(|message| message.content.to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");
        let mut matched_lorebooks = cfg.lab.lorebooks.iter()
            .filter(|book| book.enabled)
            .filter_map(|book| {
                let score = book.keywords.iter().filter(|keyword| {
                    let keyword = keyword.trim().to_lowercase();
                    !keyword.is_empty() && recent_user_context.contains(&keyword)
                }).count();
                (score > 0).then_some((score, book))
            })
            .collect::<Vec<_>>();
        matched_lorebooks.sort_by(|a, b| b.0.cmp(&a.0));
        let lore_context = matched_lorebooks.into_iter()
            .take(5)
            .map(|(_, book)| format!("[{}]\n{}", book.name, book.content))
            .collect::<Vec<_>>()
            .join("\n\n")
            .chars().take(8000).collect::<String>();
        let preset_context = cfg.lab.active_preset.as_ref()
            .and_then(|id| cfg.lab.presets.iter().find(|preset| &preset.id == id))
            .map(|preset| preset.instructions.chars().take(2000).collect::<String>())
            .unwrap_or_default();
        let body = serde_json::json!({
            "msgs": &h[skip..], "chat_id": self.cur.get_untracked(), "name": cfg.name, "who": cfg.who,
            "rules": cfg.rules, "len": cfg.len, "you": cfg.you, "keep": cfg.keep,
            "lore_context": lore_context, "preset_context": preset_context
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
                402 => return self.fail("Инструкции персонального ИИ превышают лимит бесплатной версии. Сократите их или посмотрите доступные тарифы.".to_string(), 3),
                429 => return self.fail(pick(&data().tpl_limit), 2),
                401 => {
                    self.me.set(None);
                    self.mi.set(0);
                    ls_set("luna-model2", "0");
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
            let mut buf: Vec<u8> = Vec::new();
            let mut shown = 0usize;
            const CHUNKS: [usize; 6] = [3, 4, 4, 5, 3, 4];

            // Start the typing phase one second after the reply bubble appears.
            gloo_timers::future::TimeoutFuture::new(450).await;
            if self.run.get_value() != my {
                return;
            }

            while let Some(chunk) = stream.next().await {
                if self.run.get_value() != my {
                    return;
                }
                let Ok(v) = chunk else { break };
                buf.extend(js_sys::Uint8Array::new(&v).to_vec());

                // Do not expose a replacement glyph when a UTF-8 character is split across network chunks.
                let decoded = match std::str::from_utf8(&buf) {
                    Ok(text) => text.to_string(),
                    Err(error) => String::from_utf8_lossy(&buf[..error.valid_up_to()]).into_owned(),
                };
                let full = clean(&decoded);
                let chars: Vec<char> = full.chars().collect();
                while shown < chars.len() {
                    if self.run.get_value() != my {
                        return;
                    }
                    shown = (shown + CHUNKS[shown % CHUNKS.len()]).min(chars.len());
                    let visible: String = chars[..shown].iter().collect();
                    self.stream.set(Some(visible));
                    self.scroll(false);
                    gloo_timers::future::TimeoutFuture::new(60).await;
                }
            }
            if self.run.get_value() != my {
                return;
            }
            let out = clean(String::from_utf8_lossy(&buf).trim());
            if out.is_empty() {
                return self.fail(ERR.into(), 0);
            }
            self.stream.set(Some(out.clone()));
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
        if !part.is_empty() {
            self.hist.update(|h| h.push(Msg { role: "assistant".into(), content: part.clone() }));
            self.push_ai("", part, 0);
            self.persist();
        } else {
            self.stream.set(None);
            self.busy.set(false);
        }
    }
}
