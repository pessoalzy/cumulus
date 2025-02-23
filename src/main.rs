use warp::Filter;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use warp::sse::Event;

#[derive(Debug, Clone)]
struct AppState {
    content: Arc<Mutex<String>>,
    updates: Arc<Mutex<broadcast::Sender<String>>>,
}

#[derive(Serialize, Deserialize)]
struct UpdateContent {
    text: String,
}

#[tokio::main]
async fn main() {
    let (update_tx, _) = broadcast::channel(128);
    let app_state = AppState {
        content: Arc::new(Mutex::new(String::new())),
        updates: Arc::new(Mutex::new(update_tx)),
    };

    let state_filter = warp::any().map(move || app_state.clone());

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST"])
        .allow_header("content-type");

    let index_route = warp::path::end()
        .map(|| warp::reply::html(include_str!("index.html")));

    let get_content_route = warp::path!("content")
        .and(warp::get())
        .and(state_filter.clone())
        .map(|state: AppState| warp::reply::html(state.content.lock().unwrap().clone()));

    let post_content_route = warp::path!("content")
        .and(warp::post())
        .and(warp::body::json())
        .and(state_filter.clone())
        .map(|update: UpdateContent, state: AppState| {
            let mut content = state.content.lock().unwrap();
            *content = update.text.clone();
            let _ = state.updates.lock().unwrap().send(update.text);
            warp::reply::with_status("Updated", warp::http::StatusCode::OK)
        });
    
    let sse_route = warp::path("updates")
    .and(warp::get())
    .and(state_filter.clone())
    .map(|state: AppState| {
        let mut rx = state.updates.lock().unwrap().subscribe(); // 添加 mut
        let event_stream = async_stream::stream! {
            loop {
                match rx.recv().await {
                    Ok(content) => {
                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(content));
                    },
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        warp::sse::reply(warp::sse::keep_alive().stream(event_stream))
    });

    let routes = index_route
        .or(get_content_route)
        .or(post_content_route)
        .or(sse_route)
        .with(cors);

    println!("Server running at http://0.0.0.0:8000");
    warp::serve(routes).run(([0, 0, 0, 0], 8000)).await;
}

