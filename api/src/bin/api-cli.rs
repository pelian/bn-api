#![deny(unused_extern_crates)]
extern crate bigneon_api;
extern crate bigneon_db;
extern crate dotenv;
#[macro_use]
extern crate log;
#[macro_use]
extern crate logging;
extern crate clap;

use bigneon_api::config::{Config, Environment};
use bigneon_api::db::Database;
use bigneon_api::utils::ServiceLocator;
use bigneon_db::prelude::*;
use clap::*;
use dotenv::dotenv;
use log::Level::*;
use std::{thread, time};

pub fn main() {
    logging::setup_logger();
    info!("Loading environment");
    dotenv().ok();
    jlog!(Info, "Environment loaded");

    let config = Config::new(Environment::Development);
    let service_locator = ServiceLocator::new(&config);
    let database = Database::from_config(&config);

    let matches = App::new("Big Neon API CLI")
        .author("Big Neon")
        .about("Command Line Interface for running tasks for the Big Neon API")
        .subcommand(
            SubCommand::with_name("sync-purchase-metadata").about("Syncs purchase metadata"),
        )
        .get_matches();

    match matches.subcommand() {
        ("sync-purchase-metadata", Some(_)) => sync_purchase_metadata(database, service_locator),
        _ => {
            eprintln!("Invalid subcommand '{}'", matches.subcommand().0);
        }
    }
}

fn sync_purchase_metadata(database: Database, service_locator: ServiceLocator) {
    info!("Syncing purchase metadata");
    let connection = database
        .get_connection()
        .expect("Expected connection to establish");
    let mut i = 0;
    let mut exit = false;

    loop {
        if exit {
            break;
        }

        let payments = Payment::find_all_with_orders_paginated_by_provider(
            PaymentProviders::Stripe,
            i,
            50,
            connection.get(),
        )
        .expect("Expected to find all payments with orders");
        i += 1;

        if payments.len() == 0 {
            break;
        }

        for (payment, order) in payments {
            let organizations = order.organizations(connection.get()).unwrap();

            let stripe = service_locator
                .create_payment_processor(PaymentProviders::Stripe, &organizations[0])
                .expect("Expected Stripe processor");

            if let Some(external_reference) = payment.external_reference {
                let purchase_metadata = order
                    .purchase_metadata(connection.get())
                    .expect("Expected purchase metadata for order");
                let result = stripe.update_metadata(&external_reference, purchase_metadata);

                match result {
                    // Sleep to avoid hammering Stripe API
                    Ok(_) => thread::sleep(time::Duration::from_secs(1)),
                    Err(error) => {
                        error!("Error: {}", error);
                        exit = true;
                        break;
                    }
                }
            }
        }
    }
}
