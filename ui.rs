use crate::{api, data::data, state::*};
use leptos::*;
use std::time::Duration;
use wasm_bindgen::{closure::Closure, JsCast};

const CHEV: &str = r#"<path d="m9 6 6 6-6 6"/>"#;
const UD: &str = r#"<path d="m8 9 4-4 4 4"/><path d="m8 15 4 4 4-4"/>"#;
const LOCK: &str = r#"<rect x="5" y="11" width="14" height="9" rx="2.5"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>"#;
const XI: &str = r#"<path d="M6 6l12 12"/><path d="M18 6 6 18"/>"#;
const BACK: &str = r#"<path d="m15 6-6 6 6 6"/>"#;
const PLUS: &str = r#"<path d="M12 5v14"/><path d="M5 12h14"/>"#;
const SEARCH: &str = r#"<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>"#;
const GEAR: &str = r#"<path d="M4 7h10M18 7h2M4 17h2M10 17h10"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/>"#;
const MENU: &str = r#"<path d="M4 7h16"/><path d="M4 12h16"/><path d="M4 17h10"/>"#;
const DOWN: &str = r#"<path d="m6 9 6 6 6-6"/>"#;
const SEND: &str = r#"<path d="M12 19V5"/><path d="m5 12 7-7 7 7"/>"#;
const STOP: &str = r#"<rect x="7" y="7" width="10" height="10" rx="2.5"/>"#;

fn ic(name: &str) -> impl IntoView {
    let p = data().icons.get(name).cloned().unwrap_or_default();
    view! { <svg viewBox="0 0 24 24" inner_html=p></svg> }
}

fn rb(k: usize) -> impl IntoView {
    view! { <svg class="rb" viewBox="0 0 100 100" inner_html=data().rb[k].clone()></svg> }
}

fn name(ctx: Ctx) -> String {
    let n = ctx.cfg.get().name;
    if n.is_empty() {
        "Luna".into()
    } else {
        n
    }
}

fn model_name(ctx: Ctx) -> String {
    data().models[ctx.mi.get()].n.clone()
}

fn coarse() -> bool {
    window().match_media("(pointer:coarse)").ok().flatten().map_or(false, |m| m.matches())
}

// ───────────── корень ─────────────

#[component]
pub fn App() -> impl IntoView {
    let ctx = Ctx::new();
    provide_context(ctx);
    ctx.wire();
    globals(ctx);
    view! {
        <Header/>
        <PalPop/>
        <ModelPop/>
        <div class="scrim" class:on=move || ctx.drawer.get() || ctx.sheet.get() on:click=move |_| ctx.close_all()></div>
        <div class="glass toast" class:on=move || ctx.toast_on.get() role="status">{move || ctx.toast.get()}</div>
        <Drawer/>
        <Sheet/>
        <Log/>
        <Composer/>
    }
}

/// Глобальные слушатели: параллакс фона, подсветка стекла, закрытие панелей, Esc, клавиатура iOS.
fn globals(ctx: Ctx) {
    let root: web_sys::HtmlElement = document().document_element().unwrap().unchecked_into();
    let gyro = store_value(false);
    let flag = store_value(false);

    let set_px = move |x: f64, y: f64| {
        if let Ok(Some(bg)) = document().query_selector(".bg") {
            let bg: web_sys::HtmlElement = bg.unchecked_into();
            let st = bg.style();
            let _ = st.set_property("--px", &format!("{:.3}", x.clamp(-1.0, 1.0)));
            let _ = st.set_property("--py", &format!("{:.3}", y.clamp(-1.0, 1.0)));
        }
    };

    window_event_listener(ev::pointermove, move |e: web_sys::PointerEvent| {
        if flag.get_value() {
            return;
        }
        flag.set_value(true);
        request_animation_frame(move || {
            flag.set_value(false);
            if ctx.cfg.get_untracked().motion && !gyro.get_value() {
                let w = window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1.0).max(1.0);
                let h = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1.0).max(1.0);
                set_px(e.client_x() as f64 / w * 2.0 - 1.0, e.client_y() as f64 / h * 2.0 - 1.0);
            }
            if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                if let Ok(Some(g)) = t.closest(".glass") {
                    let r = g.get_bounding_client_rect();
                    let g: web_sys::HtmlElement = g.unchecked_into();
                    let st = g.style();
                    let _ = st.set_property("--mx", &format!("{}px", e.client_x() as f64 - r.left()));
                    let _ = st.set_property("--my", &format!("{}px", e.client_y() as f64 - r.top()));
                }
            }
        });
    });

    window_event_listener(ev::deviceorientation, move |e: web_sys::DeviceOrientationEvent| {
        if let (Some(g), Some(b)) = (e.gamma(), e.beta()) {
            gyro.set_value(true);
            if ctx.cfg.get_untracked().motion {
                set_px(g / 25.0, (b - 50.0) / 25.0);
            }
        }
    });

    window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            if t.closest(".pal,#palb").ok().flatten().is_none() {
                ctx.pal_open.set(false);
            }
            if t.closest(".msh,#mdlb").ok().flatten().is_none() {
                ctx.msh_open.set(false);
            }
        }
    });

    window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            ctx.close_all();
        }
    });

    let r = root.clone();
    window_event_listener(ev::visibilitychange, move |_| {
        let _ = r.class_list().toggle_with_force("paused", document().hidden());
    });

    // клавиатура на iOS: высота = видимая область
    if let Some(vv) = window().visual_viewport() {
        let v2 = vv.clone();
        let fit = Closure::<dyn FnMut()>::new(move || {
            let h = format!("{}px", v2.height());
            let st = root.style();
            if st.get_property_value("height").unwrap_or_default() != h {
                let _ = st.set_property("height", &h);
            }
            if window().scroll_y().unwrap_or(0.0) != 0.0 {
                window().scroll_to_with_x_and_y(0.0, 0.0);
            }
            ctx.scroll(false);
        });
        let _ = vv.add_event_listener_with_callback("resize", fit.as_ref().unchecked_ref());
        let _ = vv.add_event_listener_with_callback("scroll", fit.as_ref().unchecked_ref());
        fit.forget();
    }
}

// ───────────── шапка и всплывающие панели ─────────────

#[component]
fn Header() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    view! {
        <header id="hd">
            <button class="glass ib" aria-label="История чатов" on:click=move |_| { ctx.drawer.set(true); ctx.pal_open.set(false); ctx.msh_open.set(false) }>
                <svg viewBox="0 0 24 24" inner_html=MENU></svg>
            </button>
            <button class="title mdl" id="mdlb" aria-label="Выбор модели" on:click=move |_| { ctx.pal_open.set(false); ctx.msh_open.update(|v| *v = !*v) }>
                <b><em>{move || name(ctx)}</em>" "<svg viewBox="0 0 24 24" inner_html=DOWN></svg></b>
                <span>{move || if ctx.busy.get() { "Печатает".to_string() } else { model_name(ctx) }}</span>
            </button>
            <button class="glass chg" aria-label="Сменить тариф" on:click=move |_| ctx.open_sheet(Page::Plan)>"Сменить"</button>
            <button class="glass ib" id="palb" aria-label="Цвета" on:click=move |_| { ctx.msh_open.set(false); ctx.pal_open.update(|v| *v = !*v) }>
                <span class="orb"></span>
            </button>
        </header>
    }
}

#[component]
fn PalPop() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let pal_ref = create_node_ref::<html::Div>();
    let held = create_rw_signal(false);
    let lx = create_rw_signal(6.0f64);
    let n = data().pal.len();

    let mv = move |e: web_sys::PointerEvent| {
        if let Some(el) = pal_ref.get_untracked() {
            let x = e.client_x() as f64 - el.get_bounding_client_rect().left();
            lx.set((x - 22.0).clamp(6.0, 6.0 + (n - 1) as f64 * 50.0));
            let i = (((x - 6.0) / 50.0).floor().max(0.0) as usize).min(n - 1);
            if i != ctx.pal.get_untracked() {
                ctx.set_pal(i);
            }
        }
    };
    let rel = move |_: web_sys::PointerEvent| held.set(false);

    view! {
        <div class="glass pal" node_ref=pal_ref class:on=move || ctx.pal_open.get() class:held=move || held.get()
             role="radiogroup" aria-label="Цветовая палитра"
             on:pointerdown=move |e: web_sys::PointerEvent| {
                 if let Some(el) = pal_ref.get_untracked() { let _ = el.set_pointer_capture(e.pointer_id()); }
                 held.set(true);
                 mv(e);
             }
             on:pointermove=move |e: web_sys::PointerEvent| { if held.get_untracked() { mv(e) } }
             on:pointerup=rel on:pointercancel=rel>
            {data().pal.iter().enumerate().map(|(i, p)| view! {
                <div class="sw" class:sel=move || ctx.pal.get() == i role="radio"
                     style=format!("--s1:{};--s2:{};--s3:{}", p.a, p.b, p.c)></div>
            }).collect_view()}
            <div class="lens" style:left=move || format!("{}px", if held.get() { lx.get() } else { 6.0 + ctx.pal.get() as f64 * 50.0 })></div>
            <div class="pname">{move || data().pal[ctx.pal.get()].n.clone()}</div>
        </div>
    }
}

#[component]
fn ModelPop() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let msh_ref = create_node_ref::<html::Div>();
    let held = create_rw_signal(false);
    let ly = create_rw_signal(6.0f64);

    let mv = move |e: web_sys::PointerEvent| {
        if let Some(el) = msh_ref.get_untracked() {
            let y = e.client_y() as f64 - el.get_bounding_client_rect().top();
            ly.set((y - 29.0).clamp(6.0, 6.0 + 2.0 * 62.0));
            let i = (((y - 6.0) / 62.0).floor().max(0.0) as usize).min(2);
            if i != ctx.mi.get_untracked() {
                ctx.set_model(i);
            }
        }
    };
    let rel = move |_: web_sys::PointerEvent| {
        if held.get_untracked() {
            held.set(false);
            set_timeout(move || ctx.msh_open.set(false), Duration::from_millis(480));
        }
    };

    view! {
        <div class="glass msh" id="msh" node_ref=msh_ref class:on=move || ctx.msh_open.get() class:held=move || held.get()
             role="radiogroup" aria-label="Модель"
             on:pointerdown=move |e: web_sys::PointerEvent| {
                 if let Some(el) = msh_ref.get_untracked() { let _ = el.set_pointer_capture(e.pointer_id()); }
                 held.set(true);
                 mv(e);
             }
             on:pointermove=move |e: web_sys::PointerEvent| { if held.get_untracked() { mv(e) } }
             on:pointerup=rel on:pointercancel=rel>
            {data().models.iter().enumerate().map(|(k, m)| view! {
                <div class="mc" class:pro=m.pro class:sel=move || ctx.mi.get() == k>
                    <div class="glass ic">{rb(k)}</div>
                    <div class="mt"><b>{m.n.clone()}</b><span>{m.d.clone()}</span></div>
                    {if m.pro {
                        view! { <svg class="lk" viewBox="0 0 24 24" inner_html=LOCK></svg> }.into_view()
                    } else {
                        view! { <em>"авто"</em> }.into_view()
                    }}
                </div>
            }).collect_view()}
            <div class="mlens" style:top=move || format!("{}px", if held.get() { ly.get() } else { 6.0 + ctx.mi.get() as f64 * 62.0 })></div>
        </div>
    }
}

// ───────────── меню истории ─────────────

#[component]
fn Drawer() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let q = create_rw_signal(String::new());
    let list = move || {
        let q = q.get().trim().to_lowercase();
        ctx.chats.get().into_iter().filter(|c| q.is_empty() || c.title.to_lowercase().contains(&q)).collect::<Vec<_>>()
    };
    view! {
        <aside class="glass drawer" class:on=move || ctx.drawer.get() aria-label="История чатов">
            <div class="dh">
                <div class="glass av2">{move || rb(ctx.mi.get())}</div>
                <b>{move || name(ctx)}</b>
                <button class="glass ib" aria-label="Закрыть" on:click=move |_| ctx.drawer.set(false)><svg viewBox="0 0 24 24" inner_html=XI></svg></button>
            </div>
            <button class="glass nc" on:click=move |_| ctx.new_chat()><svg viewBox="0 0 24 24" inner_html=PLUS></svg>"Новый чат"</button>
            <div class="glass srch">
                <svg viewBox="0 0 24 24" inner_html=SEARCH></svg>
                <input placeholder="Поиск по чатам" autocomplete="off" prop:value=move || q.get() on:input=move |e| q.set(event_target_value(&e))/>
            </div>
            <div class="hl">
                <small>{move || {
                    if list().is_empty() {
                        if q.get().trim().is_empty() { "История пока пуста" } else { "Ничего не найдено" }
                    } else { "Чаты" }
                }}</small>
                {move || (!ctx.cfg.get().keep).then(|| view! { <small>"Сохранение истории выключено"</small> })}
                <For each=list key=|c| c.id children=move |c| {
                    let id = c.id;
                    view! {
                        <div class="hi" class:cur=move || ctx.cur.get() == Some(id) on:click=move |_| ctx.open_chat(id)>
                            <span>{c.title.clone()}</span>
                            <button aria-label="Удалить" on:click=move |e| { e.stop_propagation(); ctx.del_chat(id) }>
                                <svg viewBox="0 0 24 24" inner_html=XI></svg>
                            </button>
                        </div>
                    }
                }/>
            </div>
            <button class="glass nc" on:click=move |_| ctx.open_sheet(Page::Main)><svg viewBox="0 0 24 24" inner_html=GEAR></svg>"Настройки"</button>
        </aside>
    }
}

// ───────────── настройки ─────────────

fn pgc(ctx: Ctx, p: Page) -> impl Fn() -> &'static str + Copy {
    move || {
        if ctx.page.get() == p {
            "pg"
        } else if p == Page::Main {
            "pg l"
        } else {
            "pg r"
        }
    }
}

fn ph(ctx: Ctx, title: impl Fn() -> String + 'static, back: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div class="ph">
            <button class="glass ib" aria-label="Назад" on:click=move |_| back()><svg viewBox="0 0 24 24" inner_html=BACK></svg></button>
            <b>{title}</b>
            <button class="glass ib" aria-label="Закрыть" on:click=move |_| ctx.close_all()><svg viewBox="0 0 24 24" inner_html=XI></svg></button>
        </div>
    }
}

fn nav_row(icn: &'static str, t: impl Fn() -> String + 'static, v: impl Fn() -> String + 'static, f: impl Fn() + 'static) -> impl IntoView {
    view! {
        <button type="button" class="row navrow" on:click=move |_| f()>
            {ic(icn)}<span class="rt">{t}</span><span class="rv">{v}</span>
            <svg class="ch" viewBox="0 0 24 24" inner_html=CHEV></svg>
        </button>
    }
}

fn doc_row(ctx: Ctx, icn: Option<&'static str>, t: &'static str, key: &'static str, from: Page) -> impl IntoView {
    view! {
        <button type="button" class="row navrow" on:click=move |_| ctx.open_doc(key, from)>
            {icn.map(ic)}<span class="rt">{t}</span>
            <svg class="ch" viewBox="0 0 24 24" inner_html=CHEV></svg>
        </button>
    }
}

fn sel_row(
    icn: &'static str,
    t: &'static str,
    opts: Vec<(String, String)>,
    val: impl Fn() -> String + Copy + 'static,
    set: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let o2 = opts.clone();
    let label = move || o2.iter().find(|(k, _)| *k == val()).map(|(_, l)| l.clone()).unwrap_or_default();
    view! {
        <div class="row">
            {ic(icn)}<span class="rt">{t}</span><span class="rv">{label}</span>
            <svg class="ch" viewBox="0 0 24 24" inner_html=UD></svg>
            <select on:change=move |e| set(event_target_value(&e))>
                {opts.into_iter().map(|(k, l)| {
                    let k2 = k.clone();
                    view! { <option value=k prop:selected=move || val() == k2>{l}</option> }
                }).collect_view()}
            </select>
        </div>
    }
}

fn tog_row(icn: &'static str, t: &'static str, get: impl Fn() -> bool + Copy + 'static, toggle: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div class="row" on:click=move |_| toggle()>
            {ic(icn)}<span class="rt">{t}</span><span class="tg" class:on=get></span>
        </div>
    }
}

fn act_row(icn: &'static str, t: &'static str, danger: bool, armed: RwSignal<Option<&'static str>>, f: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div class="row" class:danger=danger on:click=move |_| {
            if danger && armed.get_untracked() != Some(t) {
                armed.set(Some(t));
                set_timeout(move || { if armed.get_untracked() == Some(t) { armed.set(None) } }, Duration::from_secs(3));
                return;
            }
            armed.set(None);
            f();
        }>
            {ic(icn)}
            <span class="rt">{move || if armed.get() == Some(t) { "Нажмите ещё раз".to_string() } else { t.to_string() }}</span>
        </div>
    }
}

#[component]
fn Sheet() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    view! {
        <div class="glass sheet" class:on=move || ctx.sheet.get() role="dialog" aria-label="Настройки">
            <div class="grab"></div>
            <div class="pgs">
                <PageMain/><PageAi/><PageAcc/><PagePlan/><PageAbout/><PageDoc/>
            </div>
        </div>
    }
}

#[component]
fn PageMain() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let armed = create_rw_signal(None::<&'static str>);
    let pal_opts: Vec<(String, String)> = data().pal.iter().enumerate().map(|(k, p)| (k.to_string(), p.n.clone())).collect();
    let mod_opts: Vec<(String, String)> = data()
        .models
        .iter()
        .enumerate()
        .map(|(k, m)| (k.to_string(), format!("{}{}", m.n, if m.pro { " (платная)" } else { "" })))
        .collect();
    let theme_opts: Vec<(String, String)> =
        [("auto", "По умолчанию"), ("dark", "Тёмная"), ("light", "Светлая")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let fs_opts: Vec<(String, String)> =
        [("s", "Мелкий"), ("m", "Обычный"), ("l", "Крупный")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();

    view! {
        <section class=pgc(ctx, Page::Main)>
            <div class="ph">
                <i></i><b>"Настройки"</b>
                <button class="glass ib" aria-label="Закрыть" on:click=move |_| ctx.close_all()><svg viewBox="0 0 24 24" inner_html=XI></svg></button>
            </div>
            <div class="sc">
                <div class="prof">
                    {move || rb(ctx.mi.get())}
                    <div>
                        <b>{move || name(ctx)}</b>
                        <span>{move || format!("{} · {}", model_name(ctx), if data().models[ctx.mi.get()].pro { "платная версия" } else { "бесплатный тариф" })}</span>
                    </div>
                </div>

                <h3>"Аккаунт"</h3>
                <div class="grp">
                    {nav_row("user",
                        move || ctx.me.get().map(|m| m.name).unwrap_or_else(|| "Войти в аккаунт".into()),
                        move || if ctx.me.get().is_some() { String::new() } else { "Не выполнен".into() },
                        move || ctx.open_page(Page::Acc))}
                </div>

                <h3>"ИИ"</h3>
                <div class="grp">
                    {nav_row("ai", || "Персональный ИИ".into(), move || name(ctx), move || ctx.open_page(Page::Ai))}
                </div>

                <h3>"Подписка"</h3>
                <div class="grp">{nav_row("lock", || "Тарифы".into(), move || model_name(ctx), move || ctx.open_page(Page::Plan))}</div>

                <h3>"Настройки"</h3>
                <div class="grp">
                    {sel_row("moon", "Тема", theme_opts, move || ctx.cfg.get().theme, move |v| ctx.cfg.update(|c| c.theme = v))}
                    {sel_row("pal", "Палитра", pal_opts, move || ctx.pal.get().to_string(), move |v| ctx.set_pal(v.parse().unwrap_or(0)))}
                    {sel_row("bolt", "Модель", mod_opts, move || ctx.mi.get().to_string(), move |v| ctx.set_model(v.parse().unwrap_or(0)))}
                    {tog_row("wave", "Анимации", move || ctx.cfg.get().motion, move || ctx.cfg.update(|c| c.motion = !c.motion))}
                    <div class="row rg">
                        {ic("glass")}<span class="rt">"Эффект стекла"</span>
                        <span class="rv">{move || format!("{}%", ctx.cfg.get().glass)}</span>
                        <input type="range" min="0" max="100" step="5" aria-label="Эффект стекла"
                               prop:value=move || ctx.cfg.get().glass.to_string()
                               on:input=move |e| ctx.cfg.update(|c| c.glass = event_target_value(&e).parse().unwrap_or(100))/>
                    </div>
                    {sel_row("text", "Размер текста", fs_opts, move || ctx.cfg.get().fs, move |v| ctx.cfg.update(|c| c.fs = v))}
                    {tog_row("clock", "Сохранять историю", move || ctx.cfg.get().keep, move || ctx.cfg.update(|c| c.keep = !c.keep))}
                </div>

                <h3>"Данные и конфиденциальность"</h3>
                <div class="grp">
                    {act_row("dl", "Экспорт чатов", false, armed, move || ctx.export_chats())}
                    {act_row("trash", "Очистить историю", true, armed, move || ctx.clear_hist())}
                </div>

                <h3>"Документы"</h3>
                <div class="grp">
                    {doc_row(ctx, Some("lock"), "Конфиденциальность", "privacy", Page::Main)}
                    {doc_row(ctx, Some("text"), "Условия использования", "terms", Page::Main)}
                    {doc_row(ctx, Some("user"), "Согласие на обработку данных", "consent", Page::Main)}
                    {doc_row(ctx, Some("clock"), "Данные в браузере", "cookies", Page::Main)}
                    {doc_row(ctx, Some("pal"), "Подписка и возврат", "subscription", Page::Main)}
                    {doc_row(ctx, Some("ai"), "Об ИИ-ответах", "ai", Page::Main)}
                </div>

                <h3>"Поддержка"</h3>
                <div class="grp">
                    {nav_row("info", || "О приложении".into(), || String::new(), move || ctx.open_page(Page::About))}
                    {act_row("reset", "Сбросить настройки", true, armed, move || {
                        ctx.cfg.set(Cfg::default());
                        ctx.set_pal(0);
                        ctx.set_model(0);
                    })}
                </div>
            </div>
        </section>
    }
}

#[component]
fn PageAi() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    view! {
        <section class=pgc(ctx, Page::Ai)>
            {ph(ctx, || "Персональный ИИ".into(), move || ctx.open_page(Page::Main))}
            <div class="sc">
                <div class="fl">"Шаблоны"</div>
                <div class="pre">
                    {data().pre.iter().map(|p| {
                        let (w, r) = (p.who.clone(), p.rules.clone());
                        view! { <button on:click=move |_| ctx.cfg.update(|c| { c.who = w.clone(); c.rules = r.clone(); })>{p.n.clone()}</button> }
                    }).collect_view()}
                </div>
                <label class="fl" for="f-name">"Имя"</label>
                <input class="fi" id="f-name" maxlength="40" placeholder="Luna" autocomplete="off"
                       prop:value=move || ctx.cfg.get().name on:input=move |e| ctx.cfg.update(|c| c.name = event_target_value(&e))/>
                <label class="fl" for="f-who">"Кто он"</label>
                <textarea class="fi" id="f-who" maxlength="1200"
                          placeholder="Например: лунный кролик-астроном. Живёт на обратной стороне Луны, любит звёзды, говорит тепло и с лёгкой иронией."
                          prop:value=move || ctx.cfg.get().who on:input=move |e| ctx.cfg.update(|c| c.who = event_target_value(&e))></textarea>
                <label class="fl" for="f-rules">"Как он должен себя вести"</label>
                <textarea class="fi" id="f-rules" maxlength="1500"
                          placeholder="Например: отвечай коротко, не читай мораль, оставайся в образе, задавай встречные вопросы."
                          prop:value=move || ctx.cfg.get().rules on:input=move |e| ctx.cfg.update(|c| c.rules = event_target_value(&e))></textarea>
                <div class="fl">"Длина ответов"</div>
                <div class="seg">
                    {[("short", "Кратко"), ("normal", "Обычно"), ("long", "Подробно")].into_iter().map(|(k, t)| view! {
                        <button class:on=move || ctx.cfg.get().len == k on:click=move |_| ctx.cfg.update(|c| c.len = k.to_string())>{t}</button>
                    }).collect_view()}
                </div>
                <label class="fl" for="f-you">"Как обращаться к вам"</label>
                <input class="fi" id="f-you" maxlength="40" autocomplete="off"
                       prop:value=move || ctx.cfg.get().you on:input=move |e| ctx.cfg.update(|c| c.you = event_target_value(&e))/>
                <button class="fi rst" on:click=move |_| ctx.cfg.update(|c| { c.name = "Luna".into(); c.who.clear(); c.rules.clear(); c.len = "normal".into(); c.you.clear(); })>"Сбросить персонажа"</button>
                <p class="hint">"Эти инструкции добавляются к каждому разговору и действуют со следующего сообщения."</p>
            </div>
        </section>
    }
}

#[component]
fn PageAcc() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let up = create_rw_signal(false);
    let ok = create_rw_signal(false);
    let err = create_rw_signal(String::new());
    let nm = create_rw_signal(String::new());
    let email = create_rw_signal(String::new());
    let pass = create_rw_signal(String::new());
    let auth_busy = create_rw_signal(false);

    let info = create_rw_signal(String::new());
    let go = move || {
        if auth_busy.get_untracked() {
            return;
        }
        let (m, p, n) = (email.get_untracked().trim().to_string(), pass.get_untracked(), nm.get_untracked().trim().to_string());
        let reg = up.get_untracked();
        if reg && n.is_empty() {
            return err.set("Введите имя".into());
        }
        if reg && n.chars().count() > 40 {
            return err.set("Имя должно быть не длиннее 40 символов".into());
        }
        if m.len() > 254 || !m.contains('@') || !m.rsplit_once('@').map_or(false, |(_, domain)| domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')) {
            return err.set("Введите корректную почту".into());
        }
        if p.chars().count() < 8 || p.chars().count() > 128 {
            return err.set("Пароль должен содержать от 8 до 128 символов".into());
        }
        if reg && !ok.get_untracked() {
            return err.set("Подтвердите согласие с условиями".into());
        }
        err.set(String::new());
        info.set(String::new());
        auth_busy.set(true);
        spawn_local(async move {
            let path = if reg { "/api/register" } else { "/api/login" };
            let result = api::auth(path, &serde_json::json!({ "name": n, "email": m, "pass": p })).await;
            auth_busy.set(false);
            match result {
                Ok(api::AuthOutcome::SignedIn(me)) => {
                    pass.set(String::new());
                    ctx.finish_login(me);
                }
                Ok(api::AuthOutcome::NeedsEmailConfirmation(address)) => {
                    pass.set(String::new());
                    up.set(false);
                    ok.set(false);
                    info.set(format!("Аккаунт создан. Подтвердите адрес {} по ссылке из письма, затем войдите.", address));
                }
                Err(e) => err.set(e),
            }
        });
    };

    view! {
        <section class=pgc(ctx, Page::Acc)>
            {ph(ctx, || "Аккаунт".into(), move || ctx.open_page(Page::Main))}
            <div class="sc">
                {move || match ctx.me.get() {
                    Some(me) => view! {
                        <div class="prof">
                            <div class="avt">{me.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into())}</div>
                            <div><b>{me.name.clone()}</b><span>{me.email.clone()}</span></div>
                        </div>
                        <div class="grp" style="margin-top:14px">
                            <div class="row"><span class="rt">"Тариф"</span><span class="rv">{move || model_name(ctx)}</span></div>
                            <div class="row"><span class="rt">"Способ входа"</span><span class="rv">"Эл. почта"</span></div>
                        </div>
                        <button class="fi rst" on:click=move |_| ctx.logout()>"Выйти"</button>
                        <p class="hint">"История чатов хранится на сервере и привязана к аккаунту."</p>
                    }.into_view(),
                    None => view! {
                        <div class="prof">
                            {move || rb(ctx.mi.get())}
                            <div>
                                <b>{move || if up.get() { "Создать аккаунт" } else { "Войдите в аккаунт" }}</b>
                                <span>"Чтобы хранить чаты и писать ИИ"</span>
                            </div>
                        </div>
                        <form on:submit=move |e| { e.prevent_default(); go(); }>
                            <Show when=move || up.get()>
                                <label class="fl" for="a-name">"Имя"</label>
                                <input class="fi" id="a-name" maxlength="40" autocomplete="name" prop:value=move || nm.get() on:input=move |e| nm.set(event_target_value(&e))/>
                            </Show>
                            <label class="fl" for="a-mail">"Электронная почта"</label>
                            <input class="fi" id="a-mail" type="email" inputmode="email" autocomplete="email" required placeholder="name@mail.com"
                                   prop:value=move || email.get() on:input=move |e| email.set(event_target_value(&e))/>
                            <label class="fl" for="a-pass">"Пароль"</label>
                            <input class="fi" id="a-pass" type="password" minlength="8" maxlength="128" required
                                   autocomplete=move || if up.get() { "new-password" } else { "current-password" }
                                   prop:value=move || pass.get() on:input=move |e| pass.set(event_target_value(&e))/>
                            <Show when=move || up.get()>
                                <div class="grp" style="margin-top:14px">
                                    <div class="row" on:click=move |_| ok.update(|v| *v = !*v)>
                                        <span class="rt">"Принимаю условия и политику"</span><span class="tg" class:on=move || ok.get()></span>
                                    </div>
                                    {doc_row(ctx, None, "Условия использования", "terms", Page::Acc)}
                                    {doc_row(ctx, None, "Политика конфиденциальности", "privacy", Page::Acc)}
                                    {doc_row(ctx, None, "Согласие на обработку данных", "consent", Page::Acc)}
                                </div>
                            </Show>
                            <p class="err">{move || err.get()}</p>
                            <p class="hint">{move || info.get()}</p>
                            <button class="cta" type="submit" disabled=move || auth_busy.get()>{move || if auth_busy.get() { "Подождите…" } else if up.get() { "Создать аккаунт" } else { "Войти" }}</button>
                            <button class="lnk" type="button" disabled=move || auth_busy.get() on:click=move |_| { err.set(String::new()); info.set(String::new()); up.update(|v| *v = !*v); }>
                                {move || if up.get() { "У меня уже есть аккаунт" } else { "Создать аккаунт" }}
                            </button>
                            <p class="hint">"Пароль обрабатывается сервером; приложение не хранит его в открытом виде."</p>
                        </form>
                    }.into_view(),
                }}
            </div>
        </section>
    }
}

#[component]
fn PagePlan() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    view! {
        <section class=pgc(ctx, Page::Plan)>
            {ph(ctx, || "Тарифы".into(), move || ctx.open_page(Page::Main))}
            <div class="sc">
                {data().models.iter().enumerate().map(|(k, m)| {
                    let feats = data().feat[k].clone();
                    let pro = m.pro;
                    view! {
                        <div class="plan" class:cur=move || ctx.mi.get() == k on:click=move |_| { if ctx.mi.get_untracked() != k { ctx.set_model(k) } }>
                            <div class="ph2">
                                {rb(k)}
                                <div><b>{m.n.clone()}</b><span>{if pro { "Платная версия" } else { "Бесплатно" }}</span></div>
                            </div>
                            <ul>{feats.into_iter().map(|f| view! { <li>{f}</li> }).collect_view()}</ul>
                            <div class="pbtn">
                                {move || if ctx.mi.get() == k {
                                    view! { <span>"Текущий тариф"</span> }.into_view()
                                } else {
                                    view! {
                                        {pro.then(|| view! { <svg class="lk" viewBox="0 0 24 24" inner_html=LOCK></svg> })}
                                        <span>"Выбрать"</span>
                                    }.into_view()
                                }}
                            </div>
                        </div>
                    }
                }).collect_view()}
                <p class="hint">"Демонстрация: оплата не подключена, тарифы переключаются свободно."</p>
            </div>
        </section>
    }
}

#[component]
fn PageAbout() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    view! {
        <section class=pgc(ctx, Page::About)>
            {ph(ctx, || "О приложении".into(), move || ctx.open_page(Page::Main))}
            <div class="sc">
                <div class="grp">
                    <div class="row"><span class="rt">"Версия"</span><span class="rv">"2.0 (Rust)"</span></div>
                    <div class="row"><span class="rt">"Хранение данных"</span><span class="rv">"Сервер"</span></div>
                </div>
                <p class="hint">"Ответы создаются внутренней AI-системой Luna. Аккаунт и история чатов хранятся на сервере, настройки внешнего вида — в браузере. Текст разговора и инструкции передаются AI-системе при отправке сообщения."</p>
            </div>
        </section>
    }
}

#[component]
fn PageDoc() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    view! {
        <section class=pgc(ctx, Page::Doc)>
            {ph(ctx,
                move || data().docs.get(&ctx.doc.get()).map(|d| d.h.clone()).unwrap_or_default(),
                move || ctx.open_page(ctx.doc_from.get_untracked()))}
            <div class="sc">
                {move || data().docs.get(&ctx.doc.get()).map(|d| view! {
                    <div class="prof">
                        {rb(ctx.mi.get_untracked())}
                        <div><b>{d.t.clone()}</b><span>{format!("Редакция от {}", data().lg_date)}</span></div>
                    </div>
                    {d.s.iter().map(|(h, t)| view! { <div class="grp doc"><b>{h.clone()}</b><p>{t.clone()}</p></div> }).collect_view()}
                }.into_view())}
            </div>
        </section>
    }
}

// ───────────── лента сообщений ─────────────

fn row_view(ctx: Ctx, r: Row) -> View {
    if r.user {
        return view! { <div class="msg me">{r.text}</div> }.into_view();
    }
    let k = ctx.mi.get_untracked();
    if r.fx == 0 {
        return view! { <div class="msg ai"><div class="glass av">{rb(k)}</div><div class="tx">{r.text}</div></div> }.into_view();
    }
    let art = if r.fx == 1 { data().rb_stop.clone() } else { data().rb_lim.clone() };
    let plan = data().models[k].n.clone();
    view! {
        <div class="msg ai">
            <div class="glass av">{rb(k)}</div>
            <div class="tx">
                <div class="rep">
                    {(!r.part.is_empty()).then(|| view! { <div>{r.part.clone()}</div> })}
                    <div class="glass big"><svg class="rb a1" viewBox="0 0 100 100" inner_html=art></svg></div>
                    <div>{r.text.clone()}</div>
                    {(r.fx == 2).then(|| view! {
                        <div class="grp lim">
                            <div class="row"><span class="rt">"Статус"</span><span class="rv">"Лимит превышен"</span></div>
                            <div class="row"><span class="rt">"Тариф"</span><span class="rv">{plan.clone()}</span></div>
                            <div class="row"><span class="rt">"Что делать"</span><span class="rv">"Подождать немного"</span></div>
                        </div>
                    })}
                </div>
            </div>
        </div>
    }
    .into_view()
}

#[component]
fn Log() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let anim = create_rw_signal(None::<usize>);
    let streaming = create_memo(move |_| ctx.stream.with(|s| s.is_some()));
    let blank = create_memo(move |_| ctx.stream.with(|s| s.as_ref().map_or(true, |t| t.is_empty())));
    let empty = create_memo(move |_| ctx.log.with(|l| l.is_empty()) && !streaming.get());
    let log_el = ctx.log_el;

    view! {
        <main id="log" node_ref=log_el>
            <Show when=move || empty.get()>
                <div class="empty">
                    <div class="glass big hey" on:click=move |_| {
                        anim.set(Some(ctx.mi.get_untracked()));
                        set_timeout(move || anim.set(None), Duration::from_millis(1100));
                    }>
                        <svg viewBox="0 0 100 100"
                             class=move || match anim.get() { Some(k) => format!("rb a{k}"), None => "rb".to_string() }
                             prop:innerHTML=move || data().rb[ctx.mi.get()].clone()></svg>
                    </div>
                    <h1>{move || name(ctx)}</h1>
                    <p>"Разговоры, истории и ролевые игры в одном месте."</p>
                </div>
            </Show>
            <For each=move || ctx.log.get() key=|r| r.id children=move |r| row_view(ctx, r)/>
            <Show when=move || streaming.get()>
                <div class="msg ai">
                    <div class="glass av">{move || rb(ctx.mi.get())}</div>
                    <div class="tx">
                        <Show when=move || blank.get()><span class="dots"><i></i><i></i><i></i></span></Show>
                        {move || ctx.stream.get().unwrap_or_default()}
                    </div>
                </div>
            </Show>
        </main>
    }
}

// ───────────── поле ввода ─────────────

#[component]
fn Composer() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let draft = create_rw_signal(String::new());
    let ta = create_node_ref::<html::Textarea>();

    let fit = move || {
        if let Some(t) = ta.get_untracked() {
            let t: &web_sys::HtmlTextAreaElement = t.unchecked_ref();
            let st = t.style();
            let _ = st.set_property("height", "auto");
            let h = t.scroll_height().min(140);
            let _ = st.set_property("height", &format!("{h}px"));
        }
    };
    let go = move || {
        if ctx.busy.get_untracked() {
            ctx.stop();
            return;
        }
        ctx.send(draft.get_untracked());
        if ctx.busy.get_untracked() {
            draft.set(String::new());
            request_animation_frame(fit);
        }
    };

    view! {
        <footer>
            <div class="glass bar">
                <textarea id="inp" rows="1" placeholder="Напишите сообщение" aria-label="Сообщение" autocomplete="off" autocapitalize="sentences"
                          node_ref=ta prop:value=move || draft.get()
                          on:input=move |e| { draft.set(event_target_value(&e)); fit(); }
                          on:keydown=move |e: web_sys::KeyboardEvent| {
                              if e.key() == "Enter" && !e.shift_key() && !coarse() { e.prevent_default(); go(); }
                          }
                          on:focus=move |_| set_timeout(move || ctx.scroll(false), Duration::from_millis(300))></textarea>
                <button class="glass send" id="send"
                        aria-label=move || if ctx.busy.get() { "Стоп" } else { "Отправить" }
                        disabled=move || !ctx.busy.get() && draft.get().trim().is_empty()
                        on:pointerdown=move |e: web_sys::PointerEvent| e.prevent_default()
                        on:click=move |_| go()>
                    <svg viewBox="0 0 24 24" prop:innerHTML=move || if ctx.busy.get() { STOP } else { SEND }></svg>
                </button>
            </div>
        </footer>
    }
}
