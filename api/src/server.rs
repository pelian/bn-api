use actix_web::http;
use actix_web::middleware::cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{server, App};
use config::Config;
use database::{ConnectionGranting, Database};
use routing;
use tari::TariClient;

pub struct AppState {
    pub config: Config,
    pub database: Box<ConnectionGranting>,
    pub token_secret: String,
    pub token_issuer: String,
}

impl AppState {
    pub fn get_tari_client(&self) -> TariClient {
        TariClient {}
    }
}

pub struct Server {
    pub config: Config,
}

impl Server {
    pub fn start(config: Config) {
        let bind_addr = format!("{}:{}", config.api_url, config.api_port);
        info!("Listening on {}", bind_addr);
        server::new({
            move || {
                App::with_state(AppState {
                    config: config.clone(),
                    database: Box::new(Database::from_config(&config)),
                    token_secret: config.token_secret.clone(),
                    token_issuer: config.token_issuer.clone(),
                }).middleware(Logger::default())
                    .configure(|a| {
                        routing::routes(
                            Cors::for_app(a)
                                .allowed_origin(&config.allowed_origins)
                                .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                                .allowed_headers(vec![
                                    http::header::AUTHORIZATION,
                                    http::header::ACCEPT,
                                ])
                                .allowed_header(http::header::CONTENT_TYPE)
                                .max_age(3600),
                        )
                    })
            }
        }).bind(&bind_addr)
            .expect(&format!("Can not bind to {}", bind_addr))
            .run();
    }
}