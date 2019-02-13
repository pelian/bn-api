use actix_web::{HttpResponse, Path, Query};
use auth::user::User as AuthUser;
use bigneon_db::models::{Organization, Report, Scopes};
use chrono::prelude::*;
use db::Connection;
use errors::*;
use helpers::application;
use models::PathParameters;
use std::str;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReportQueryParameters {
    pub report: String,
    pub start_utc: Option<NaiveDateTime>,
    pub end_utc: Option<NaiveDateTime>,
    pub event_id: Option<Uuid>,
}

pub fn get_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    match query.report.trim() {
        "transaction_details" => transaction_detail_report((connection, query, path, user)),
        "event_summary" => event_summary_report((connection, query, path, user)),
        "weekly_settlement" => weekly_settlement_report((connection, query, path, user)),
        "ticket_count" => ticket_counts((connection, query, path, user)),
        "audit_report" => audit_report((connection, query, path, user)),
        "reconciliation_summary" => reconciliation_summary_report((connection, query, path, user)),
        "reconciliation_details" => reconciliation_detail_report((connection, query, path, user)),
        _ => application::not_found(),
    }
}

pub fn transaction_detail_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if query.event_id.is_some() {
        user.requires_scope_for_organization(
            Scopes::EventFinancialReports,
            &organization,
            connection,
        )?;
    } else {
        user.requires_scope_for_organization(Scopes::OrgReports, &organization, connection)?;
    }

    let result = Report::transaction_detail_report(
        query.event_id,
        Some(path.id),
        query.start_utc,
        query.end_utc,
        connection,
    )?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn event_summary_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if query.event_id.is_some() {
        user.requires_scope_for_organization(
            Scopes::EventFinancialReports,
            &organization,
            connection,
        )?;
    } else {
        // TODO: Switch this out for bad request
        return application::unprocessable("event_id parameter is required");
    }

    let result = Report::summary_event_report(
        //We catch the is_none() above so I'll use unwrap here
        query.event_id.unwrap(),
        query.start_utc,
        query.end_utc,
        connection,
    )?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn audit_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if query.event_id.is_some() {
        user.requires_scope_for_organization(
            Scopes::EventFinancialReports,
            &organization,
            connection,
        )?;
    } else {
        // TODO: Switch this out for bad request
        return application::unprocessable("event_id parameter is required");
    }

    let all_sales_result = Report::summary_event_report(
        //We catch the is_none() above so I'll use unwrap here
        query.event_id.unwrap(),
        query.start_utc,
        query.end_utc,
        connection,
    )?;

    let end_date = query.end_utc.unwrap_or(Utc::now().naive_utc());
    let end_date_sales_result = Report::summary_event_report(
        //We catch the is_none() above so I'll use unwrap here
        query.event_id.unwrap(),
        Some(end_date.date().and_hms(0, 0, 0)),
        Some(end_date),
        connection,
    )?;

    // ticket counts
    // TODO: update this query to do the inventory at end_date
    let ticket_counts = Report::ticket_count_report(query.event_id, Some(path.id), connection)?;

    Ok(HttpResponse::Ok().json(json!({
    "end_date_sales":end_date_sales_result,
    "all_sales": all_sales_result,
    "inventory": ticket_counts
    })))
}

pub fn weekly_settlement_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let result =
        Report::organization_summary_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn ticket_counts(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if query.event_id.is_some() {
        user.requires_scope_for_organization(
            Scopes::EventFinancialReports,
            &organization,
            connection,
        )?;
    } else {
        user.requires_scope_for_organization(Scopes::OrgReports, &organization, connection)?;
    }

    let result = Report::ticket_count_report(query.event_id, Some(path.id), connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn reconciliation_summary_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let result =
        Report::reconciliation_summary_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn reconciliation_detail_report(
    (connection, query, path, user): (
        Connection,
        Query<ReportQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let result =
        Report::reconciliation_detail_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}
