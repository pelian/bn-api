#!/usr/bin/env bash
export FRONT_END_URL="http://localhost"
export BIGNEON_DB=bigneon
export TEST_DATABASE_URL=postgres://postgres:password123@localhost/bigneon_ci
export TEST_DATABASE_ADMIN_URL=postgres://postgres:password123@localhost/bigneon_ci
export BUILD_DIR="api"
export TARI_URL="TEST"
export COMMUNICATION_DEFAULT_SOURCE_EMAIL="noreply@bigneon.com"
export COMMUNICATION_DEFAULT_SOURCE_PHONE="0112223333"
export TOKEN_SECRET=travis_secret
export TOKEN_ISSUER=bg-on-travis
export STRIPE_SECRET_KEY="sk_test_iGn9c6EJyuF3Gx0QH6uitQlb"
export SENDGRID_API_KEY=" "
export SENDGRID_TEMPLATE_BN_REFUND="d-9ba23272db854578a5609e4e4c608f9f"
export SENDGRID_TEMPLATE_BN_USER_REGISTERED="d-9ba23272db854578a5609e4e4c608f9f"
export SENDGRID_TEMPLATE_BN_PURCHASE_COMPLETED="d-c23ba549dd0749bbb3b244b758c05dd7"
export SENDGRID_TEMPLATE_BN_ORG_INVITE="d-19ea07c6169e4fe887b6527ef16cb1ea"
export SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS="d-f6a449f0281e404899eb4d580bc342a3"
export SENDGRID_TEMPLATE_BN_PASSWORD_RESET="d-193ea5665fc54c8ca19c6325c8e46703"
export SENDGRID_TEMPLATE_BN_USER_INVITE="d-fcf7791b781644a8960820058c9074fd"
export GH_USER_EMAIL='sdbondi@users.noreply.github.com'
export GH_USER_NAME='Travis CI'
export HTTP_KEEP_ALIVE=75
export BLOCK_EXTERNAL_COMMS=1
export TWILIO_ACCOUNT_ID=" "
export TWILIO_API_KEY=" "
export API_KEYS_ENCRYPTION_KEY="test_key"
export GLOBEE_API_KEY="GDFOzMkPAw79a8TCAHKkiknJB6bEYgbb"
export GLOBEE_BASE_URL="https://test.globee.com/payment-api/v1/"
export IPN_BASE_URL="TEST"
export DATABASE_URL=postgres://postgres:password123@localhost/bigneon_ci
printenv
./scripts/run-api-tests.sh
./scripts/run-other-tests.sh
export RUST_BACKTRACE=1
export RUST_LOG=error # Postman output is very verbose
./scripts/run-integration-tests-local.sh
