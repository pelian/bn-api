extern crate chrono;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;

use std::io::Write;

use chrono::{DateTime, Utc};
use env_logger::{Builder, Env};

const DATETIME_FORMAT: &'static str = "[%Y-%m-%d][%H:%M:%S]";

#[derive(Serialize, Debug)]
struct LogEntry {
    level: String,
    #[serde(serialize_with = "custom_datetime_serializer")]
    time: DateTime<Utc>,
    target: Option<String>,
    message: String,
    #[serde(flatten)]
    meta: Option<serde_json::Value>,
}

fn custom_datetime_serializer<S>(x: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(format!("{}", x.format(DATETIME_FORMAT)).as_str())
}

/// A convenience wrapper around the log! macro for writing log messages that ElasticSearch can
/// ingest.
/// You can use the default logging form:
/// `jlog!(log::level::Info, "Log message")`
/// which produces
/// `{"level": "INFO", "target": "my_module", "message":"Log message"}`
/// Or you can provide metadata for ES to use:
/// ```text
///   let val = -1;
///   jlog!(Error, "Amount must be positive", {"value": val})
/// ```
/// which will produce:
/// `{"level": "ERROR", "target": "my_module", "message": "Amount must be positive", "value": -1}`
#[macro_export]
macro_rules! jlog {
    ($t:path, $msg:expr) => {{
        use $crate::transform_message;
        transform_message($t, None, $msg, None)
    }};
    ($t:path, $msg:expr, $json:tt) => {{
        use $crate::transform_message;
        let meta = json!($json);
        transform_message($t, None, $msg, Some(meta))
    }};
    ($t:path, $target: expr, $msg:expr) => {{
        use $crate::transform_message;
        transform_message($t, Some($target), $msg, None)
    }};
    ($t:path, $target: expr, $msg:expr, $json:tt) => {{
        use $crate::transform_message;
        let meta = json!($json);
        transform_message($t, Some($target), $msg, Some(meta))
    }};
}

pub fn transform_message(
    level: log::Level,
    target: Option<&str>,
    msg: &str,
    meta: Option<serde_json::Value>,
) {
    let mut inner = LogEntry {
        level: format!("{}", level),
        target: target.map(|t| t.to_string()),
        time: chrono::Utc::now(),
        message: msg.trim().to_string(),
        meta,
    };
    match target {
        Some(t) => log!(
            target: t,
            level,
            "{}",
            serde_json::to_string(&inner).unwrap()
        ),
        None => {
            let t = std::env::current_exe()
                .map(|p| {
                    p.file_name()
                        .map(|s| s.to_str().unwrap_or("none").to_string())
                })
                .unwrap_or(Some("none".to_string()))
                .unwrap();
            inner.target = Some(t);
            let t = inner.target.as_ref().unwrap().as_str();
            log!(
                target: t,
                level,
                "{}",
                serde_json::to_string(&inner).unwrap()
            )
        }
    }
}

fn is_json(msg: &String) -> bool {
    msg.starts_with("{") && msg.ends_with("}")
}

pub fn setup_logger() {
    Builder::from_env(Env::default().default_filter_or("debug"))
        .format(|buf, record| {
            let msg = format!("{}", record.args()).trim().to_string();
            if !is_json(&msg) {
                let entry = LogEntry {
                    level: record.level().to_string(),
                    time: chrono::Utc::now(),
                    target: Some(record.target().to_string()),
                    message: msg,
                    meta: None,
                };

                match serde_json::to_string(&entry) {
                    Ok(s) => writeln!(buf, "{}", s),
                    Err(err) => writeln!(
                        buf,
                        "Failed to serialize log entry: Error: {:?}, Entry: {:?}",
                        err, entry
                    ),
                }
            } else {
                writeln!(buf, "{}", msg)
            }
        })
        .init();
}

#[cfg(test)]
mod tests {
    use log::Level::*;

    #[test]
    fn test_jlog() {
        // super::setup_logger().unwrap();
        // Level, Message
        jlog!(Warn, "message");
        // Level, message, meta
        jlog!(Warn, "test", {"a": 1} );
        // Level, message, meta
        jlog!(Error, "test", {"a": 1, "b": "jake", "c": [3, 2, 1]});
        // Level, target, message, meta
        jlog!(
            Debug,
            "bigneon::domain_actions",
            "Found no actions to process",
            {}
        );
    }
}
