// main.rs
use warp::Filter;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct AppState {
    content: Arc<Mutex<String>>,
}

#[derive(Serialize, Deserialize)]
struct UpdateContent {
    text: String,
}

#[tokio::main]
async fn main() {
    let app_state = AppState {
        content: Arc::new(Mutex::new(String::new())),
    };

    let state_filter = warp::any().map(move || app_state.clone());

    // GET 路由：返回HTML页面
    let index_route = warp::path::end()
        .and(state_filter.clone())
        .map(|_state: AppState| {
            let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Simple Pastebin</title>
    <style>
        body {{ max-width: 800px; margin: 20px auto; padding: 0 20px; }}
        textarea {{ 
            width: 100%; 
            height: 400px; 
            padding: 10px;
            font-family: monospace;
            border: 2px solid #ccc;
            border-radius: 5px;
            resize: vertical;
        }}
    </style>
</head>
<body>
    <textarea id="content" placeholder="Enter text here..."></textarea>
    <script>
        // 加载现有内容
        fetch('/content')
            .then(r => r.text())
            .then(text => {{
                document.getElementById('content').value = text;
            }});

        // 自动保存逻辑
        const textarea = document.getElementById('content');
        let saveTimeout;
        
        textarea.addEventListener('input', () => {{
            clearTimeout(saveTimeout);
            saveTimeout = setTimeout(() => {{
                fetch('/content', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ text: textarea.value }})
                }});
            }}, 500); // 500ms防抖
        }});
    </script>
</body>
</html>
            "#);
            warp::reply::html(html)
        });

    // GET 路由：获取当前内容
    let get_content_route = warp::path!("content")
        .and(warp::get())
        .and(state_filter.clone())
        .map(|state: AppState| {
            let content = state.content.lock().unwrap().clone();
            warp::reply::html(content)
        });

    // POST 路由：更新内容
    let post_content_route = warp::path!("content")
        .and(warp::post())
        .and(warp::body::json())
        .and(state_filter.clone())
        .map(|update: UpdateContent, state: AppState| {
            let mut content = state.content.lock().unwrap();
            *content = update.text;
            warp::reply::with_status("Updated", warp::http::StatusCode::OK)
        });

    let routes = index_route
        .or(get_content_route)
        .or(post_content_route);

    println!("Server running at http://0.0.0.0:8000");
    warp::serve(routes).run(([0, 0, 0, 0], 8000)).await;
}
