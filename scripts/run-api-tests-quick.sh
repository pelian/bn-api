#!/usr/bin/env bash
set -e
./target/release/bndb_cli create -c $DATABASE_URL -f -e superuser@test.com -p password -m 8883
cd api && cargo test --release -- --nocapture
