use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::*;
use std::fmt;
use std::io::Write;
use std::str;
use std::str::FromStr;
use utils::errors::EnumParseError;

macro_rules! string_enum {
    ($name:ident [$($value:ident),+]) => {

        #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Eq, Hash, FromSqlRow, AsExpression)]
        #[sql_type = "Text"]
        pub enum $name {
            $(
                $value,
            )*
        }

        impl ToSql<Text, Pg> for $name {
            fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
                match *self {
                    $(
                      $name::$value => out.write_all(stringify!($value).as_bytes())?,
                    )*
                }
                Ok(IsNull::No)
            }
        }

        impl FromSql<Text, Pg> for $name {
            fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
                let s = str::from_utf8(not_none!(bytes))?;
                s.parse().map_err(|_| format!("Unrecognized enum variant:{}", s).into())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
             let s = match self {
                  $(
                    $name::$value => stringify!($value),
                   )*
                };
                write!(f, "{}", s)
            }
        }

        impl FromStr for $name {
            type Err = EnumParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
               $(
                  if s.eq_ignore_ascii_case(stringify!($value)) {
                     return Ok($name::$value);
                  }
               )*

               Err(EnumParseError {
                          message: "Could not parse value".to_string(),
                          enum_type: stringify!($name).to_string(),
                          value: s.to_string(),
                      })
            }
        }
    }
}

string_enum! { AssetStatus [Unsynced] }
string_enum! { BroadcastAudience [ PeopleAtTheEvent ]}
string_enum! { CartItemStatus [CodeExpired, HoldExpired, TicketNullified, TicketNotReserved, Valid] }
string_enum! { CodeTypes [Access, Discount] }
string_enum! { CommunicationChannelType [Email, Sms, Push]}
string_enum! { DomainEventTypes [
    EventArtistCreated,
    EventArtistAdded,
    EventCancelled,
    EventCreated,
    EventPublished,
    EventUpdated,
    EventUnpublished,
    FeeScheduleCreated,
    HoldCreated,
    HoldDeleted,
    HoldQuantityChanged,
    OrderBehalfOfUserChanged,
    OrderCompleted,
    OrderCreated,
    OrderStatusUpdated,
    OrderUpdated,
    OrganizationCreated,
    PaymentCancelled,
    PaymentCreated,
    PaymentCompleted,
    PaymentRefund,
    PaymentProviderIPN,
    PaymentMethodCreated,
    PaymentMethodUpdated,
    PaymentUpdated,
    UserLogin,
    UserRegistration,
    LostPassword,
    PurchaseCompleted,
    TransferTicketStarted,
    TransferTicketCancelled,
    TransferTicketCompleted,
    TicketInstanceAddedToHold,
    TicketInstanceNullified,
    TicketInstancePurchased,
    TicketInstanceRedeemed,
    TicketInstanceReleasedFromHold
]}
string_enum! { DomainActionTypes [
    BroadcastPushNotification,
    // Email/SMS/Push Communication
    Communication,
    // Marketing Contacts
    MarketingContactsCreateEventList,
    MarketingContactsBulkEventFanListImport,
    PaymentProviderIPN,
    SendPurchaseCompletedCommunication

]}
string_enum! { BroadcastStatus [Pending, InProgress, Completed, Cancelled]}
string_enum! { BroadcastChannel [PushNotification]}
string_enum! { BroadcastType [LastCall]}
string_enum! { DomainActionStatus [Pending, RetriesExceeded, Errored, Success, Cancelled]}
string_enum! { EventStatus [Draft,Closed,Published,Offline]}
string_enum! { EventSearchSortField [ Name, EventStart]}
string_enum! { EventOverrideStatus [PurchaseTickets,SoldOut,OnSaleSoon,TicketsAtTheDoor,Free,Rescheduled,Cancelled,OffSale,Ended]}
string_enum! { EventTypes [ Music, Conference]}
string_enum! { ExternalPaymentType [Cash, CreditCard, Voucher]}
string_enum! { FanSortField [FirstName, LastName, Email, Phone, Orders, FirstOrder, LastOrder, Revenue] }
string_enum! { HistoryType [Purchase]}
string_enum! { HoldTypes [Discount, Comp] }
string_enum! { HoldStatus [Published, Deleted] }
string_enum! { OrderStatus [Cancelled, Draft, Paid, PendingPayment] }
string_enum! { OrderItemTypes [Tickets, PerUnitFees, EventFees, Discount]}
string_enum! { OrderTypes [Cart, BackOffice] }
string_enum! { PaymentMethods [External, CreditCard, Provider] }
string_enum! { PaymentProviders [External, Globee, Stripe] }
string_enum! { PaymentStatus [Authorized, Completed, Requested, Refunded, Unpaid, PendingConfirmation, Cancelled, Draft, Unknown, PendingIpn] }
string_enum! { PastOrUpcoming [Past,Upcoming]}
string_enum! { Roles [Admin, DoorPerson, OrgMember, OrgOwner, OrgAdmin, OrgBoxOffice, Promoter, PromoterReadOnly, User] }
string_enum! { SettlementStatus[PendingSettlement, RequiresAudit, SettledInFull] }
string_enum! { SettlementTransactionType[OrderItem, Manual, Report] }
string_enum! { SortingDir[ Asc, Desc ] }
string_enum! { Tables [Broadcasts, Events, EventArtists, FeeSchedules, Holds, Orders, Organizations, Payments, PaymentMethods, TicketInstances, Users] }
string_enum! { TicketInstanceStatus [Available, Reserved, Purchased, Redeemed, Nullified]}
string_enum! { TicketPricingStatus [Published, Deleted, Default] }
string_enum! { TicketTypeStatus [NoActivePricing, Published, SoldOut, Cancelled] }
string_enum! { SoldOutBehavior[ ShowSoldOut, Hide ]}

impl Roles {
    pub fn get_event_limited_roles() -> Vec<Roles> {
        vec![Roles::Promoter, Roles::PromoterReadOnly]
    }
}

impl Default for EventStatus {
    fn default() -> EventStatus {
        EventStatus::Draft
    }
}

impl Default for EventTypes {
    fn default() -> EventTypes {
        EventTypes::Music
    }
}

impl Default for BroadcastType {
    fn default() -> BroadcastType {
        BroadcastType::LastCall
    }
}

impl Default for BroadcastStatus {
    fn default() -> BroadcastStatus {
        BroadcastStatus::Pending
    }
}

impl Default for BroadcastChannel {
    fn default() -> BroadcastChannel {
        BroadcastChannel::PushNotification
    }
}

impl Tables {
    pub fn table_name(&self) -> String {
        self.to_string().to_ascii_lowercase()
    }
}

#[test]
fn get_event_limited_roles() {
    assert_eq!(
        Roles::get_event_limited_roles(),
        vec![Roles::Promoter, Roles::PromoterReadOnly]
    );
}

#[test]
fn display() {
    assert_eq!(Roles::Admin.to_string(), "Admin");
    assert_eq!(Roles::OrgAdmin.to_string(), "OrgAdmin");
    assert_eq!(Roles::OrgMember.to_string(), "OrgMember");
    assert_eq!(Roles::OrgOwner.to_string(), "OrgOwner");
    assert_eq!(Roles::OrgBoxOffice.to_string(), "OrgBoxOffice");
    assert_eq!(Roles::DoorPerson.to_string(), "DoorPerson");
    assert_eq!(Roles::Promoter.to_string(), "Promoter");
    assert_eq!(Roles::PromoterReadOnly.to_string(), "PromoterReadOnly");
    assert_eq!(Roles::User.to_string(), "User");
}

#[test]
fn parse() {
    assert_eq!(Roles::Admin, "Admin".parse().unwrap());
    assert_eq!(Roles::OrgMember, "OrgMember".parse().unwrap());
    assert_eq!(Roles::OrgOwner, "OrgOwner".parse().unwrap());
    assert_eq!(Roles::OrgBoxOffice, "OrgBoxOffice".parse().unwrap());
    assert!("Invalid Role".parse::<Roles>().is_err());
}

#[test]
fn to_table_name() {
    assert_eq!(Tables::Events.table_name(), "events");
}
