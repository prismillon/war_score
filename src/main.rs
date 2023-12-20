use actix_web::{get, web, App, HttpServer, Result, Responder, HttpResponse};
use serde::Serialize;
use rusqlite::Connection;

struct War {
    id: i32,
    tag: String,
    ennemy_tag: String,
}

struct Race {
    diff: i32,
}

#[derive(Serialize)]
struct WarData {
    id: i32,
    tag: String,
    ennemy_tag: String,
    score: i32,
    ennemy_score: i32,
    diff: i32,
    last_diff: Option<i32>,
    race_count: i32,
}


fn query_db(channel_id: String) -> Option<WarData> {
    let conn = match Connection::open("/app/mk.db") {
        Ok(v) => v,
        Err(_) => return None,
    };
    let war_query = conn.query_row("SELECT id, tag, ennemyTag FROM wars where channelId = ?1 order by id desc limit 1", [channel_id], 
        |row| {
            Ok(War {
                id: row.get(0)?,
                tag: row.get(1)?,
                ennemy_tag: row.get(2)?,
            })
        }
    );
    
    let war_data = match war_query {
        Ok(v) => v,
        Err(_) => return None,
    };
    
    let mut races_stmt = match conn.prepare("select diff from races where warId = ?1") {
        Ok(v) => v,
        Err(_) => return None,
    };

    let races_query = races_stmt.query_map([war_data.id], |row| {
        Ok(Race {
            diff: row.get(0)?,
        })
    });

    let race_data = match races_query {
        Ok(v) => v,
        Err(_) => return None,
    };

    let mut diffs: Vec<i32> = vec![];

    for race in race_data {
        match race {
            Ok(r) => diffs.push(r.diff),
            Err(_) => continue,
        }
    };

    let diffs = diffs;
    let race_count = i32::try_from(diffs.len()).unwrap_or(0);
    let diff: i32 = diffs.iter().sum();
    let score = race_count * 41 + diff/2;
    let ennemy_score = race_count * 41 - diff/2;
    let last_diff = diffs.iter().last();

    let res = WarData {
        id: war_data.id,
        tag: war_data.tag,
        ennemy_tag: war_data.ennemy_tag,
        score,
        ennemy_score,
        diff,
        last_diff: Some(*last_diff.unwrap_or(&0)),
        race_count,
    };

    Some(res)
}



#[derive(Serialize)]
struct OverlayData {
    tag: String,
    enemy_tag: String,
    score: i32,
    ennemy_score: i32,
    diff: i32,
    last_diff: Option<i32>,
    race_count: i32,
}

#[get("/overlay/{channel_id}")]
async fn overlay(path: web::Path<String>) -> Result<impl Responder> {
    let channel_id = path.into_inner();
    let json_data = query_db(channel_id.clone());

    match json_data {
        Some(data) => {
            let overlay_data = OverlayData {
                tag: data.tag,
                enemy_tag: data.ennemy_tag,
                score: data.score,
                ennemy_score: data.ennemy_score,
                diff: data.diff,
                last_diff: data.last_diff,
                race_count: data.race_count,
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
                    background: rgba(0, 0, 0, 0.4);
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
                setInterval(async () => {{
                    const response = await fetch('/json/{}');
                    const newData = await response.json();
                
                    if (JSON.stringify(newData) !== JSON.stringify(currentData)) {{
                        location.reload();
                    }}
                    currentData = newData;

                }}, 5000);
                </script>
                </head>
                <body>
                <div id="team-num-2" class="overlay-container respect-kusaan team-num-2">
                    <div class="overlay-inner">
                    <div class="header">
                        <p class="score-dif {}">{}</p>
                        <p class="space"></p>
                        <p class="left-race">race count: {}</p>
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
                channel_id,
                class,
                diff,
                overlay_data.race_count,
                overlay_data.tag,
                overlay_data.score,
                overlay_data.ennemy_score,
                overlay_data.enemy_tag
            );

            Ok(HttpResponse::Ok().content_type("text/html").body(html_response))
        }
        None => Ok(HttpResponse::NotFound().body("Data not found")),
    }
}


#[get("/json/{channel_id}")]
async fn index(path: web::Path<String>) -> Result<impl Responder> {
    let channel_id = path.into_inner();

    Ok(web::Json(query_db(channel_id)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
        .service(index)
        .service(overlay)
    })
    .bind("127.0.0.1:55991")?
    .run()
    .await
}
