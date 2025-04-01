use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use actix_web_actors::ws;
use log::{error, info};
use redis::Commands;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize)]
struct WarData {
    tag: String,
    enemy_tag: String,
    home_score: Vec<i32>,
    enemy_score: Vec<i32>,
    diff: Vec<i32>,
    last_diff: Option<i32>,
}

#[derive(Serialize)]
struct OverlayData {
    tag: String,
    enemy_tag: String,
    score: i32,
    enemy_score: i32,
    diff: i32,
    last_diff: Option<i32>,
    race_left: i32,
}

fn query_db(channel_id: String) -> Option<OverlayData> {
    let client = match redis::Client::open("redis://localhost:6379") {
        Ok(v) => v,
        Err(e) => {
            error!(target: &channel_id, "{e}");
            return None;
        }
    };
    info!(target: &channel_id, "connected to redis");

    let mut con = match client.get_connection() {
        Ok(v) => v,
        Err(_) => return None,
    };
    info!(target: &channel_id, "connection made to redis");

    let war_data: String = match con.get(&channel_id) {
        Ok(v) => v,
        Err(e) => {
            error!(target: &channel_id, "{e}");
            return None;
        }
    };
    info!(target: &channel_id, "war data: {war_data}");

    let war_state: WarData = match serde_json::from_str(war_data.as_str()) {
        Ok(v) => v,
        Err(e) => {
            error!(target: &channel_id, "{e}");
            return None;
        }
    };
    info!(target: &channel_id, "data parsed");

    let diff: i32 = war_state.diff.iter().sum();

    let race_count = i32::try_from(war_state.diff.len()).unwrap_or(0);
    let score = race_count * 41 + diff / 2;
    let enemy_score = race_count * 41 - diff / 2;
    let last_diff = war_state.diff.iter().last().copied();
    let race_left = match 12 - race_count {
        v if v < 0 && v > -4 => v + 4,
        v if v <= -4 => 0,
        v => v,
    };

    let res = OverlayData {
        tag: war_state.tag,
        enemy_tag: war_state.enemy_tag,
        score,
        enemy_score,
        diff,
        last_diff,
        race_left,
    };

    Some(res)
}

#[get("/overlay/{channel_id}")]
async fn overlay(path: web::Path<String>) -> Result<impl Responder> {
    let channel_id = path.into_inner();
    let json_data = query_db(channel_id.clone());

    match json_data {
        Some(data) => {
            let overlay_data = OverlayData {
                tag: data.tag,
                enemy_tag: data.enemy_tag,
                score: data.score,
                enemy_score: data.enemy_score,
                diff: data.diff,
                last_diff: data.last_diff,
                race_left: data.race_left,
            };

            let class = if overlay_data.diff > 0 {
                "plus"
            } else if overlay_data.diff < 0 {
                "minus"
            } else {
                ""
            };

            let diff = if overlay_data.diff > 0 {
                format!("+{}", overlay_data.diff)
            } else {
                overlay_data.diff.to_string()
            };

            let html_response = format!(
                r#"
                <head>
                <meta charset="UTF-8">
                <meta http-equiv="Content-Type" content="text/html; charset=utf-8" />
                <style>
                    :root {{
                    --root-background: rgba(0, 0, 0, 0);
                    --t2-header-height: 50px;
                    --t2-body-height: 80px;
                    --t2-team-height: 85px;
                    --t2-space-width: 100px;
                    --t2-team-width: 150px;
                    --t2-score-width: 130px;
                    --t2-all-width: calc(var(--t2-space-width) + (var(--t2-team-width) * 2) + (var(--t2-score-width) * 2));
                    --t2-dif-width: calc(var(--t2-team-width) + var(--t2-score-width));
                    --t2-header-font: 35px;
                    --t2-score-font: 60px;
                    --t2-team-font: 50px;
                    --t2-plus-color: orangered;
                    --t2-minus-color: deepskyblue;
                    --t2-race-color: gold;
                    --t2-win-font: 35px;
                    --t2-win-background: yellow;
                    --t2-win-color: orangered;
                    }}
                    body, div, p {{
                    display: block;
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                    border: none;
                    border-radius: 0;
                    overflow: hidden;
                    white-space: nowrap;
                    text-align: center;
                    }}
                    p {{
                    display: inline-block;
                    height: 100%;
                    }}
                    body {{
                    background: var(--root-background);
                    background-size: cover;
                    background-position: center;
                    margin: 0;
                    height: 100vh;
                    display: flex;
                    flex-direction: column;
                    }}
                    .team-span {{
                    display: inline-block;
                    transform-origin: left top;
                    }}
                    .overlay-container {{
                    width: 700px;
                    height: 160px;
                    justify-content: center;
                    flex-direction: column;
                    align-items: center; 
                    flex-wrap: wrap;
                    font-weight: bold;
                    font-family: sans-serif;
                    margin-top: auto;
                    position: fixed;
                    bottom: 0;
                    left: 50%;
                    transform: translateX(-50%);
                    }}
                    .overlay-container.respect-kusaan .overlay-inner {{
                    background: rgba(0, 0, 0, 0.5);
                    color: #ffffff;
                    border-radius: 25px;
                    position: relative;
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .overlay-inner {{
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header {{
                    height: var(--t2-header-height);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header p {{
                    line-height: calc(10px + var(--t2-header-height));
                    font-size: var(--t2-header-font);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header .score-dif {{
                    width: var(--t2-dif-width);
                    text-align: right;
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header .score-dif.plus {{
                    color: var(--t2-plus-color);
                    content: "+";
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header .score-dif.minus {{
                    color: var(--t2-minus-color);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header .space {{
                    width: var(--t2-space-width);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .header .left-race {{
                    color: var(--t2-race-color);
                    width: var(--t2-dif-width);
                    text-align: left;
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body {{
                    height: var(--t2-body-height);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body p {{
                    line-height: var(--t2-body-height);
                    font-size: var(--t2-score-font);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body .team {{
                    width: var(--t2-team-width);
                    font-size: var(--t2-team-font);
                    line-height: var(--t2-team-height);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body .score {{
                    width: var(--t2-score-width);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body .score-1 {{
                    text-align: right;
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body .score-2 {{
                    text-align: left;
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .body .split {{
                    width: var(--t2-space-width);
                    }}
                    .overlay-container.respect-kusaan.team-num-2 .win {{
                    position: absolute;
                    left: 0;
                    top: 0;
                    font-size: var(--t2-win-font);
                    width: var(--t2-team-width);
                    height: var(--t2-header-height);
                    line-height: var(--t2-header-height);
                    background: var(--t2-win-background);
                    color: var(--t2-win-color);
                    }}
                </style>
                <script>
                let currentData;
                let ws;
                let previousScore = {score};
                let previousEnemyScore = {enemy_score};

                function animateNumber(element, start, end, duration) {{
                    const startTime = performance.now();
                    const updateNumber = (currentTime) => {{
                        const elapsed = currentTime - startTime;
                        const progress = Math.min(elapsed / duration, 1);
                        
                        const easeInOut = t => t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
                        const current = Math.round(start + (end - start) * easeInOut(progress));
                        
                        element.textContent = current;
                        
                        if (progress < 1) {{
                            requestAnimationFrame(updateNumber);
                        }}
                    }};
                    
                    requestAnimationFrame(updateNumber);
                }}

                function connectWebSocket() {{
                    ws = new WebSocket(`ws://${{window.location.host}}/ws/{channel_id}`);
                    
                    ws.onmessage = function(event) {{
                        const newData = JSON.parse(event.data);
                        
                        if (JSON.stringify(newData) !== JSON.stringify(currentData)) {{
                            const scoreElement = document.querySelector('.score-1');
                            const enemyScoreElement = document.querySelector('.score-2');
                            const diffElement = document.querySelector('.score-dif');
                            
                            if (newData.score !== previousScore) {{
                                animateNumber(scoreElement, previousScore, newData.score, 800);
                            }}
                            
                            if (newData.enemy_score !== previousEnemyScore) {{
                                animateNumber(enemyScoreElement, previousEnemyScore, newData.enemy_score, 800);
                            }}
                            
                            const diff = newData.diff;
                            const diffClass = diff > 0 ? 'plus' : diff < 0 ? 'minus' : '';
                            const diffText = diff > 0 ? `+${{diff}}` : diff.toString();
                            
                            diffElement.className = `score-dif ${{diffClass}}`;
                            diffElement.textContent = diffText;
                            
                            document.querySelector('.left-race').textContent = `race left: ${{newData.race_left}}`;
                            
                            previousScore = newData.score;
                            previousEnemyScore = newData.enemy_score;
                            currentData = newData;
                        }}
                    }};
                    
                    ws.onclose = function() {{
                        setTimeout(connectWebSocket, 1000);
                    }};
                }}
                
                connectWebSocket();
                </script>
                </head>
                <body>
                <div id="team-num-2" class="overlay-container respect-kusaan team-num-2">
                    <div class="overlay-inner">
                    <div class="header">
                        <p class="score-dif {}">{}</p>
                        <p class="space"></p>
                        <p class="left-race">race left: {}</p>
                    </div>
                    <div class="body">
                        <p class="team team-1"><span class="team-span">{}</span></p>
                        <p class="score score-1">{}</p>
                        <p class="split">-</p>
                        <p class="score score-2">{}</p>
                        <p class="team team-2"><span class="team-span">{}</span></p>
                    </div>
                    </div>
                </div>
                </body>
                </html>
                "#,
                class,
                diff,
                overlay_data.race_left,
                overlay_data.tag,
                overlay_data.score,
                overlay_data.enemy_score,
                overlay_data.enemy_tag,
                score = overlay_data.score,
                enemy_score = overlay_data.enemy_score
            );

            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(html_response))
        }
        None => Ok(HttpResponse::NotFound().body("Data not found")),
    }
}

#[get("/api/{channel_id}")]
async fn index(path: web::Path<String>) -> Result<impl Responder> {
    let channel_id = path.into_inner();

    Ok(web::Json(query_db(channel_id)))
}

struct WebSocketConnection {
    channel_id: String,
    hb: Instant,
}

impl Actor for WebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            if let Some(data) = query_db(act.channel_id.clone()) {
                let json = serde_json::to_string(&data).unwrap();
                ctx.text(json);
            }
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(_)) => {
                if let Some(data) = query_db(self.channel_id.clone()) {
                    let json = serde_json::to_string(&data).unwrap();
                    ctx.text(json);
                }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WebSocketConnection {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(30), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(75) {
                ctx.stop();
            } else {
                ctx.ping(b"");
            }
        });
    }
}

#[get("/ws/{channel_id}")]
async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();
    let resp = ws::start(
        WebSocketConnection {
            channel_id,
            hb: Instant::now(),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(overlay).service(ws_index))
        .bind("0.0.0.0:25991")?
        .run()
        .await
}
