use crate::{api, data::data, state::*};
use leptos::*;
use std::time::Duration;
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};

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
        <div id="app">
            <div id="shell">
                <Header/>
                <Log/>
                <Composer/>
            </div>
            <PalPop/>
            <ModelPop/>
            <div class="scrim" class:on=move || ctx.drawer.get() || ctx.sheet.get() on:click=move |_| ctx.close_all()></div>
            <div class="glass toast" class:on=move || ctx.toast_on.get() role="status">{move || ctx.toast.get()}</div>
            <Drawer/>
            <Sheet/>
        </div>
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

    // Фокус снимается только жестами внутри ленты (#log):
    // обычные нажатия по шапке и другим элементам не закрывают клавиатуру.
    window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            ctx.close_all();
        }
    });

    let r = root.clone();
    window_event_listener(ev::visibilitychange, move |_| {
        let _ = r.class_list().toggle_with_force("paused", document().hidden());
    });

    // iOS Safari: размер и вертикальное смещение видимой области.
    if let Some(vv) = window().visual_viewport() {
        let update_viewport = {
            let root = root.clone();
            let vv = vv.clone();
            move || {
                let log_metrics = document().query_selector("#log").ok().flatten()
                    .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
                    .map(|el| {
                        let distance = (el.scroll_height() - el.scroll_top() - el.client_height()).max(0);
                        (el, distance)
                    });
                let _ = root.style().set_property("--vvh", &format!("{}px", vv.height()));
                let _ = root.style().set_property("--vvt", &format!("{}px", vv.offset_top()));
                let layout_h = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(vv.height());
                let keyboard_open = vv.height() < layout_h - 120.0;
                let _ = root.class_list().toggle_with_force("keyboard-open", keyboard_open);
                if let Some((_, distance)) = log_metrics.as_ref() {
                    let distance = *distance;
                    request_animation_frame(move || {
                        if let Ok(Some(node)) = document().query_selector("#log") {
                            if let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() {
                                let top = (el.scroll_height() - el.client_height() - distance).max(0);
                                el.set_scroll_top(top);
                            }
                        }
                    });
                }
                // Не сбрасываем window.scrollTo() на каждом событии VisualViewport:
                // Safari сам меняет visual viewport при появлении клавиатуры, а принудительный
                // сброс создаёт гонку с его внутренним repositioning.
            }
        };
        update_viewport();
        let fit = Closure::<dyn FnMut()>::new(update_viewport);
        let _ = vv.add_event_listener_with_callback("resize", fit.as_ref().unchecked_ref());
        let _ = vv.add_event_listener_with_callback("scroll", fit.as_ref().unchecked_ref());
        fit.forget();
    } else {
        let h = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let _ = root.style().set_property("--vvh", &format!("{}px", h));
        let _ = root.style().set_property("--vvt", "0px");
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
            <button class="glass chg" aria-label=move || if ctx.me.get().is_some() { "О приложении" } else { "Войти через Telegram" } on:click=move |_| { if ctx.me.get().is_some() { ctx.open_sheet(Page::About) } else { ctx.open_sheet(Page::Acc) } }>{move || if ctx.me.get().is_some() { "О приложении" } else { "Войти" }}</button>
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

    create_effect(move |_| {
        ctx.model_preview.set(ctx.mi.get());
    });

    let mv = move |e: web_sys::PointerEvent| {
        if let Some(el) = msh_ref.get_untracked() {
            let y = e.client_y() as f64 - el.get_bounding_client_rect().top();
            ly.set((y - 29.0).clamp(6.0, 6.0 + 2.0 * 62.0));
            let i = (((y - 6.0) / 62.0).floor().max(0.0) as usize).min(2);
            ctx.model_preview.set(i);
        }
    };
    let rel = move |_: web_sys::PointerEvent| {
        if held.get_untracked() {
            held.set(false);
            let selected = ctx.model_preview.get_untracked();
            if selected != ctx.mi.get_untracked() {
                ctx.set_model(selected);
            }
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
                <div class="mc" class:pro=m.pro class:sel=move || ctx.model_preview.get() == k>
                    <div class="glass ic">{rb(k)}</div>
                    <div class="mt"><b>{m.n.clone()}</b><span>{m.d.clone()}</span></div>
                    {
                        let is_pro = m.pro;
                        move || {
                            if is_pro && !ctx.me.get().map(|me| me.channel_member).unwrap_or(false) {
                                view! { <svg class="lk" viewBox="0 0 24 24" inner_html=LOCK></svg> }.into_view()
                            } else if !is_pro {
                                view! { <em>"авто"</em> }.into_view()
                            } else {
                                ().into_view()
                            }
                        }
                    }
                </div>
            }).collect_view()}
            <div class="mlens" style:top=move || format!("{}px", if held.get() { ly.get() } else { 6.0 + ctx.model_preview.get() as f64 * 62.0 })></div>
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
        <div class="row">
            {ic(icn)}<span class="rt">{t}</span>
            <button class="tg" type="button" role="switch" aria-label=t
                    aria-checked=move || if get() { "true" } else { "false" }
                    class:on=get on:click=move |_| toggle()></button>
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
                <PageMain/><PageAi/><PageAcc/><PagePlan/><PageMemory/><PageLab/><PageAbout/><PageDoc/>
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

                <h3>"Память"</h3>
                <div class="grp">
                    {nav_row("clock", || "Память Luna".into(), || "Истории и факты".into(), move || ctx.open_page(Page::Memory))}
                </div>

                <h3>"Лаборатория"</h3>
                <div class="grp">
                    {nav_row("bolt", || "Лаборатория".into(), || "Персоны · Лорбуки · Пресеты".into(), move || ctx.open_page(Page::Lab))}
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
    let prompt_words = move || {
        let cfg = ctx.cfg.get();
        cfg.who.split_whitespace().count() + cfg.rules.split_whitespace().count()
    };
    let prompt_limit = move || ctx.me.get().map(|m| m.prompt_limit).unwrap_or(100);
    let prompt_over = move || prompt_words() > prompt_limit();
    view! {
        <section class=pgc(ctx, Page::Ai)>
            {ph(ctx, || "Персональный ИИ".into(), move || ctx.open_page(Page::Main))}
            <div class="sc ai-settings-sc">
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
                <textarea class="fi" id="f-who" maxlength="12000"
                          placeholder="Например: лунный кролик-астроном. Живёт на обратной стороне Луны, любит звёзды, говорит тепло и с лёгкой иронией."
                          prop:value=move || ctx.cfg.get().who on:input=move |e| ctx.cfg.update(|c| c.who = event_target_value(&e))></textarea>
                <label class="fl" for="f-rules">"Как он должен себя вести"</label>
                <textarea class="fi" id="f-rules" maxlength="18000"
                          placeholder="Например: отвечай коротко, не читай мораль, оставайся в образе, задавай встречные вопросы."
                          prop:value=move || ctx.cfg.get().rules on:input=move |e| ctx.cfg.update(|c| c.rules = event_target_value(&e))></textarea>
                <p class="hint">{move || format!("Инструкции: {} / {} слов", prompt_words(), prompt_limit())}</p>
                {move || if prompt_over() {
                    view! {
                        <div class="grp lim prompt-limit-warning">
                            <p>"Вы превысили лимит бесплатной версии. Сократите инструкции или посмотрите платные тарифы с большим лимитом."</p>
                            <button class="cta" type="button" on:click=move |_| ctx.open_page(Page::Plan)>"Посмотреть тарифы"</button>
                        </div>
                    }.into_view()
                } else { ().into_view() }}
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
    let err = create_rw_signal(String::new());
    let auth_busy = create_rw_signal(false);

    let start_login = move |_| {
        if auth_busy.get_untracked() {
            return;
        }
        err.set(String::new());
        auth_busy.set(true);
        let global = js_sys::global();
        let start = js_sys::Reflect::get(&global, &"lunaTelegramStart".into())
            .ok()
            .and_then(|value| value.dyn_into::<js_sys::Function>().ok());
        match start {
            Some(function) => {
                if function.call0(&global).is_err() {
                    auth_busy.set(false);
                    err.set("Не удалось открыть Telegram. Попробуйте ещё раз.".into());
                }
            }
            None => {
                auth_busy.set(false);
                err.set("Модуль входа не загрузился. Обновите страницу.".into());
            }
        }
    };

    view! {
        <section class=pgc(ctx, Page::Acc)>
            {ph(ctx, || "Аккаунт".into(), move || ctx.open_page(Page::Main))}
            <div class="sc">
                {move || match ctx.me.get() {
                    Some(me) => view! {
                        <div class="prof">
                            <div class="avt">{me.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into())}</div>
                            <div>
                                <b>{me.name.clone()}</b>
                                <span>{if me.email.is_empty() { "Telegram".to_string() } else { me.email.clone() }}</span>
                            </div>
                        </div>
                        <div class="grp" style="margin-top:14px">
                            <div class="row"><span class="rt">"Тариф"</span><span class="rv">{move || model_name(ctx)}</span></div>
                            <div class="row"><span class="rt">"Способ входа"</span><span class="rv">"Telegram"</span></div>
                        </div>
                        <button class="fi rst" on:click=move |_| ctx.logout()>"Выйти"</button>
                        <p class="hint">"История чатов хранится на сервере и привязана к аккаунту."</p>
                    }.into_view(),
                    None => view! {
                        <div class="prof">
                            {move || rb(ctx.mi.get())}
                            <div>
                                <b>"Войти в Luna"</b>
                                <span>"Используйте свой аккаунт Telegram"</span>
                            </div>
                        </div>
                        <button class="cta" type="button" disabled=move || auth_busy.get() on:click=start_login>
                            {move || if auth_busy.get() { "Открываем Telegram…" } else { "Войти через Telegram" }}
                        </button>
                        <p class="err">{move || err.get()}</p>
                        <div class="grp" style="margin-top:14px">
                            {doc_row(ctx, None, "Условия использования", "terms", Page::Acc)}
                            {doc_row(ctx, None, "Политика конфиденциальности", "privacy", Page::Acc)}
                            {doc_row(ctx, None, "Согласие на обработку данных", "consent", Page::Acc)}
                        </div>
                        <p class="hint">"Вход выполняется через Telegram. Luna не получает ваш пароль Telegram."</p>
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
                                        {(pro && !ctx.me.get().map(|me| me.channel_member).unwrap_or(false)).then(|| view! { <svg class="lk" viewBox="0 0 24 24" inner_html=LOCK></svg> })}
                                        <span>"Выбрать"</span>
                                    }.into_view()
                                }}
                            </div>
                        </div>
                    }
                }).collect_view()}
                <p class="hint">"Бесплатный тариф доступен всем. Luna и Luna Pro доступны после подтверждения подписки на Telegram-канал Luna AI. Проверка выполняется сервером при выборе тарифа."</p>
            </div>
        </section>
    }
}


#[component]
fn PageLab() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let tab = create_rw_signal("personas".to_string());
    let persona_name = create_rw_signal(String::new());
    let preset_name = create_rw_signal(String::new());
    let preset_instructions = create_rw_signal(String::new());
    let preset_len = create_rw_signal("normal".to_string());
    let lore_name = create_rw_signal(String::new());
    let lore_keywords = create_rw_signal(String::new());
    let lore_content = create_rw_signal(String::new());

    let add_persona = move |_| {
        let label = persona_name.get_untracked().trim().to_string();
        if label.is_empty() {
            ctx.say("Укажите название персоны.");
            return;
        }
        let current = ctx.cfg.get_untracked();
        let id = format!("persona-{}", js_sys::Date::now() as u64);
        ctx.cfg.update(|cfg| cfg.lab.personas.push(LabPersona {
            id, name: label, who: current.who.clone(), rules: current.rules.clone(),
        }));
        persona_name.set(String::new());
        ctx.say("Персона сохранена.");
    };

    let add_preset = move |_| {
        let label = preset_name.get_untracked().trim().to_string();
        if label.is_empty() {
            ctx.say("Укажите название пресета.");
            return;
        }
        let id = format!("preset-{}", js_sys::Date::now() as u64);
        let len = preset_len.get_untracked();
        let instructions = preset_instructions.get_untracked().trim().to_string();
        ctx.cfg.update(|cfg| cfg.lab.presets.push(LabPreset { id, name: label, len, instructions }));
        preset_name.set(String::new());
        preset_instructions.set(String::new());
        ctx.say("Пресет сохранён.");
    };

    let add_lorebook = move |_| {
        let label = lore_name.get_untracked().trim().to_string();
        let content = lore_content.get_untracked().trim().to_string();
        let keywords = lore_keywords.get_untracked().split(',').map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()).take(20).collect::<Vec<_>>();
        if label.is_empty() || content.is_empty() || keywords.is_empty() {
            ctx.say("Заполните название, ключевые слова и содержание лорбука.");
            return;
        }
        let id = format!("lore-{}", js_sys::Date::now() as u64);
        ctx.cfg.update(|cfg| cfg.lab.lorebooks.push(LabLorebook {
            id, name: label, keywords, content, enabled: true,
        }));
        lore_name.set(String::new());
        lore_keywords.set(String::new());
        lore_content.set(String::new());
        ctx.say("Запись лорбука сохранена.");
    };

    view! {
        <section class=pgc(ctx, Page::Lab)>
            {ph(ctx, || "Лаборатория".into(), move || ctx.open_page(Page::Main))}
            <div class="sc lab-sc">
                <div class="lab-intro">
                    <div class="lab-mark">{rb(ctx.mi.get())}</div>
                    <div>
                        <b>"Настрой свою Luna"</b>
                        <p>"Персонажи, знания и стиль ответа — в одном месте."</p>
                    </div>
                </div>
                <div class="lab-tabs" role="tablist" aria-label="Инструменты Luna">
                    <button class:on=move || tab.get() == "personas" on:click=move |_| tab.set("personas".into())>"Персоны"</button>
                    <button class:on=move || tab.get() == "lore" on:click=move |_| tab.set("lore".into())>"Лорбуки"</button>
                    <button class:on=move || tab.get() == "presets" on:click=move |_| tab.set("presets".into())>"Пресеты"</button>
                    <button class:on=move || tab.get() == "vectors" on:click=move |_| tab.set("vectors".into())>"Embeddings"</button>
                    <button class:on=move || tab.get() == "plugins" on:click=move |_| tab.set("plugins".into())>"Плагины"</button>
                </div>
                {move || match tab.get().as_str() {
                    "personas" => view! {
                        <div class="lab-panel">
                            <div class="lab-heading"><b>"Персоны"</b><span>{move || format!("{} сохранено", ctx.cfg.get().lab.personas.len())}</span></div>
                            <p class="hint">"Сохрани текущую личность Luna как отдельную персону. Применение заменит активное имя и инструкции, но не удалит сохранённые профили."</p>
                            <label class="fl" for="lab-persona-name">"Название новой персоны"</label>
                            <input class="fi" id="lab-persona-name" maxlength="60" placeholder="Например, Детектив" prop:value=move || persona_name.get() on:input=move |e| persona_name.set(event_target_value(&e))/>
                            <button class="cta lab-action" type="button" on:click=add_persona>"Сохранить текущую персону"</button>
                            <div class="lab-list">
                                {move || ctx.cfg.get().lab.personas.iter().map(|persona| {
                                    let item = persona.clone();
                                    let apply = persona.clone();
                                    let delete_id = persona.id.clone();
                                    view! {
                                        <div class="lab-item">
                                            <div class="lab-item-copy"><b>{item.name.clone()}</b><span>{if item.who.is_empty() { "Без описания".to_string() } else { item.who.chars().take(90).collect::<String>() }}</span></div>
                                            <button class="lab-mini" type="button" on:click=move |_| ctx.cfg.update(|cfg| {
                                                cfg.name = apply.name.clone(); cfg.who = apply.who.clone(); cfg.rules = apply.rules.clone();
                                            })>"Применить"</button>
                                            <button class="lab-remove" type="button" aria-label="Удалить персону" on:click=move |_| ctx.cfg.update(|cfg| cfg.lab.personas.retain(|p| p.id != delete_id))>"×"</button>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }.into_view(),
                    "lore" => view! {
                        <div class="lab-panel">
                            <div class="lab-heading"><b>"Лорбуки"</b><span>{move || format!("{} записей", ctx.cfg.get().lab.lorebooks.len())}</span></div>
                            <p class="hint">"Добавляй факты о мире, героях и событиях. Запись попадёт в контекст, когда ключевое слово встретится в твоём последнем сообщении."</p>
                            <label class="fl" for="lab-lore-name">"Название"</label>
                            <input class="fi" id="lab-lore-name" maxlength="60" placeholder="Например, Мир Астэр" prop:value=move || lore_name.get() on:input=move |e| lore_name.set(event_target_value(&e))/>
                            <label class="fl" for="lab-lore-keys">"Ключевые слова через запятую"</label>
                            <input class="fi" id="lab-lore-keys" maxlength="500" placeholder="Астэр, северное королевство, магия" prop:value=move || lore_keywords.get() on:input=move |e| lore_keywords.set(event_target_value(&e))/>
                            <label class="fl" for="lab-lore-content">"Содержание"</label>
                            <textarea class="fi" id="lab-lore-content" maxlength="4000" placeholder="Опиши факты, которые Luna должна учитывать…" prop:value=move || lore_content.get() on:input=move |e| lore_content.set(event_target_value(&e))></textarea>
                            <button class="cta lab-action" type="button" on:click=add_lorebook>"Добавить в лорбук"</button>
                            <div class="lab-list">
                                {move || ctx.cfg.get().lab.lorebooks.iter().map(|entry| {
                                    let item = entry.clone();
                                    let toggle_id = entry.id.clone();
                                    let delete_id = entry.id.clone();
                                    view! {
                                        <div class="lab-item lab-lore-item">
                                            <button class="lab-state" type="button" class:on=item.enabled aria-label="Включить или выключить запись" on:click=move |_| ctx.cfg.update(|cfg| {
                                                if let Some(found) = cfg.lab.lorebooks.iter_mut().find(|v| v.id == toggle_id) { found.enabled = !found.enabled; }
                                            })><span></span></button>
                                            <div class="lab-item-copy"><b>{item.name.clone()}</b><span>{item.keywords.join(" · ")}</span><small>{item.content.chars().take(110).collect::<String>()}</small></div>
                                            <button class="lab-remove" type="button" aria-label="Удалить запись" on:click=move |_| ctx.cfg.update(|cfg| cfg.lab.lorebooks.retain(|v| v.id != delete_id))>"×"</button>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }.into_view(),
                    "presets" => view! {
                        <div class="lab-panel">
                            <div class="lab-heading"><b>"Пресеты ответа"</b><span>{move || format!("{} сохранено", ctx.cfg.get().lab.presets.len())}</span></div>
                            <p class="hint">"Пресет задаёт длину и дополнительные правила ответа. Он применяется поверх выбранной персоны."</p>
                            <label class="fl" for="lab-preset-name">"Название пресета"</label>
                            <input class="fi" id="lab-preset-name" maxlength="60" placeholder="Например, Короткие ответы" prop:value=move || preset_name.get() on:input=move |e| preset_name.set(event_target_value(&e))/>
                            <div class="fl">"Длина ответа"</div>
                            <div class="seg">
                                <button class:on=move || preset_len.get() == "short" on:click=move |_| preset_len.set("short".into())>"Кратко"</button>
                                <button class:on=move || preset_len.get() == "normal" on:click=move |_| preset_len.set("normal".into())>"Обычно"</button>
                                <button class:on=move || preset_len.get() == "long" on:click=move |_| preset_len.set("long".into())>"Подробно"</button>
                            </div>
                            <label class="fl" for="lab-preset-instructions">"Дополнительные правила"</label>
                            <textarea class="fi" id="lab-preset-instructions" maxlength="1200" placeholder="Например: сначала вывод, затем короткое объяснение." prop:value=move || preset_instructions.get() on:input=move |e| preset_instructions.set(event_target_value(&e))></textarea>
                            <button class="cta lab-action" type="button" on:click=add_preset>"Сохранить пресет"</button>
                            <div class="lab-list">
                                {move || ctx.cfg.get().lab.presets.iter().map(|preset| {
                                    let item = preset.clone();
                                    let apply = preset.clone();
                                    let active_id = preset.id.clone();
                                    let delete_id = preset.id.clone();
                                    view! {
                                        <div class="lab-item">
                                            <div class="lab-item-copy"><b>{item.name.clone()}</b><span>{match item.len.as_str() { "short" => "Кратко", "long" => "Подробно", _ => "Обычно" }}</span></div>
                                            <button class="lab-mini" type="button" class:on=move || ctx.cfg.get().lab.active_preset.as_deref() == Some(active_id.as_str()) on:click=move |_| ctx.cfg.update(|cfg| {
                                                cfg.len = apply.len.clone(); cfg.lab.active_preset = Some(apply.id.clone());
                                            })>"Применить"</button>
                                            <button class="lab-remove" type="button" aria-label="Удалить пресет" on:click=move |_| ctx.cfg.update(|cfg| {
                                                cfg.lab.presets.retain(|p| p.id != delete_id);
                                                if cfg.lab.active_preset.as_deref() == Some(delete_id.as_str()) { cfg.lab.active_preset = None; }
                                            })>"×"</button>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }.into_view(),
                    "vectors" => view! {
                        <div class="lab-panel">
                            <div class="lab-heading"><b>"Векторная память"</b><span class="lab-status">"Английский · активен"</span></div>
                            <p class="hint">"Для английских записей Luna строит embeddings и ищет совпадения по смыслу. Для русского текста пока используются ключевые слова и правила активации лорбуков."</p>
                            <div class="lab-info-row"><span>"Embeddings-модель"</span><b>"Supabase gte-small · 384 · EN"</b></div>
                            <div class="lab-info-row"><span>"Векторное хранилище"</span><b>"Postgres pgvector · HNSW"</b></div>
                            <div class="lab-info-row"><span>"Поиск"</span><b>"Смысловой EN · ключевой RU"</b></div>
                            <p class="hint">"Важно: gte-small обучена только для английского. Я не применяю её к русским текстам, чтобы не выдавать слабые совпадения за семантический поиск. Для полноценного RU/EN-поиска потребуется отдельная мультиязычная модель. Записи и векторы изолированы по аккаунтам."</p>
                        </div>
                    }.into_view(),
                    _ => view! {
                        <div class="lab-panel">
                            <div class="lab-heading"><b>"Плагины"</b><span class="lab-status">"Не подключены"</span></div>
                            <p class="hint">"Плагинов с реальным серверным исполнением пока нет. Здесь появится каталог интеграций с описанием разрешений, подтверждением действий и журналом вызовов."</p>
                            <div class="lab-info-row"><span>"Реестр инструментов"</span><b>"Не настроен"</b></div>
                            <div class="lab-info-row"><span>"Разрешения пользователя"</span><b>"Не настроены"</b></div>
                            <div class="lab-info-row"><span>"Подтверждение действий"</span><b>"Не реализовано"</b></div>
                            <p class="hint">"Никакие внешние действия не будут выполняться без явного разрешения и серверной проверки."</p>
                        </div>
                    }.into_view(),
                }}
            </div>
        </section>
    }
}

#[component]
fn PageMemory() -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let items = create_rw_signal(Vec::<Memory>::new());
    let loading = create_rw_signal(false);
    let error = create_rw_signal(String::new());

    create_effect(move |_| {
        if ctx.page.get() == Page::Memory && ctx.me.get().is_some() {
            loading.set(true);
            error.set(String::new());
            spawn_local(async move {
                match api::memories().await {
                    Ok(rows) => items.set(rows),
                    Err(_) => error.set("Не удалось загрузить память. Проверьте подключение и попробуйте снова.".into()),
                }
                loading.set(false);
            });
        }
    });

    let category_name = |category: &str| -> &'static str {
        match category {
            "character" => "Персонажи",
            "relationship" => "Отношения",
            "world" => "Мир",
            "event" => "События",
            "preference" => "Предпочтения",
            "correction" => "Исправления",
            "open_thread" => "Незавершённые линии",
            _ => "Другое",
        }
    };

    view! {
        <section class=pgc(ctx, Page::Memory)>
            {ph(ctx, || "Память Luna".into(), move || ctx.open_page(Page::Main))}
            <div class="sc memory-sc">
                <p class="hint">"Здесь хранятся подтверждённые факты о персонажах, отношениях и событиях. Закреплённые записи имеют приоритет. Память можно исправить или удалить."</p>
                {move || if ctx.me.get().is_none() {
                    view! { <div class="grp"><p class="hint">"Войдите в аккаунт, чтобы просматривать сохранённую память."</p></div> }.into_view()
                } else if loading.get() && items.get().is_empty() {
                    view! { <p class="hint">"Загружаем память…"</p> }.into_view()
                } else if !error.get().is_empty() {
                    view! {
                        <div class="grp">
                            <p class="hint">{move || error.get()}</p>
                            <button class="fi rst" on:click=move |_| {
                                loading.set(true);
                                spawn_local(async move {
                                    match api::memories().await {
                                        Ok(rows) => { items.set(rows); error.set(String::new()); }
                                        Err(_) => error.set("Не удалось загрузить память.".into()),
                                    }
                                    loading.set(false);
                                });
                            }>"Повторить"</button>
                        </div>
                    }.into_view()
                } else if items.get().is_empty() {
                    view! { <div class="grp"><p class="hint">"Пока нет сохранённых воспоминаний. Важные подтверждённые факты Luna будет добавлять автоматически после разговоров."</p></div> }.into_view()
                } else {
                    view! {
                        <div class="mem-list">
                            <For
                                each=move || items.get()
                                key=|m| m.id.clone()
                                children=move |m| {
                                    let id = m.id.clone();
                                    let save_id = id.clone();
                                    let pin_id = id.clone();
                                    let delete_id = id.clone();
                                    let edit = create_rw_signal(m.content.clone());
                                    let busy = create_rw_signal(false);
                                    let pinned = create_rw_signal(m.is_pinned);
                                    let category = category_name(&m.category).to_string();
                                    let scope = if m.chat_id.is_some() { "Этот чат" } else { "Общая память" };
                                    view! {
                                        <div class="grp mem-item">
                                            <div class="mem-meta">
                                                <span>{category}</span>
                                                <span>{scope}</span>
                                                <span>{format!("Важность {}", m.importance)}</span>
                                            </div>
                                            <textarea class="fi mem-edit" prop:value=move || edit.get()
                                                on:input=move |e| edit.set(event_target_value(&e))></textarea>
                                            <div class="mem-actions">
                                                <button class="glass" disabled=move || busy.get() on:click=move |_| {
                                                        let id = save_id.clone();
                                                        busy.set(true);
                                                        let value=edit.get_untracked();
                                                        spawn_local(async move {
                                                            match api::update_memory(&id, &serde_json::json!({"content":value})).await {
                                                                Ok(saved) => { items.update(|all| if let Some(row)=all.iter_mut().find(|x| x.id==saved.id) { *row=saved; }); ctx.say("Память обновлена."); }
                                                                Err(_) => ctx.say("Не удалось сохранить запись."),
                                                            }
                                                            busy.set(false);
                                                        });
                                                    }>"Сохранить"</button>
                                                <button class="glass" disabled=move || busy.get() on:click=move |_| {
                                                        let id = pin_id.clone();
                                                        busy.set(true);
                                                        spawn_local(async move {
                                                            match api::update_memory(&id, &serde_json::json!({"is_pinned":!pinned.get_untracked()})).await {
                                                                Ok(saved) => {
                                                                    pinned.set(saved.is_pinned);
                                                                    items.update(|all| if let Some(row)=all.iter_mut().find(|x| x.id==saved.id) { *row=saved; });
                                                                },
                                                                Err(_) => ctx.say("Не удалось изменить закрепление."),
                                                            }
                                                            busy.set(false);
                                                        });
                                                    }>{move || if pinned.get() { "Открепить" } else { "Закрепить" }}</button>
                                                <button class="glass mem-delete" disabled=move || busy.get() on:click=move |_| {
                                                        let id = delete_id.clone();
                                                        busy.set(true);
                                                        spawn_local(async move {
                                                            if api::delete_memory(&id).await {
                                                                items.update(|all| all.retain(|row| row.id != id));
                                                            } else { ctx.say("Не удалось удалить запись."); }
                                                            busy.set(false);
                                                        });
                                                    }>"Удалить"</button>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    }.into_view()
                }}
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
            <div class="sc about-sc">
                <div class="about-content">
                    <h2>"Место для общения и историй"</h2>
                    <p class="about-lead">"Luna AI — приложение, в котором можно создавать персонажей, придумывать сюжеты и вести собственные диалоги."</p>

                    <section class="about-section">
                        <h3>"Что такое Luna AI"</h3>
                        <p>"Это пространство для общения, творчества и ролевых историй. Вы задаёте направление, создаёте героев и развиваете сюжет в собственном темпе."</p>
                    </section>

                    <section class="about-section">
                        <h3>"Авторский проект"</h3>
                        <p>"Luna — самостоятельный авторский проект, который я развиваю лично. В работе использую AI-инструменты и подход вайб-кодинга: превращаю идеи в функции и постепенно совершенствую приложение."</p>
                    </section>

                    <section class="about-section">
                        <h3>"Развитие и качество"</h3>
                        <p>"Luna постоянно развивается. Поэтому иногда возможны ошибки, недоработки или временные сбои. Я стараюсь оперативно исправлять проблемы, однако не могу гарантировать полностью бесперебойную работу."</p>
                    </section>

                    <section class="about-section about-support-section">
                        <h3>"Поддержать проект"</h3>
                        <p>"Если вам нравится Luna и вы хотите поддержать её развитие, подписывайтесь на мой Telegram-канал."</p>
                        <a class="cta" href="https://t.me/LunaAiroll" target="_blank" rel="noopener noreferrer">"Подписаться на Telegram-канал"</a>
                    </section>
                </div>
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
                    <div class="grp doc">
                        <div class="doc-head">
                            {rb(ctx.mi.get_untracked())}
                            <div><b>{d.t.clone()}</b><span>{format!("Редакция от {}", data().lg_date)}</span></div>
                        </div>
                        <div class="doc-content">{d.s.iter().map(|(h, t)| view! { <section class="doc-section"><b>{h.clone()}</b>{t.split("\n\n").filter(|paragraph| !paragraph.trim().is_empty()).map(|paragraph| view! { <p>{paragraph.trim().to_string()}</p> }).collect_view()}</section> }).collect_view()}</div>
                    </div>
                }.into_view())}
            </div>
        </section>
    }
}

// ───────────── лента сообщений ─────────────

fn row_view(ctx: Ctx, r: Row, anim: RwSignal<Option<usize>>, last_ai_id: Memo<Option<u64>>) -> View {
    if r.user {
        return view! { <div class="msg me">{r.text}</div> }.into_view();
    }
    let k = ctx.mi.get_untracked();
    if r.fx == 0 {
        let id = r.id;
        return view! {
            <div class="msg ai">
                <div class="glass av">
                    <svg viewBox="0 0 100 100"
                         class=move || if last_ai_id.get() == Some(id) {
                             match anim.get() { Some(a) => format!("rb a{a}"), None => "rb".to_string() }
                         } else { "rb".to_string() }
                         prop:innerHTML=move || if last_ai_id.get() == Some(id) {
                             data().rb[ctx.mi.get()].clone()
                         } else { data().rb[k].clone() }></svg>
                </div>
                <div class="tx">{r.text}</div>
            </div>
        }.into_view();
    }
    let art = if r.fx == 1 { data().rb_stop.clone() } else { data().rb_lim.clone() };
    let plan = data().models[k].n.clone();
    view! {
        <div class="msg ai">
            <div class="glass av">
                <svg viewBox="0 0 100 100"
                     class=move || if last_ai_id.get() == Some(r.id) {
                         match anim.get() { Some(a) => format!("rb a{a}"), None => "rb".to_string() }
                     } else { "rb".to_string() }
                     prop:innerHTML=move || if last_ai_id.get() == Some(r.id) {
                         data().rb[ctx.mi.get()].clone()
                     } else { data().rb[k].clone() }></svg>
            </div>
            <div class="tx">
                <div class="rep">
                    {(!r.part.is_empty()).then(|| view! { <div>{r.part.clone()}</div> })}
                    <div class="glass big"><svg class="rb a1" viewBox="0 0 100 100" inner_html=art></svg></div>
                    <div>{r.text.clone()}</div>
                    {(r.fx == 2 || r.fx == 3).then(|| view! {
                        <div class="grp lim">
                            <div class="row"><span class="rt">"Статус"</span><span class="rv">{if r.fx == 3 { "Промт длиннее лимита" } else { "Лимит превышен" }}</span></div>
                            <div class="row"><span class="rt">"Тариф"</span><span class="rv">{if r.fx == 3 { "Бесплатный".to_string() } else { plan.clone() }}</span></div>
                            {if r.fx == 3 {
                                view! { <button class="cta" type="button" on:click=move |_| ctx.open_page(Page::Plan)>"Посмотреть тарифы"</button> }.into_view()
                            } else {
                                view! { <div class="row"><span class="rt">"Что делать"</span><span class="rv">"Подождать немного"</span></div> }.into_view()
                            }}
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
    let last_model = create_rw_signal(ctx.mi.get_untracked());
    create_effect(move |_| {
        let selected = ctx.mi.get();
        if selected != last_model.get_untracked() {
            last_model.set(selected);
            anim.set(Some(selected));
            set_timeout(move || {
                if anim.get_untracked() == Some(selected) {
                    anim.set(None);
                }
            }, Duration::from_millis(1100));
        }
    });
    let streaming = create_memo(move |_| ctx.stream.with(|s| s.is_some()));
    let blank = create_memo(move |_| ctx.stream.with(|s| s.as_ref().map_or(true, |t| t.is_empty())));
    let dots_visible = create_rw_signal(true);
    let dots_leaving = create_rw_signal(false);
    create_effect(move |_| {
        if blank.get() {
            dots_visible.set(true);
            dots_leaving.set(false);
        } else if dots_visible.get_untracked() {
            dots_leaving.set(true);
            set_timeout(move || {
                if !blank.get_untracked() {
                    dots_visible.set(false);
                    dots_leaving.set(false);
                }
            }, Duration::from_millis(190));
        }
    });
    let empty = create_memo(move |_| ctx.log.with(|l| l.is_empty()) && !streaming.get());
    let empty_visible = create_rw_signal(ctx.log.get_untracked().is_empty());
    let empty_leaving = create_rw_signal(false);
    create_effect(move |_| {
        if empty.get() {
            empty_visible.set(true);
            empty_leaving.set(false);
        } else if empty_visible.get_untracked() {
            empty_leaving.set(true);
            set_timeout(move || {
                if !empty.get_untracked() {
                    empty_visible.set(false);
                    empty_leaving.set(false);
                }
            }, Duration::from_millis(260));
        }
    });
    let last_ai_id = create_memo(move |_| ctx.log.with(|l| l.iter().rev().find(|r| !r.user).map(|r| r.id)));
    let log_el = ctx.log_el;
    let touch_start_y = store_value(0.0f64);
    let near_bottom = create_rw_signal(true);
    let show_jump = create_rw_signal(false);
    let scroll_bottom = move |smooth: bool| {
        request_animation_frame(move || {
            if let Some(el) = log_el.get_untracked() {
                let o = web_sys::ScrollToOptions::new();
                o.set_top(el.scroll_height() as f64);
                o.set_behavior(if smooth { web_sys::ScrollBehavior::Smooth } else { web_sys::ScrollBehavior::Auto });
                el.scroll_to_with_scroll_to_options(&o);
            }
        });
    };
    create_effect(move |_| {
        let _ = ctx.log.get();
        if near_bottom.get_untracked() {
            show_jump.set(false);
            scroll_bottom(true);
        } else {
            show_jump.set(true);
        }
    });
    create_effect(move |_| {
        let stream_now = ctx.stream.get();
        if stream_now.is_some() && near_bottom.get_untracked() {
            scroll_bottom(false);
        }
    });

    view! {
        <main id="log" node_ref=log_el
              on:pointerdown=move |e: web_sys::PointerEvent| touch_start_y.set_value(e.client_y() as f64)
              on:pointerup=move |e: web_sys::PointerEvent| {
                  if e.client_y() as f64 - touch_start_y.get_value() > 55.0 {
                      if let Some(active) = document().get_element_by_id("inp").and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok()) {
                          let _ = active.blur();
                      }
                  }
              }
              on:click=move |_| {
                  if document().document_element().map_or(false, |el| el.class_list().contains("keyboard-open")) {
                      if let Some(active) = document().get_element_by_id("inp").and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok()) {
                          active.blur();
                      }
                  }
              }
              on:scroll=move |_| {
                  if let Some(el) = log_el.get_untracked() {
                      let distance = el.scroll_height() - el.scroll_top() - el.client_height();
                      let at_bottom = distance <= 80;
                      near_bottom.set(at_bottom);
                      show_jump.set(!at_bottom);
                  }
              }>
            <div id="messages-content">
            <Show when=move || empty_visible.get()>
                <div class="empty" class:leaving=move || empty_leaving.get()>
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
            <For each=move || ctx.log.get() key=|r| r.id children=move |r| row_view(ctx, r, anim, last_ai_id)/>
            <Show when=move || streaming.get()>
                <div class="msg ai">
                    <div class="glass av">{move || rb(ctx.mi.get())}</div>
                    <div class="tx">
                        <Show when=move || dots_visible.get()>
                            <span class="dots" class:leaving=move || dots_leaving.get()><i></i><i></i><i></i></span>
                        </Show>
                        <span class="stream-text" class:active=move || !blank.get()>
                            <For
                                each=move || { ctx.stream.get().unwrap_or_default().chars().enumerate().collect::<Vec<_>>() }
                                key=|(i, _)| *i
                                children=move |(_, ch)| view! { <span class="stream-char">{ch.to_string()}</span> }
                            />
                        </span>
                    </div>
                </div>
            </Show>
            </div>
            <button class="jump glass" class:visible=move || show_jump.get()
                    aria-label="К последнему сообщению"
                    on:click=move |_| { near_bottom.set(true); show_jump.set(false); scroll_bottom(true); }>
                "↓"
            </button>
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
            // pointerdown на кнопке отменён: textarea сохраняет фокус,
            // поэтому iOS Safari не закрывает клавиатуру после отправки.
            request_animation_frame(fit);
            request_animation_frame(move || {
                if let Some(t) = ta.get_untracked() {
                    let t: &web_sys::HtmlElement = t.unchecked_ref();
                    let options = web_sys::FocusOptions::new();
                    options.set_prevent_scroll(true);
                    let _ = t.focus_with_options(&options);
                }
            });
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
                          on:focus=move |_| {
                              // Do not scroll the document: Safari resizes the visual viewport.
                              // The message list remains the only scrollable chat region.
                              request_animation_frame(move || ctx.scroll(false));
                          }
                          on:blur=move |_| {
                              // #app follows visualViewport resize/scroll events.
                          }></textarea>
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
