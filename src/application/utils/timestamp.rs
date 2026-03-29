use chrono::Utc;

pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339()
}
