use axum::{ middleware, routing::{ delete, get, post, put }, Router };

use crate::{ db::AppState, handlers, middleware as mw };

pub fn create_routes(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/auth/me", get(handlers::auth::get_current_user))
        .route("/api/nutrition/analyze", post(handlers::nutrition::analyze_food))
        .route("/api/nutrition/analyze-text", post(handlers::nutrition::analyze_food_text))
        .route("/api/nutrition/quick-check", post(handlers::nutrition::quick_food_check))
        .route("/api/nutrition-info", get(handlers::nutrition_info::get_nutrition_info))
        .route("/api/food-wiki/search", get(handlers::food_wiki::search_foods))
        .route("/api/food-wiki/:fdc_id", get(handlers::food_wiki::get_food_details))
        .route("/api/food-wiki/foods", post(handlers::food_wiki::get_foods))
        .route("/api/recipes/search", get(handlers::recipes::search_recipes))
        .route("/api/recipes/random", get(handlers::recipes::get_random_recipes))
        .route("/api/recipes/:meal_id", get(handlers::recipes::get_recipe_by_id))
        .route("/api/recipes/category/:category", get(handlers::recipes::filter_by_category))
        .route("/api/recipes/area/:area", get(handlers::recipes::filter_by_area))
        .route("/api/health/profile", post(handlers::health::create_or_update_profile))
        .route("/api/health/profile", get(handlers::health::get_profile))
        .route("/api/meals/log", post(handlers::meals::log_meal))
        .route("/api/meals/daily", get(handlers::meals::get_daily_meals))
        .route("/api/meals/period-stats", get(handlers::meals::get_period_stats))
        .route("/api/meals/:id", put(handlers::meals::update_meal))
        .route("/api/meals/:id", delete(handlers::meals::delete_meal))
        .route("/api/reports/generate", post(handlers::reports::generate_report))
        .route("/api/reports", get(handlers::reports::get_user_reports))
        .route("/api/reports/:id", get(handlers::reports::get_report_by_id))
        .route("/api/reports/:id", delete(handlers::reports::delete_report))
        .route_layer(middleware::from_fn_with_state(state.clone(), mw::auth::auth_middleware));

    let public_routes = Router::new()
        .route("/api/auth/google", get(handlers::auth::google_auth_url))
        .route("/api/auth/google/callback", get(handlers::auth::google_callback))
        .route("/api/auth/verify-email", get(handlers::auth::verify_email));
        // .route("/api/auth/debug-config", get(handlers::auth::debug_config));

    Router::new()
        .route("/", get(handlers::dashboard::serve_dashboard))
        .route("/docs", get(handlers::dashboard::serve_docs))
        .route("/status", get(handlers::status::status_check))
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), mw::api_key::api_key_middleware))
}
