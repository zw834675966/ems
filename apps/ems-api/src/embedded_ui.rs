#![cfg(feature = "embedded-ui")]

use axum::{
    http::{StatusCode, Uri, header},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/admin/dist"]
struct Assets;

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data.into_owned()).into_response()
        }
        None => {
            // SPA：路由（如 /ems/point-mappings）落到 index.html
            if !path.contains('.') {
                if let Some(index) = Assets::get("index.html") {
                    return (
                        [(header::CONTENT_TYPE, "text/html")],
                        index.data.into_owned(),
                    )
                        .into_response();
                }
            }
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

