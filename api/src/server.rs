use actix_web::http;
use actix_web::middleware::cors::Cors;
use actix_web::{server, App};

use config::Config;
use db::*;
use domain_events::DomainActionMonitor;
use middleware::{AppVersionHeader, BigNeonLogger, DatabaseTransaction};
use routing;
use std::io;
use utils::spotify;
use utils::ServiceLocator;

// Must be valid JSON
const LOGGER_FORMAT: &'static str = r#"{"level": "INFO", "target":"bigneon::request", "remote_ip":"%a", "user_agent": "%{User-Agent}i", "request": "%r", "status_code": %s, "response_time": %D, "api_version":"%{x-app-version}o", "client_version": "%{X-API-Client-Version}i" }"#;

pub struct AppState {
    pub config: Config,
    pub database: Database,
    pub service_locator: ServiceLocator,
}

impl AppState {
    pub fn new(config: Config, database: Database) -> AppState {
        AppState {
            database,
            service_locator: ServiceLocator::new(&config),
            config,
        }
    }
}
pub struct Server {
    pub config: Config,
}

impl Server {
    pub fn start(
        config: Config,
        process_actions: bool,
        process_http: bool,
        process_actions_til_empty: bool,
    ) {
        let bind_addr = format!("{}:{}", config.api_host, config.api_port);

        let database = Database::from_config(&config);

        let mut domain_action_monitor =
            DomainActionMonitor::new(config.clone(), database.clone(), 1);
        if process_actions_til_empty {
            domain_action_monitor.run_til_empty().unwrap();
            return;
        }

        if process_actions {
            domain_action_monitor.start()
        }

        if config.spotify_auth_token.is_some() {
            let token = config.spotify_auth_token.clone().unwrap();
            spotify::SINGLETON.set_auth_token(&token);
        }

        if process_http {
            info!("Listening on {}", bind_addr);
            let keep_alive = server::KeepAlive::Tcp(config.http_keep_alive);
            server::new({
                move || {
                    App::with_state(AppState::new(config.clone(), database.clone()))
                        .middleware(BigNeonLogger::new(LOGGER_FORMAT))
                        .middleware(DatabaseTransaction::new())
                        .middleware(AppVersionHeader::new())
                        .configure(|a| {
                            let mut cors_config = Cors::for_app(a);
                            match config.allowed_origins.as_ref() {
                                "*" => cors_config.send_wildcard(),
                                _ => cors_config.allowed_origin(&config.allowed_origins),
                            };
                            cors_config
                                .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                                .allowed_headers(vec![
                                    http::header::AUTHORIZATION,
                                    http::header::ACCEPT,
                                    "X-API-Client-Version"
                                        .parse::<http::header::HeaderName>()
                                        .unwrap(),
                                ])
                                .allowed_header(http::header::CONTENT_TYPE)
                                .max_age(3600);

                            routing::routes(&mut cors_config)
                        })
                }
            })
            .keep_alive(keep_alive)
            .bind(&bind_addr)
            .unwrap_or_else(|_| panic!("Can not bind to {}", bind_addr))
            .run();
        } else {
            info!("Press enter to stop");
            let mut input = String::new();
            let _ = io::stdin().read_line(&mut input);
        }

        if process_actions {
            domain_action_monitor.stop()
        }
    }
}
