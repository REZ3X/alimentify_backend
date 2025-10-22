use axum::{ middleware, routing::{ get, post }, Router };

use crate::{ db::AppState, handlers, middleware as mw };

pub fn create_routes(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/auth/me", get(handlers::auth::get_current_user))
        .route_layer(middleware::from_fn_with_state(state.clone(), mw::auth::auth_middleware));

    let public_routes = Router::new()
        .route("/api/auth/google", get(handlers::auth::google_auth_url))
        .route("/api/auth/google/callback", get(handlers::auth::google_callback))
        .route("/api/auth/verify-email", get(handlers::auth::verify_email));

    Router::new()
        .route("/status", get(handlers::status::status_check))
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), mw::api_key::api_key_middleware))
}
