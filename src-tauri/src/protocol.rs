//! `milano://localhost/asset/<sha256>/<original|256|1024>` — read-only access
//! to imported assets and their thumbnails. Nothing outside the asset and
//! thumbnail directories can be reached: the path must be a content hash.

use tauri::http::{header, Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use milano_core::images::Variant;

use crate::state::AppState;

pub const SCHEME: &str = "milano";

fn error(status: StatusCode, msg: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(msg.as_bytes().to_vec())
        .expect("static response")
}

pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().to_string();
    tauri::async_runtime::spawn(async move {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let (id, variant) = match parts.as_slice() {
            ["asset", id, v] => match Variant::parse(v) {
                Some(variant) => (id.to_string(), variant),
                None => return responder.respond(error(StatusCode::BAD_REQUEST, "unknown variant")),
            },
            _ => return responder.respond(error(StatusCode::NOT_FOUND, "not found")),
        };
        let state = app.state::<AppState>();
        let response = match state.thumbnails.load(&id, variant).await {
            Ok((bytes, mime)) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                // The URL is the content hash: it never changes meaning.
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .header("X-Content-Type-Options", "nosniff")
                .header("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'")
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(bytes.as_ref().clone())
                .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "response")),
            Err(milano_core::Error::NotFound(_)) => error(StatusCode::NOT_FOUND, "asset not found"),
            Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        };
        responder.respond(response);
    });
}
