use models::Roles;
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use std::str::FromStr;
use utils::errors::EnumParseError;

#[derive(PartialEq, Debug, Copy, Clone, Eq, Ord, PartialOrd)]
pub enum Scopes {
    ArtistWrite,
    BoxOfficeTicketRead,
    BoxOfficeTicketWrite,
    CodeRead,
    CodeWrite,
    CompRead,
    CompWrite,
    DashboardRead,
    EventBroadcast,
    EventCancel,
    EventFinancialReports,
    EventInterest,
    EventReports,
    EventScan,
    EventViewGuests,
    EventWrite,
    HoldRead,
    HoldWrite,
    OrderMakeExternalPayment,
    OrderRead,
    OrderReadOwn,
    OrderRefund,
    OrgAdmin,
    OrgAdminUsers,
    OrgFans,
    OrgFinancialReports,
    OrgRead,
    OrgReadEvents,
    OrgReports,
    OrgUsers,
    OrgWrite,
    RedeemTicket,
    RegionWrite,
    TicketAdmin,
    TicketRead,
    TicketTransfer,
    TicketTypeRead,
    TicketTypeWrite,
    UserRead,
    VenueWrite,
}

impl Serialize for Scopes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for Scopes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Scopes::ArtistWrite => "artist:write",
            Scopes::BoxOfficeTicketRead => "box-office-ticket:read",
            Scopes::BoxOfficeTicketWrite => "box-office-ticket:write",
            Scopes::CodeRead => "code:read",
            Scopes::CodeWrite => "code:write",
            Scopes::CompRead => "comp:read",
            Scopes::CompWrite => "comp:write",
            Scopes::DashboardRead => "dashboard:read",
            Scopes::EventBroadcast => "event:broadcast",
            Scopes::EventCancel => "event:cancel",
            Scopes::EventWrite => "event:write",
            Scopes::EventFinancialReports => "event:financial-reports",
            Scopes::EventInterest => "event:interest",
            Scopes::EventReports => "event:reports",
            Scopes::EventScan => "event:scan",
            Scopes::EventViewGuests => "event:view-guests",
            Scopes::HoldRead => "hold:read",
            Scopes::HoldWrite => "hold:write",
            Scopes::OrderRead => "order:read",
            Scopes::OrderMakeExternalPayment => "order:make-external-payment",
            Scopes::OrderReadOwn => "order:read-own",
            Scopes::OrderRefund => "order:refund",
            Scopes::OrgAdmin => "org:admin",
            Scopes::OrgRead => "org:read",
            Scopes::OrgReadEvents => "org:read-events",
            Scopes::OrgReports => "org:reports",
            Scopes::OrgFans => "org:fans",
            Scopes::OrgFinancialReports => "org:financial-reports",
            Scopes::OrgWrite => "org:write",
            Scopes::OrgAdminUsers => "org:admin-users",
            Scopes::OrgUsers => "org:users",
            Scopes::RedeemTicket => "redeem:ticket",
            Scopes::RegionWrite => "region:write",
            Scopes::UserRead => "user:read",
            Scopes::VenueWrite => "venue:write",
            Scopes::TicketAdmin => "ticket:admin",
            Scopes::TicketRead => "ticket:read",
            Scopes::TicketTransfer => "ticket:transfer",
            Scopes::TicketTypeRead => "ticket-type:read",
            Scopes::TicketTypeWrite => "ticket-type:write",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Scopes {
    type Err = EnumParseError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let s = match s {
            "artist:write" => Scopes::ArtistWrite,
            "box-office-ticket:read" => Scopes::BoxOfficeTicketRead,
            "box-office-ticket:write" => Scopes::BoxOfficeTicketWrite,
            "code:read" => Scopes::CodeRead,
            "code:write" => Scopes::CodeWrite,
            "comp:read" => Scopes::CompRead,
            "comp:write" => Scopes::CompWrite,
            "dashboard:read" => Scopes::DashboardRead,
            "event:broadcast" => Scopes::EventBroadcast,
            "event:cancel" => Scopes::EventCancel,
            "event:write" => Scopes::EventWrite,
            "event:financial-reports" => Scopes::EventFinancialReports,
            "event:interest" => Scopes::EventInterest,
            "event:reports" => Scopes::EventReports,
            "event:scan" => Scopes::EventScan,
            "event:view-guests" => Scopes::EventViewGuests,
            "hold:read" => Scopes::HoldRead,
            "hold:write" => Scopes::HoldWrite,
            "order:read" => Scopes::OrderRead,
            "order:make-external-payment" => Scopes::OrderMakeExternalPayment,
            "order:read-own" => Scopes::OrderReadOwn,
            "order:refund" => Scopes::OrderRefund,
            "org:admin" => Scopes::OrgAdmin,
            "org:read" => Scopes::OrgRead,
            "org:read-events" => Scopes::OrgReadEvents,
            "org:reports" => Scopes::OrgReports,
            "org:fans" => Scopes::OrgFans,
            "org:financial-reports" => Scopes::OrgFinancialReports,
            "org:write" => Scopes::OrgWrite,
            "org:admin-users" => Scopes::OrgAdminUsers,
            "org:users" => Scopes::OrgUsers,
            "redeem:ticket" => Scopes::RedeemTicket,
            "region:write" => Scopes::RegionWrite,
            "user:read" => Scopes::UserRead,
            "venue:write" => Scopes::VenueWrite,
            "ticket:admin" => Scopes::TicketAdmin,
            "ticket:read" => Scopes::TicketRead,
            "ticket:transfer" => Scopes::TicketTransfer,
            "ticket-type:read" => Scopes::TicketTypeRead,
            "ticket-type:write" => Scopes::TicketTypeWrite,
            _ => {
                return Err(EnumParseError {
                    message: "Could not parse value".to_string(),
                    enum_type: "Scopes".to_string(),
                    value: s.to_string(),
                });
            }
        };
        Ok(s)
    }
}

pub fn get_scopes(roles: Vec<Roles>) -> Vec<Scopes> {
    let mut scopes: Vec<Scopes> = roles
        .into_iter()
        .flat_map(|r| get_scopes_for_role(r))
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

fn get_scopes_for_role(role: Roles) -> Vec<Scopes> {
    use models::Roles::*;
    let mut roles = match role {
        User => {
            let mut roles = vec![
                Scopes::EventInterest,
                Scopes::OrderReadOwn,
                Scopes::TicketTransfer,
            ];
            roles
        }
        DoorPerson => {
            let mut roles = vec![
                Scopes::RedeemTicket,
                Scopes::OrgReadEvents,
                Scopes::HoldRead,
                Scopes::EventScan,
                Scopes::TicketRead,
                Scopes::EventViewGuests,
            ];
            roles
        }
        OrgBoxOffice => {
            let mut roles = vec![
                Scopes::DashboardRead,
                Scopes::EventViewGuests,
                Scopes::OrderMakeExternalPayment,
                Scopes::BoxOfficeTicketRead,
            ];
            roles.extend(get_scopes_for_role(Roles::DoorPerson));
            roles
        }
        PromoterReadOnly => {
            let mut roles = vec![
                Scopes::CodeRead,
                Scopes::CompRead,
                Scopes::DashboardRead,
                Scopes::EventViewGuests,
                Scopes::EventInterest,
                Scopes::HoldRead,
                Scopes::OrderRead,
                Scopes::OrgReadEvents,
                Scopes::TicketRead,
                Scopes::TicketTypeRead,
            ];
            roles
        }
        Promoter => {
            let mut roles = vec![
                Scopes::CodeWrite,
                Scopes::CompWrite,
                // Scopes::EventFinancialReports,
                // Scopes::EventReports,
                // To be updated later
                Scopes::EventWrite,
                Scopes::TicketTypeRead,
                Scopes::TicketTypeWrite,
                Scopes::HoldWrite,
                // Scopes::OrderRefund,
            ];
            roles.extend(get_scopes_for_role(Roles::PromoterReadOnly));
            roles
        }
        OrgMember => {
            let mut roles = vec![
                Scopes::ArtistWrite,
                Scopes::BoxOfficeTicketRead,
                Scopes::BoxOfficeTicketWrite,
                Scopes::CodeRead,
                Scopes::CodeWrite,
                Scopes::CompRead,
                Scopes::CompWrite,
                Scopes::DashboardRead,
                Scopes::EventCancel,
                Scopes::EventScan,
                Scopes::EventViewGuests,
                Scopes::EventWrite,
                Scopes::HoldRead,
                Scopes::HoldWrite,
                Scopes::OrderRead,
                Scopes::OrderRefund,
                Scopes::OrgRead,
                Scopes::OrgReadEvents,
                Scopes::OrgFans,
                Scopes::RedeemTicket,
                Scopes::TicketAdmin,
                Scopes::TicketRead,
                Scopes::TicketTypeRead,
                Scopes::TicketTypeWrite,
                Scopes::VenueWrite,
            ];
            roles.extend(get_scopes_for_role(Roles::User));
            roles
        }
        OrgAdmin => {
            let mut roles = vec![
                Scopes::OrgWrite,
                Scopes::UserRead,
                Scopes::OrgUsers,
                Scopes::EventBroadcast,
                Scopes::EventFinancialReports,
                Scopes::EventReports,
                Scopes::OrgReports,
            ];
            roles.extend(get_scopes_for_role(OrgMember));
            roles.extend(get_scopes_for_role(Roles::OrgBoxOffice));
            roles
        }
        OrgOwner => {
            let mut roles = vec![Scopes::OrgAdminUsers];
            roles.extend(get_scopes_for_role(Roles::OrgAdmin));
            roles
        }
        Admin => {
            let mut roles = vec![
                Scopes::OrgAdmin,
                Scopes::RegionWrite,
                Scopes::OrgFinancialReports,
            ];
            roles.extend(get_scopes_for_role(OrgOwner));
            roles
        }
    };
    roles.sort();
    roles.dedup();

    roles
}

#[test]
fn get_scopes_for_role_test() {
    let res = get_scopes_for_role(Roles::OrgOwner);
    assert_equiv!(
        vec![
            Scopes::ArtistWrite,
            Scopes::BoxOfficeTicketRead,
            Scopes::BoxOfficeTicketWrite,
            Scopes::CodeRead,
            Scopes::CodeWrite,
            Scopes::CompRead,
            Scopes::CompWrite,
            Scopes::DashboardRead,
            Scopes::EventBroadcast,
            Scopes::EventCancel,
            Scopes::EventFinancialReports,
            Scopes::EventInterest,
            Scopes::EventReports,
            Scopes::EventScan,
            Scopes::EventViewGuests,
            Scopes::EventWrite,
            Scopes::HoldRead,
            Scopes::HoldWrite,
            Scopes::OrderMakeExternalPayment,
            Scopes::OrderRead,
            Scopes::OrderReadOwn,
            Scopes::OrderRefund,
            Scopes::OrgAdminUsers,
            Scopes::OrgFans,
            Scopes::OrgRead,
            Scopes::OrgReports,
            Scopes::OrgUsers,
            Scopes::OrgWrite,
            Scopes::RedeemTicket,
            Scopes::TicketAdmin,
            Scopes::TicketRead,
            Scopes::TicketTransfer,
            Scopes::TicketTypeRead,
            Scopes::TicketTypeWrite,
            Scopes::UserRead,
            Scopes::VenueWrite,
            Scopes::OrgReadEvents
        ],
        res
    );
}

#[test]
fn scopes_to_string() {
    assert_eq!("org:admin".to_string(), Scopes::OrgAdmin.to_string());
}

#[test]
fn get_scopes_test() {
    let mut res = get_scopes(vec![Roles::OrgOwner])
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    res.sort();
    assert_equiv!(
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:admin-users",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "ticket-type:read",
            "ticket-type:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "user:read",
            "venue:write",
        ],
        res
    );
    let mut res = get_scopes(vec![Roles::Admin])
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    res.sort();
    assert_equiv!(
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "region:write",
            "ticket-type:read",
            "ticket-type:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "user:read",
            "venue:write",
        ],
        res
    );

    let res = get_scopes(vec![Roles::OrgOwner, Roles::Admin])
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    assert_equiv!(
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "region:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "user:read",
            "venue:write",
        ],
        res
    );
}

#[test]
fn from_str() {
    let s: Scopes = "ticket:read".parse().unwrap();
    assert_eq!(Scopes::TicketRead, s);
}
