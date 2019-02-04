#!/usr/bin/env bash

# Ensure we are in the root of the git repo
cd $(git rev-parse --show-toplevel)
cd db
cargo run --release create -c $DATABASE_URL -f -e superuser@test.com -p password -m 8883 || {
    echo "Migrations failed"
    exit 1
}
cd ../api
cargo build --release
cargo run --release -- -t false &
export SERVER_PID=$!$1
# Run newman tests
#apt-get install nodejs
#apt-get install npm
cd ../target
    # Workaround for invalid NPM certificate

npm config set strict-ssl false
npm init -y
npm install newman

node ./node_modules/newman/bin/newman.js run --timeout-request 60000 ../integration-tests/bigneon-tests.postman_collection.json -e ../integration-tests/travis.postman_environment.json
NEWMAN_EXIT_CODE=$?
kill -s SIGTERM $SERVER_PID
if [ $NEWMAN_EXIT_CODE -ne 0 ]
then
    exit $NEWMAN_EXIT_CODE
fi
cd ../api
cargo run --release -- -b true

