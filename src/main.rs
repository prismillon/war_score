use actix_web::{get, rt, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt;
use log::{error, info};
use redis::Commands;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::interval;

#[derive(Serialize, Deserialize)]
struct WarData {
    tag: String,
    enemy_tag: String,
    home_score: Vec<f64>,
    enemy_score: Vec<f64>,
    diff: Vec<i32>,
    last_diff: Option<i32>,
    #[serde(default)]
    home_pen: i32,
    #[serde(default)]
    enemy_pen: i32,
}

#[derive(Serialize, Clone)]
struct OverlayData {
    tag: String,
    enemy_tag: String,
    score: i32,
    enemy_score: i32,
    diff: i32,
    last_diff: Option<i32>,
    race_left: i32,
    home_pen: i32,
    enemy_pen: i32,
}

fn query_db(channel_id: String) -> Option<OverlayData> {
    let client = match redis::Client::open("redis://redis:6379") {
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

    let race_count = i32::try_from(war_state.diff.len()).unwrap_or(0);
    let score = war_state.home_score.iter().sum::<f64>().round() as i32 - war_state.home_pen;
    let enemy_score =
        war_state.enemy_score.iter().sum::<f64>().round() as i32 - war_state.enemy_pen;
    let diff = score - enemy_score;
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
        home_pen: war_state.home_pen,
        enemy_pen: war_state.enemy_pen,
    };

    Some(res)
}

const OVERLAY_HEAD: &str = r##"<head>
<meta charset="UTF-8">
<title>war score</title>
<style>
@import url('https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@600;700&family=Titan+One&display=swap');

:root {
  --glass: rgba(13, 16, 23, 0.62);
  --stroke: rgba(247, 248, 244, 0.16);
  --chalk: #F7F8F4;
  --chalk-dim: rgba(247, 248, 244, 0.6);
  --lead: #FFC530;
  --trail: #5BC2FF;
  --pen: #FF5A5F;
  --ink: #10131A;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  background: transparent;
  height: 100vh;
  overflow: hidden;
}

.bug {
  position: fixed;
  bottom: 28px;
  left: 50%;
  transform: translateX(-50%);
  font-family: 'Titan One', 'Arial Rounded MT Bold', sans-serif;
  color: var(--chalk);
  animation: bug-in 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
}

@keyframes bug-in {
  from { opacity: 0; transform: translate(-50%, 14px); }
  to   { opacity: 1; transform: translate(-50%, 0); }
}

.panel {
  background: var(--glass);
  border: 1px solid var(--stroke);
  border-radius: 18px;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
  padding: 12px 26px 11px;
  transition: opacity 0.3s, filter 0.3s;
}

.main {
  display: flex;
  align-items: center;
  gap: 18px;
}

.tag {
  width: 148px;
  font-size: 30px;
  line-height: 1.2;
  text-align: center;
  overflow: hidden;
}
.tag-span {
  display: inline-block;
  white-space: nowrap;
  transform-origin: center center;
}

.score-cell { position: relative; }
.score {
  min-width: 112px;
  font-size: 52px;
  line-height: 1.1;
  text-align: center;
  font-variant-numeric: tabular-nums;
}

.pod {
  position: relative;
  min-width: 92px;
  padding: 9px 14px 7px;
  border-radius: 12px;
  font-size: 25px;
  line-height: 1;
  text-align: center;
  background: rgba(247, 248, 244, 0.12);
  transition: background 0.3s, color 0.3s;
}
.pod.plus  { background: var(--lead);  color: var(--ink); }
.pod.minus { background: var(--trail); color: var(--ink); }
.pod::before, .pod::after {
  content: "";
  position: absolute;
  top: 50%;
  margin-top: -7px;
  border: 7px solid transparent;
  opacity: 0;
  transition: opacity 0.3s;
}
.pod::before { left: -7px;  border-left-width: 0;  border-right-color: var(--lead); }
.pod::after  { right: -7px; border-right-width: 0; border-left-color: var(--trail); }
.pod.plus::before { opacity: 1; }
.pod.minus::after { opacity: 1; }

.strip {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-top: 9px;
}
.strip::before { content: ""; width: 118px; }
.pips { display: flex; gap: 5px; }
.pip {
  width: 24px;
  height: 12px;
  border-radius: 3px;
  background: rgba(247, 248, 244, 0.22);
}
.pip.spent {
  background: repeating-conic-gradient(#EDEFEA 0% 25%, #171B23 0% 50%);
  background-size: 12px 12px;
}
.pip.just { animation: pip-pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1); }
@keyframes pip-pop {
  0%   { transform: scaleY(0.2); }
  60%  { transform: scaleY(1.3); }
  100% { transform: scaleY(1); }
}

.races {
  width: 118px;
  font-family: 'Saira Condensed', 'Arial Narrow', sans-serif;
  font-weight: 700;
  font-size: 16px;
  letter-spacing: 0.14em;
  color: var(--chalk-dim);
}

.pen {
  position: absolute;
  left: 50%;
  bottom: 100%;
  transform: translateX(-50%);
  font-family: 'Saira Condensed', 'Arial Narrow', sans-serif;
  font-weight: 700;
  font-size: 15px;
  letter-spacing: 0.1em;
  line-height: 1;
  padding: 4px 10px 3px;
  border-radius: 8px;
  background: var(--pen);
  color: #fff;
  white-space: nowrap;
  box-shadow: 0 0 0 3px var(--glass);
  animation: pen-in 0.3s ease-out both;
}
.pen:empty { display: none; }
@keyframes pen-in {
  from { opacity: 0; transform: translate(-50%, 6px); }
  to   { opacity: 1; transform: translate(-50%, 0); }
}

@media (prefers-reduced-motion: reduce) {
  .bug, .pip.just, .pen { animation: none; }
  .panel, .pod, .pod::before, .pod::after { transition: none; }
}
</style>
<script>
const REDUCED = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const TOTAL_RACES = 12;

let ws;
let currentData = null;
let previousScore = 0;
let previousEnemyScore = 0;

function animateNumber(element, start, end, duration) {
  if (REDUCED || start === end) {
    element.textContent = end;
    return;
  }
  const startTime = performance.now();
  const update = (now) => {
    const progress = Math.min((now - startTime) / duration, 1);
    const easeInOut = t => t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
    element.textContent = Math.round(start + (end - start) * easeInOut(progress));
    if (progress < 1) requestAnimationFrame(update);
  };
  requestAnimationFrame(update);
}

function fitTag(span) {
  span.style.transform = 'none';
  const max = span.parentElement.clientWidth;
  const width = span.getBoundingClientRect().width;
  if (width > max) span.style.transform = 'scale(' + (max / width) + ')';
}

function fitTags() {
  document.querySelectorAll('.tag-span').forEach(fitTag);
}

function setTag(selector, text) {
  const span = document.querySelector(selector);
  if (span.textContent !== text) span.textContent = text;
  fitTag(span);
}

function racesLabel(left) {
  if (left === 0) return 'FINAL';
  return left + (left === 1 ? ' RACE LEFT' : ' RACES LEFT');
}

function updatePips(raceLeft) {
  const spent = Math.min(Math.max(TOTAL_RACES - raceLeft, 0), TOTAL_RACES);
  document.querySelectorAll('.pip').forEach((pip, i) => {
    const wasSpent = pip.classList.contains('spent');
    pip.classList.toggle('spent', i < spent);
    if (!wasSpent && i < spent) pip.classList.add('just');
  });
  setTimeout(() => {
    document.querySelectorAll('.pip.just').forEach(p => p.classList.remove('just'));
  }, 600);
}

function apply(data) {
  setTag('.tag-home .tag-span', data.tag);
  setTag('.tag-enemy .tag-span', data.enemy_tag);

  if (data.score !== previousScore) {
    animateNumber(document.querySelector('.score-home'), previousScore, data.score, 800);
  }
  if (data.enemy_score !== previousEnemyScore) {
    animateNumber(document.querySelector('.score-enemy'), previousEnemyScore, data.enemy_score, 800);
  }

  const pod = document.querySelector('.pod');
  pod.className = 'pod ' + (data.diff > 0 ? 'plus' : data.diff < 0 ? 'minus' : '');
  pod.textContent = data.diff > 0 ? '+' + data.diff : String(data.diff);

  updatePips(data.race_left);
  document.querySelector('.races').textContent = racesLabel(data.race_left);

  document.querySelector('.pen-home').textContent = data.home_pen > 0 ? 'PEN -' + data.home_pen : '';
  document.querySelector('.pen-enemy').textContent = data.enemy_pen > 0 ? 'PEN -' + data.enemy_pen : '';

  previousScore = data.score;
  previousEnemyScore = data.enemy_score;
  currentData = data;
}

function handleError() {
  const panel = document.querySelector('.panel');
  panel.style.opacity = '0.5';
  panel.style.filter = 'grayscale(100%)';
}

function handleDataAvailable() {
  const panel = document.querySelector('.panel');
  panel.style.opacity = '1';
  panel.style.filter = 'none';
}

function connectWebSocket() {
  const channel = window.location.pathname.split('/').filter(Boolean).pop();
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws';
  ws = new WebSocket(proto + '://' + window.location.host + '/ws/' + channel);

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.error) {
      handleError();
      return;
    }
    handleDataAvailable();
    if (JSON.stringify(data) !== JSON.stringify(currentData)) {
      apply(data);
    }
  };

  ws.onclose = () => setTimeout(connectWebSocket, 1000);
}

document.addEventListener('DOMContentLoaded', () => {
  previousScore = parseInt(document.querySelector('.score-home').textContent, 10) || 0;
  previousEnemyScore = parseInt(document.querySelector('.score-enemy').textContent, 10) || 0;
  fitTags();
  document.fonts.ready.then(fitTags);
  connectWebSocket();
});
</script>
</head>"##;

#[get("/overlay/{channel_id}")]
async fn overlay(path: web::Path<String>) -> Result<impl Responder> {
    let channel_id = path.into_inner();
    let json_data = query_db(channel_id);

    let (diff_class, diff_text, race_left, tag, score, enemy_score, enemy_tag, pen_home, pen_enemy) =
        match &json_data {
            Some(data) => (
                if data.diff > 0 {
                    "plus"
                } else if data.diff < 0 {
                    "minus"
                } else {
                    ""
                },
                if data.diff > 0 {
                    format!("+{}", data.diff)
                } else {
                    data.diff.to_string()
                },
                data.race_left,
                data.tag.as_str(),
                data.score,
                data.enemy_score,
                data.enemy_tag.as_str(),
                if data.home_pen > 0 {
                    format!("PEN -{}", data.home_pen)
                } else {
                    String::new()
                },
                if data.enemy_pen > 0 {
                    format!("PEN -{}", data.enemy_pen)
                } else {
                    String::new()
                },
            ),
            None => ("", "0".to_string(), 12, "...", 0, 0, "...", String::new(), String::new()),
        };

    let spent = (12 - race_left).clamp(0, 12);
    let pips: String = (0..12)
        .map(|i| {
            if i < spent {
                r#"<span class="pip spent"></span>"#
            } else {
                r#"<span class="pip"></span>"#
            }
        })
        .collect();

    let races_label = match race_left {
        0 => "FINAL".to_string(),
        1 => "1 RACE LEFT".to_string(),
        n => format!("{n} RACES LEFT"),
    };

    let html_response = format!(
        r##"<!DOCTYPE html>
<html lang="en">
{head}
<body>
  <div class="bug">
    <div class="panel">
      <div class="main">
        <p class="tag tag-home"><span class="tag-span">{tag}</span></p>
        <div class="score-cell">
          <p class="score score-home">{score}</p>
          <p class="pen pen-home">{pen_home}</p>
        </div>
        <p class="pod {diff_class}">{diff_text}</p>
        <div class="score-cell">
          <p class="score score-enemy">{enemy_score}</p>
          <p class="pen pen-enemy">{pen_enemy}</p>
        </div>
        <p class="tag tag-enemy"><span class="tag-span">{enemy_tag}</span></p>
      </div>
      <div class="strip">
        <div class="pips">{pips}</div>
        <p class="races">{races_label}</p>
      </div>
    </div>
  </div>
</body>
</html>"##,
        head = OVERLAY_HEAD,
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(html_response))
}

#[get("/api/{channel_id}")]
async fn index(path: web::Path<String>) -> Result<impl Responder> {
    let channel_id = path.into_inner();

    Ok(web::Json(query_db(channel_id)))
}

/// Run `query_db` on a blocking threadpool so the synchronous redis call
/// never blocks the async runtime.
async fn query(channel_id: &str) -> Option<OverlayData> {
    let channel_id = channel_id.to_owned();
    web::block(move || query_db(channel_id))
        .await
        .ok()
        .flatten()
}

const DATA_UNAVAILABLE: &str = r#"{"error": "War data not available"}"#;

#[get("/ws/{channel_id}")]
async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let channel_id = path.into_inner();
    let (res, mut session, msg_stream) = actix_ws::handle(&req, stream)?;
    let mut msg_stream = msg_stream.aggregate_continuations();

    rt::spawn(async move {
        let mut hb = Instant::now();
        let mut last_data: Option<OverlayData> = None;

        // Send initial state
        match query(&channel_id).await {
            Some(data) => {
                if session
                    .text(serde_json::to_string(&data).unwrap())
                    .await
                    .is_err()
                {
                    return;
                }
                last_data = Some(data);
            }
            None => {
                let _ = session.text(DATA_UNAVAILABLE).await;
            }
        }

        let mut hb_interval = interval(Duration::from_secs(30));
        let mut poll_interval = interval(Duration::from_secs(1));

        let close_reason = loop {
            tokio::select! {
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(AggregatedMessage::Ping(bytes))) => {
                            hb = Instant::now();
                            if session.pong(&bytes).await.is_err() {
                                break None;
                            }
                        }
                        Some(Ok(AggregatedMessage::Pong(_))) => {
                            hb = Instant::now();
                        }
                        Some(Ok(AggregatedMessage::Text(_))) => {
                            match query(&channel_id).await {
                                Some(data) => {
                                    if session
                                        .text(serde_json::to_string(&data).unwrap())
                                        .await
                                        .is_err()
                                    {
                                        break None;
                                    }
                                    last_data = Some(data);
                                }
                                None => {
                                    if session.text(DATA_UNAVAILABLE).await.is_err() {
                                        break None;
                                    }
                                    last_data = None;
                                }
                            }
                        }
                        Some(Ok(AggregatedMessage::Binary(bin))) => {
                            if session.binary(bin).await.is_err() {
                                break None;
                            }
                        }
                        Some(Ok(AggregatedMessage::Close(reason))) => break reason,
                        Some(Err(_)) | None => break None,
                    }
                }
                _ = hb_interval.tick() => {
                    if Instant::now().duration_since(hb) > Duration::from_secs(75) {
                        break None;
                    }
                    if session.ping(b"").await.is_err() {
                        break None;
                    }
                }
                _ = poll_interval.tick() => {
                    let current_data = query(&channel_id).await;

                    // Handle data availability changes
                    match (&last_data, &current_data) {
                        (Some(_), None) => {
                            if session.text(DATA_UNAVAILABLE).await.is_err() {
                                break None;
                            }
                            last_data = None;
                        }
                        (None, Some(new_data)) => {
                            if session
                                .text(serde_json::to_string(new_data).unwrap())
                                .await
                                .is_err()
                            {
                                break None;
                            }
                            last_data = current_data;
                        }
                        (Some(old_data), Some(new_data)) => {
                            if old_data.tag != new_data.tag
                                || old_data.enemy_tag != new_data.enemy_tag
                                || old_data.score != new_data.score
                                || old_data.enemy_score != new_data.enemy_score
                                || old_data.diff != new_data.diff
                                || old_data.race_left != new_data.race_left
                                || old_data.home_pen != new_data.home_pen
                                || old_data.enemy_pen != new_data.enemy_pen
                            {
                                if session
                                    .text(serde_json::to_string(new_data).unwrap())
                                    .await
                                    .is_err()
                                {
                                    break None;
                                }
                            }
                            last_data = current_data;
                        }
                        (None, None) => {}
                    }
                }
            }
        };

        let _ = session.close(close_reason).await;
    });

    Ok(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(overlay).service(ws_index))
        .bind("0.0.0.0:25991")?
        .run()
        .await
}
