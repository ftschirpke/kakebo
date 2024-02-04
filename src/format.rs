pub fn format_value(value: i32) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}
