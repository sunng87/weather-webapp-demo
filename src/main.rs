use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::Extension;
use axum::prelude::*;
use axum::AddExtensionLayer;
use handlebars::Handlebars;
use hyper::Server;
use metriki_core::MetricsRegistry;
use metriki_log_reporter::LogReporterBuilder;
use metriki_tower::http::HyperMetricsLayerBuilder;
use openweathermap::weather;
use serde_json::json;

async fn weather_index(Extension(hbs): Extension<Arc<Handlebars<'_>>>) -> response::Html<String> {
    let api_key = env::var("WEATHERAPP_API_KEY").unwrap();
    let weather = weather("Beijing,CN", "metric", "en", &api_key)
        .await
        .unwrap();

    let html = hbs
        .render(
            "index",
            &json!({ "name": weather.name,
                      "temp": weather.main.temp }),
        )
        .unwrap();

    response::Html(html)
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut hbs = Handlebars::new();
    hbs.set_dev_mode(true);
    hbs.register_template_file("index", "./templates/index.hbs")
        .unwrap();

    let metriki = MetricsRegistry::arc();
    let log_reporter = LogReporterBuilder::default()
        .registry(metriki.clone())
        .interval_secs(30)
        .build()
        .unwrap();
    log_reporter.start();

    let hbs = Arc::new(hbs);
    let app = route("/", get(weather_index))
        .layer(AddExtensionLayer::new(hbs))
        .layer(
            HyperMetricsLayerBuilder::default()
                .registry(metriki.clone())
                .base_metric_name("weatherapp")
                .build()
                .unwrap(),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
