//! Donkr: the dating app for donkeys.
//!
//! One Rust binary (Axum + Maud) renders every page, SQLite stores the herd and the chats,
//! and a few lines of vanilla JS in `public/app.js` make carrots and messages feel instant.

use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;

/// The signed-in donkey. Every demo needs a protagonist.
const ME: &str = "burrito";

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, Serialize)]
struct Donkey {
    id: String,
    name: String,
    age: i64,
    tribe: String,
    distance_tenths: i64,
    online: bool,
    fresh: bool,
    headline: String,
    bio: String,
    looking_for: String,
    height_hands: i64,
    favourite_snack: String,
    carrots: i64,
}

impl Donkey {
    fn distance(&self) -> String {
        format!("{}.{} mi", self.distance_tenths / 10, self.distance_tenths % 10)
    }
    fn photo(&self) -> String {
        format!("/donkeys/{}.jpg", self.id)
    }
}

#[derive(Debug, Clone, Serialize)]
struct Message {
    from_me: bool,
    body: String,
    sent_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Recruit {
    handle: String,
    photo: String,
    caption: String,
    likes: i64,
    comments: i64,
}

// ---------------------------------------------------------------------------------------------
// Database ("Studbase")
// ---------------------------------------------------------------------------------------------

fn root() -> PathBuf {
    env::current_dir()
        .ok()
        .filter(|d| d.join("public").is_dir())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn open_db() -> Connection {
    let dir = root().join("data");
    std::fs::create_dir_all(&dir).expect("create data dir");
    let conn = Connection::open(dir.join("studbase.db")).expect("open studbase");
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS donkeys (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, age INTEGER NOT NULL, tribe TEXT NOT NULL,
            distance_tenths INTEGER NOT NULL, online INTEGER NOT NULL, fresh INTEGER NOT NULL,
            headline TEXT NOT NULL, bio TEXT NOT NULL, looking_for TEXT NOT NULL,
            height_hands INTEGER NOT NULL, favourite_snack TEXT NOT NULL, carrots INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT, donkey_id TEXT NOT NULL, from_me INTEGER NOT NULL,
            body TEXT NOT NULL, sent_at TEXT NOT NULL DEFAULT (strftime('%H:%M', 'now', 'localtime'))
         );
         CREATE TABLE IF NOT EXISTS carrots_sent (donkey_id TEXT PRIMARY KEY);",
    )
    .expect("create schema");
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM donkeys", [], |r| r.get(0)).unwrap();
    if count == 0 {
        seed(&conn);
    }
    conn
}

fn seed(conn: &Connection) {
    // id, name, age, tribe, distance (tenths of a mile), online, fresh, headline, bio, looking for, hands, snack
    let herd: &[(&str, &str, i64, &str, i64, bool, bool, &str, &str, &str, i64, &str)] = &[
        (ME, "Burrito", 7, "Jack", 0, true, false, "It's me. Hi.", "Wrapped up in my own thoughts. Big on long walks, bigger on snacks.", "Someone to share a hay bale with", 11, "Carrots (obviously)"),
        ("gerald", "Gerald", 7, "Jack", 3, true, false, "Red halter, green flags only", "Dressed up for you. Halter optional on the second date.", "A Jenny who can keep up on trails", 12, "Apple slices"),
        ("maurice", "Maurice", 9, "Mini", 5, true, false, "Small donkey, big personality", "I'm 8 hands tall and I make every one of them count.", "Someone who doesn't mind looking down", 8, "Peppermints"),
        ("dorian", "Dorian", 5, "Shaggy", 8, true, false, "Winter coat all year", "I don't brush. I'm told it's a vibe.", "Cuddles in the barn when it rains", 11, "Oat hay"),
        ("clive", "Clive", 11, "Silver jack", 11, true, false, "Distinguished. Grey. Patient.", "Retired from the pack-trail business. Now I mostly stand majestically.", "A slow burn", 11, "Bran mash"),
        ("pancho", "Pancho", 6, "Jack", 12, true, false, "Wildflower enthusiast", "Will stop to smell every single flower on the walk. Every one.", "A meadow and a muse", 10, "Clover"),
        ("biscuit", "Biscuit", 4, "Jenny", 16, true, false, "Soft face, strong opinions", "Golden hour is my good side. Every hour is golden hour.", "Someone who brings treats unprompted", 10, "Ginger biscuits"),
        ("theo", "Theo", 8, "Spotted", 19, true, false, "Yes, the spots are real", "Pinto, not painted. Ask me about my patches.", "A fence-side conversation", 12, "Watermelon rind"),
        ("ferdinand", "Ferdinand", 12, "Mammoth", 20, true, false, "Big ears. Bigger heart.", "Mammoth jack. I can hear you thinking about me from two fields away.", "A Jenny who likes them tall", 15, "Sugar beet"),
        ("waffles", "Waffles", 2, "Foal", 23, false, true, "New in town 🐣", "Just got my first halter. Everything is exciting. EVERYTHING.", "Friends (I'm two)", 7, "Milk"),
        ("nacho", "Nacho", 6, "Jack", 24, false, true, "Desert donkey, cool head", "Wears a hat for shade, not for fashion. Okay, a little for fashion.", "A road trip partner", 11, "Cactus pear"),
        ("reginald", "Reginald", 10, "Silver jack", 27, false, false, "Ever so pleased to meet you", "Hedgerow connoisseur. I have opinions about brambles.", "Proper courtship. Letters. Haybales.", 11, "Digestives"),
        ("pickles", "Pickles", 5, "Jenny", 29, true, true, "Flower crown, no crown attitude", "Spring is my season. Also summer. I am a seasonal donkey.", "Someone to make daisy chains with", 10, "Strawberries"),
        ("hamish", "Hamish", 9, "Shaggy", 33, false, false, "Highland hills, highland chills", "I live on a misty hill and I'm emotionally available.", "A fellow hill walker", 11, "Neeps"),
        ("toast", "Toast", 4, "Jack", 35, true, false, "Caught mid-yawn, still cute", "Morning person? No. Afternoon person? Also no. Snack person? Yes.", "Naps, together", 10, "Bread crusts"),
        ("brayson", "Sir Brayson", 14, "Mammoth", 41, false, false, "Groomed. Regal. Single.", "Champion at three county shows. Currently champion of your heart (pending).", "A Jenny of standing", 15, "Molasses cubes"),
    ];
    for d in herd {
        conn.execute(
            "INSERT INTO donkeys (id, name, age, tribe, distance_tenths, online, fresh, headline, bio, looking_for, height_hands, favourite_snack, carrots)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![d.0, d.1, d.2, d.3, d.4, d.5, d.6, d.7, d.8, d.9, d.10, d.11, (d.4 * 7 + 13) % 90],
        )
        .expect("seed donkey");
    }
    let chats: &[(&str, bool, &str)] = &[
        ("gerald", false, "nice halter"),
        ("gerald", true, "thanks, it's load-bearing"),
        ("gerald", false, "hee-haw 👀"),
        ("biscuit", false, "are those your real ears"),
        ("biscuit", true, "unfortunately yes"),
        ("ferdinand", false, "u up? (I can hear that you are)"),
    ];
    for (id, mine, body) in chats {
        conn.execute("INSERT INTO messages (donkey_id, from_me, body) VALUES (?1, ?2, ?3)", params![id, mine, body])
            .expect("seed chat");
    }
}

fn row_to_donkey(r: &rusqlite::Row) -> rusqlite::Result<Donkey> {
    Ok(Donkey {
        id: r.get(0)?,
        name: r.get(1)?,
        age: r.get(2)?,
        tribe: r.get(3)?,
        distance_tenths: r.get(4)?,
        online: r.get(5)?,
        fresh: r.get(6)?,
        headline: r.get(7)?,
        bio: r.get(8)?,
        looking_for: r.get(9)?,
        height_hands: r.get(10)?,
        favourite_snack: r.get(11)?,
        carrots: r.get(12)?,
    })
}

const DONKEY_COLS: &str = "id, name, age, tribe, distance_tenths, online, fresh, headline, bio, looking_for, height_hands, favourite_snack, carrots";

fn herd(conn: &Connection, filter: &str) -> Vec<Donkey> {
    let where_clause = match filter {
        "" | "online" => "WHERE online = 1 OR id = ?1",
        "jacks" => "WHERE tribe LIKE '%jack%' OR id = ?1",
        "fresh" => "WHERE fresh = 1 OR id = ?1",
        _ => "WHERE 1 = 1 OR id = ?1",
    };
    let sql = format!("SELECT {DONKEY_COLS} FROM donkeys {where_clause} ORDER BY id != ?1, distance_tenths");
    let mut stmt = conn.prepare(&sql).unwrap();
    stmt.query_map(params![ME], row_to_donkey).unwrap().filter_map(Result::ok).collect()
}

fn one(conn: &Connection, id: &str) -> Option<Donkey> {
    conn.query_row(&format!("SELECT {DONKEY_COLS} FROM donkeys WHERE id = ?1"), params![id], row_to_donkey)
        .optional()
        .unwrap()
}

fn messages(conn: &Connection, id: &str) -> Vec<Message> {
    let mut stmt = conn.prepare("SELECT from_me, body, sent_at FROM messages WHERE donkey_id = ?1 ORDER BY id").unwrap();
    stmt.query_map(params![id], |r| Ok(Message { from_me: r.get(0)?, body: r.get(1)?, sent_at: r.get(2)? }))
        .unwrap()
        .filter_map(Result::ok)
        .collect()
}

fn recruits() -> Vec<Recruit> {
    let path = root().join("data/recruits.json");
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// Every donkey speaks fluent donkey. Replies are picked from the message so chats feel alive.
fn bray_back(body: &str) -> String {
    let lines = [
        "hee-haw 😳",
        "HEE. HAW.",
        "*flicks ears flirtatiously*",
        "only if you bring carrots",
        "sorry I was eating. what did you say",
        "my owner says I can't text after 9",
        "is that a hay bale in your pocket",
        "brb rolling in dust",
    ];
    let pick = body.bytes().fold(7usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
    lines[pick % lines.len()].to_string()
}

// ---------------------------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------------------------

#[derive(PartialEq)]
enum Tab {
    Grid,
    Inbox,
    Recruits,
    Me,
}

fn page(title: &str, tab: Tab, body: Markup) -> Html<String> {
    Html(
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (title) " · Donkr" }
                    link rel="icon" href="/logo.svg";
                    link rel="stylesheet" href="/app.css";
                }
                body {
                    div.phone {
                        div.statusbar { span { "9:41" } span.notch {} span.icons { "▂▄▆ ◔" } }
                        div.rainbow {}
                        main.screen { (body) }
                        nav.tabs {
                            a href="/" class=[(tab == Tab::Grid).then_some("on")] { span { "▦" } "Grid" }
                            a href="/inbox" class=[(tab == Tab::Inbox).then_some("on")] { span { "💬" } "Inbox" }
                            a href="/recruits" class=[(tab == Tab::Recruits).then_some("on")] { span { "📣" } "Recruits" }
                            a href=(format!("/d/{ME}")) class=[(tab == Tab::Me).then_some("on")] { span { "🫏" } "Me" }
                        }
                    }
                    script src="/app.js" {}
                }
            }
        }
        .into_string(),
    )
}

fn header(title: &str, sub: Option<&str>) -> Markup {
    html! {
        header.top {
            div.brand {
                img.logo src="/logo.svg" alt="";
                div { h1 { (title) } @if let Some(s) = sub { p.sub { (s) } } }
            }
            a.gear href="/about" title="About Donkr" { "⚙" }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pages
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct GridQuery {
    f: Option<String>,
}

async fn grid(State(s): State<AppState>, Query(q): Query<GridQuery>) -> impl IntoResponse {
    let filter = q.f.unwrap_or_default();
    let donkeys = herd(&s.db.lock().unwrap(), &filter);
    let chip = |key: &str, label: &str| {
        let on = filter == key || (key == "online" && filter.is_empty());
        let href = if key == "online" { "/".to_string() } else { format!("/?f={key}") };
        html! { a.chip.on[on] href=(href) { (label) } }
    };
    page(
        "Grid",
        Tab::Grid,
        html! {
            (header("Donkr", None))
            div.chips { (chip("online", "Online now")) (chip("jacks", "Jacks nearby")) (chip("fresh", "Fresh hay")) }
            div.grid {
                @for d in &donkeys {
                    a.tile href=(format!("/d/{}", d.id)) {
                        img src=(d.photo()) alt=(d.name) loading="lazy";
                        @if d.id == ME { span.you { "You" } } @else { span.dist { (d.distance()) } }
                        @if d.fresh { span.fresh { "new here" } }
                        span.name { @if d.online { i.dot {} } (d.name) ", " (d.age) }
                    }
                }
            }
        },
    )
}

async fn profile(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let db = s.db.lock().unwrap();
    let Some(d) = one(&db, &id) else { return not_found().into_response() };
    let carroted: bool = db.query_row("SELECT 1 FROM carrots_sent WHERE donkey_id = ?1", params![id], |_| Ok(true)).optional().unwrap().unwrap_or(false);
    let me = d.id == ME;
    page(
        &d.name,
        if me { Tab::Me } else { Tab::Grid },
        html! {
            div.hero {
                img src=(d.photo()) alt=(d.name);
                a.back href="/" { "‹" }
                div.hero-text {
                    h1 { @if d.online { i.dot {} } (d.name) ", " (d.age) }
                    p { (d.headline) }
                }
            }
            div.card {
                div.stats {
                    div { b { (d.tribe) } span { "Tribe" } }
                    div { b { (d.height_hands) " hh" } span { "Height" } }
                    div { b { @if me { "here" } @else { (d.distance()) } } span { "Away" } }
                    div { b data-carrots=(d.id) { (d.carrots) } span { "🥕 received" } }
                }
                h3 { "About" }
                p { (d.bio) }
                h3 { "Looking for" }
                p { (d.looking_for) }
                h3 { "Favourite snack" }
                p { (d.favourite_snack) }
                @if !me {
                    div.actions {
                        button.carrot.sent[carroted] data-id=(d.id) { @if carroted { "🥕 Carrot sent" } @else { "🥕 Send a carrot" } }
                        a.btn.chat href=(format!("/chat/{}", d.id)) { "💬 Chat" }
                    }
                } @else {
                    p.note { "This is you. You look great. Very donkey." }
                }
            }
        },
    )
    .into_response()
}

async fn carrot(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let db = s.db.lock().unwrap();
    if one(&db, &id).is_none() || id == ME {
        return StatusCode::NOT_FOUND.into_response();
    }
    let fresh = db.execute("INSERT OR IGNORE INTO carrots_sent (donkey_id) VALUES (?1)", params![id]).unwrap() == 1;
    if fresh {
        db.execute("UPDATE donkeys SET carrots = carrots + 1 WHERE id = ?1", params![id]).unwrap();
    }
    let carrots: i64 = db.query_row("SELECT carrots FROM donkeys WHERE id = ?1", params![id], |r| r.get(0)).unwrap();
    Json(serde_json::json!({ "carrots": carrots })).into_response()
}

async fn chat(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let db = s.db.lock().unwrap();
    let Some(d) = one(&db, &id) else { return not_found().into_response() };
    if d.id == ME {
        return Redirect::to("/inbox").into_response();
    }
    let msgs = messages(&db, &id);
    page(
        &format!("Chat with {}", d.name),
        Tab::Inbox,
        html! {
            header.chat-top {
                a.back href="/inbox" { "‹" }
                img.avatar src=(d.photo()) alt="";
                div { h2 { (d.name) } p.sub { @if d.online { "Online now · " } (d.distance()) " away" } }
            }
            div.thread #thread {
                @if msgs.is_empty() { p.empty { "Say something. Donkeys love a bold opener." } }
                @for m in &msgs {
                    div.bubble.me[m.from_me] { (m.body) span.time { (m.sent_at) } }
                }
            }
            form.composer method="post" action=(format!("/chat/{}", d.id)) data-id=(d.id) {
                input name="body" placeholder="Hee-haw…" autocomplete="off" maxlength="280" required;
                button { "Send" }
            }
        },
    )
    .into_response()
}

#[derive(Deserialize)]
struct NewMessage {
    body: String,
}

fn store_message(db: &Connection, id: &str, body: &str) -> Option<(Message, Message)> {
    let body = body.trim();
    if body.is_empty() || one(db, id).is_none() || id == ME {
        return None;
    }
    db.execute("INSERT INTO messages (donkey_id, from_me, body) VALUES (?1, 1, ?2)", params![id, body]).unwrap();
    let reply = bray_back(body);
    db.execute("INSERT INTO messages (donkey_id, from_me, body) VALUES (?1, 0, ?2)", params![id, reply]).unwrap();
    let mut last = messages(db, id);
    let r = last.pop()?;
    let m = last.pop()?;
    Some((m, r))
}

async fn send(State(s): State<AppState>, Path(id): Path<String>, Form(f): Form<NewMessage>) -> impl IntoResponse {
    let db = s.db.lock().unwrap();
    store_message(&db, &id, &f.body);
    Redirect::to(&format!("/chat/{id}"))
}

async fn send_json(State(s): State<AppState>, Path(id): Path<String>, Json(f): Json<NewMessage>) -> impl IntoResponse {
    let db = s.db.lock().unwrap();
    match store_message(&db, &id, &f.body) {
        Some((mine, reply)) => Json(serde_json::json!({ "mine": mine, "reply": reply })).into_response(),
        None => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn inbox(State(s): State<AppState>) -> impl IntoResponse {
    let db = s.db.lock().unwrap();
    let mut stmt = db
        .prepare(&format!(
            "SELECT {}, m.body, m.sent_at FROM donkeys d JOIN messages m ON m.id = (SELECT MAX(id) FROM messages WHERE donkey_id = d.id) ORDER BY m.id DESC",
            DONKEY_COLS.split(", ").map(|c| format!("d.{c}")).collect::<Vec<_>>().join(", ")
        ))
        .unwrap();
    let rows: Vec<(Donkey, String, String)> = stmt
        .query_map([], |r| Ok((row_to_donkey(r)?, r.get(13)?, r.get(14)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    page(
        "Inbox",
        Tab::Inbox,
        html! {
            (header("Inbox", Some("Hay there. Your conversations.")))
            ul.inbox {
                @for (d, last, at) in &rows {
                    li { a href=(format!("/chat/{}", d.id)) {
                        img.avatar src=(d.photo()) alt="";
                        div { b { @if d.online { i.dot {} } (d.name) } span { (last) } }
                        time { (at) }
                    } }
                }
                @if rows.is_empty() { li.empty { "No chats yet. Go send a carrot." } }
            }
        },
    )
}

async fn recruits_page() -> impl IntoResponse {
    let list = recruits();
    page(
        "Recruits",
        Tab::Recruits,
        html! {
            (header("Recruits", Some("Donkeys we want on Donkr")))
            p.pitch { "Let's get some asses in here. Top posts from #donkeysofinstagram, ranked by likes." }
            div.recruits {
                @for r in &list {
                    div.recruit {
                        img src=(r.photo) alt="";
                        div.meta {
                            b { (r.handle) }
                            p { (r.caption) }
                            span { "♥ " (r.likes) "  ·  💬 " (r.comments) }
                        }
                        button.invite { "Invite" }
                    }
                }
            }
            p.fine { "Sample data. Run " code { "scripts/recruit.mjs" } " with your own Apify token to fetch real posts." }
        },
    )
}

async fn about() -> impl IntoResponse {
    page(
        "About",
        Tab::Me,
        html! {
            (header("About Donkr", Some("Find your stable relationship")))
            div.card.about {
                p { "Donkr is a location-based dating app for donkeys. Jacks, Jennies, minis and mammoths welcome." }
                h3 { "Built with" }
                ul { li { "Rust (Axum + Maud)" } li { "Studbase™ (it's SQLite)" } li { "~60 lines of vanilla JS" } }
                h3 { "Studbase benchmark" }
                table.bench {
                    tr { th { "" } th { "Supabase" } th { "Studbase" } }
                    tr { td { "Throughput" } td { "1x" } td { "11.8x" } }
                    tr { td { "p99 latency" } td { "1x" } td { "−87%" } }
                    tr { td { "Vibes" } td { "fine" } td { "immaculate" } }
                }
                p.fine { (PreEscaped("Benchmark methodology: trust me bro.")) }
            }
        },
    )
}

fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        page("Not found", Tab::Grid, html! { (header("404", Some("This donkey has wandered off"))) p.pitch { "Nothing here but hay. " a href="/" { "Back to the grid" } } }),
    )
}

async fn fallback() -> impl IntoResponse {
    not_found()
}

#[tokio::main]
async fn main() {
    let state = AppState { db: Arc::new(Mutex::new(open_db())) };
    let app = Router::new()
        .route("/", get(grid))
        .route("/d/:id", get(profile))
        .route("/d/:id/carrot", post(carrot))
        .route("/chat/:id", get(chat).post(send))
        .route("/api/chat/:id", post(send_json))
        .route("/inbox", get(inbox))
        .route("/recruits", get(recruits_page))
        .route("/about", get(about))
        .nest_service("/donkeys", ServeDir::new(root().join("public/donkeys")))
        .fallback_service(ServeDir::new(root().join("public")).fallback(get(fallback)))
        .with_state(state);
    let port = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(4180u16);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    println!("🫏 Donkr running on http://localhost:{port}");
    axum::serve(listener, app).await.unwrap();
}
