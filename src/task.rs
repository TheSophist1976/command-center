use chrono::{DateTime, Datelike, Local, NaiveDate, Utc, Weekday};
use serde::{Serialize, Serializer};

// -- Recurrence --

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntervalUnit {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Recurrence {
    Interval { unit: IntervalUnit, count: u32 },
    NthWeekday { n: u8, weekday: Weekday },
    WeeklyOn { weekday: Weekday, every_n_weeks: u32 },
}

impl std::fmt::Display for Recurrence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Recurrence::Interval { unit, count } => {
                let name = match unit {
                    IntervalUnit::Daily => "daily",
                    IntervalUnit::Weekly => "weekly",
                    IntervalUnit::Monthly => "monthly",
                    IntervalUnit::Yearly => "yearly",
                };
                if *count == 1 {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}:{}", name, count)
                }
            }
            Recurrence::NthWeekday { n, weekday } => {
                let day = weekday_abbrev(*weekday);
                write!(f, "monthly:{}:{}", n, day)
            }
            Recurrence::WeeklyOn { weekday, every_n_weeks } => {
                let day = weekday_abbrev(*weekday);
                if *every_n_weeks == 1 {
                    write!(f, "weekly:{}", day)
                } else {
                    write!(f, "weekly:{}:{}", every_n_weeks, day)
                }
            }
        }
    }
}

impl std::str::FromStr for Recurrence {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        let parts: Vec<&str> = lower.split(':').collect();

        // Simple unit with no count: "daily", "weekly", "monthly", "yearly"
        if parts.len() == 1 {
            let unit = match parts[0] {
                "daily" => IntervalUnit::Daily,
                "weekly" => IntervalUnit::Weekly,
                "monthly" => IntervalUnit::Monthly,
                "yearly" => IntervalUnit::Yearly,
                _ => return Err(format!(
                    "Invalid recurrence: '{}'. Valid: daily, weekly, monthly, yearly, daily:N, weekly:N, monthly:N, yearly:N, weekly:DAY, weekly:N:DAY, monthly:N:DAY",
                    s
                )),
            };
            return Ok(Recurrence::Interval { unit, count: 1 });
        }

        // Two parts: "unit:count" or "weekly:DAY"
        if parts.len() == 2 {
            // Check if "weekly:DAY" (WeeklyOn shorthand)
            if parts[0] == "weekly" {
                if let Some(weekday) = parse_weekday(parts[1]) {
                    return Ok(Recurrence::WeeklyOn { weekday, every_n_weeks: 1 });
                }
            }
            let unit = match parts[0] {
                "daily" => IntervalUnit::Daily,
                "weekly" => IntervalUnit::Weekly,
                "monthly" => IntervalUnit::Monthly,
                "yearly" => IntervalUnit::Yearly,
                _ => return Err(format!("Invalid recurrence: '{}'. Unknown unit.", s)),
            };
            let count: u32 = parts[1].parse().map_err(|_| {
                format!("Invalid recurrence: '{}'. Count must be a number.", s)
            })?;
            if count == 0 {
                return Err(format!("Invalid recurrence: '{}'. Count must be >= 1.", s));
            }
            return Ok(Recurrence::Interval { unit, count });
        }

        // Three parts: "weekly:N:DAY" (WeeklyOn) or "monthly:N:DAY" (NthWeekday)
        if parts.len() == 3 && parts[0] == "weekly" {
            let count: u32 = parts[1].parse().map_err(|_| {
                format!("Invalid recurrence: '{}'. N must be a number.", s)
            })?;
            if count == 0 {
                return Err(format!("Invalid recurrence: '{}'. N must be >= 1.", s));
            }
            let weekday = parse_weekday(parts[2])
                .ok_or_else(|| format!("Invalid recurrence: '{}'. Unknown weekday.", s))?;
            return Ok(Recurrence::WeeklyOn { weekday, every_n_weeks: count });
        }

        if parts.len() == 3 && parts[0] == "monthly" {
            let n: u8 = parts[1].parse().map_err(|_| {
                format!("Invalid recurrence: '{}'. N must be a number.", s)
            })?;
            if n == 0 || n > 5 {
                return Err(format!("Invalid recurrence: '{}'. N must be 1-5.", s));
            }
            let weekday = parse_weekday(parts[2])
                .ok_or_else(|| format!("Invalid recurrence: '{}'. Unknown weekday.", s))?;
            return Ok(Recurrence::NthWeekday { n, weekday });
        }

        Err(format!(
            "Invalid recurrence: '{}'. Valid: daily, weekly, monthly, yearly, daily:N, weekly:N, monthly:N, yearly:N, weekly:DAY, weekly:N:DAY, monthly:N:DAY",
            s
        ))
    }
}

fn weekday_abbrev(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" => Some(Weekday::Mon),
        "tue" => Some(Weekday::Tue),
        "wed" => Some(Weekday::Wed),
        "thu" => Some(Weekday::Thu),
        "fri" => Some(Weekday::Fri),
        "sat" => Some(Weekday::Sat),
        "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Calculate the next due date for a recurring task.
/// If `current_due` is None, calculates from today.
pub fn next_due_date(recurrence: &Recurrence, current_due: Option<NaiveDate>) -> NaiveDate {
    let base = current_due.unwrap_or_else(|| Local::now().date_naive());
    match recurrence {
        Recurrence::Interval { unit, count } => match unit {
            IntervalUnit::Daily => base + chrono::Duration::days(*count as i64),
            IntervalUnit::Weekly => base + chrono::Duration::weeks(*count as i64),
            IntervalUnit::Monthly => add_months(base, *count),
            IntervalUnit::Yearly => add_months(base, *count * 12),
        },
        Recurrence::NthWeekday { n, weekday } => {
            find_next_nth_weekday(base, *n, *weekday)
        }
        Recurrence::WeeklyOn { weekday, every_n_weeks } => {
            if base.weekday() == *weekday {
                // Base is already the target weekday, advance by N weeks
                base + chrono::Duration::weeks(*every_n_weeks as i64)
            } else {
                // Find next occurrence of the target weekday
                let days_ahead = (weekday.num_days_from_monday() as i64
                    - base.weekday().num_days_from_monday() as i64
                    + 7) % 7;
                base + chrono::Duration::days(days_ahead)
            }
        }
    }
}

/// Add N months to a date, clamping to month-end if needed.
fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total_months = date.month0() + months;
    let new_year = date.year() + (total_months / 12) as i32;
    let new_month = (total_months % 12) + 1;
    // Try original day, clamp down if it doesn't exist
    let max_day = days_in_month(new_year, new_month);
    let new_day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, new_day).unwrap()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    // Get the first day of the next month, then subtract one day
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .pred_opt()
    .unwrap()
    .day()
}

/// Find the nth occurrence of a weekday in the next month(s) after `base`.
fn find_next_nth_weekday(base: NaiveDate, n: u8, weekday: Weekday) -> NaiveDate {
    // Start searching from the month after base
    let mut year = base.year();
    let mut month = base.month();
    // Move to next month
    if month == 12 {
        year += 1;
        month = 1;
    } else {
        month += 1;
    }
    // Search up to 12 months for the nth weekday
    for _ in 0..12 {
        if let Some(date) = nth_weekday_in_month(year, month, n, weekday) {
            return date;
        }
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    // Fallback (should not happen for n<=5)
    base + chrono::Duration::days(30)
}

/// Find the nth occurrence of a weekday in a given month, or None if it doesn't exist.
fn nth_weekday_in_month(year: i32, month: u32, n: u8, weekday: Weekday) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_weekday = first.weekday();
    // Days until the first occurrence of the target weekday
    let days_ahead = (weekday.num_days_from_monday() as i64
        - first_weekday.num_days_from_monday() as i64
        + 7) % 7;
    let first_occurrence = first + chrono::Duration::days(days_ahead);
    let target = first_occurrence + chrono::Duration::weeks((n - 1) as i64);
    // Check it's still in the same month
    if target.month() == month {
        Some(target)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub tags: Vec<String>,
    #[serde(serialize_with = "serialize_datetime")]
    pub created: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_datetime")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_date")]
    pub due_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing)]
    pub recurrence: Option<Recurrence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFile {
    pub format_version: u32,
    pub next_id: u32,
    pub tasks: Vec<Task>,
}

impl TaskFile {
    pub fn new() -> Self {
        Self {
            format_version: 1,
            next_id: 1,
            tasks: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn find_task(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn find_task_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    #[cfg(test)]
    pub fn remove_task(&mut self, id: u32) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Open,
    Done,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::Done => write!(f, "done"),
        }
    }
}

impl std::str::FromStr for Status {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Status::Open),
            "done" => Ok(Status::Done),
            _ => Err(format!("Invalid status: '{}'. Valid values: open, done", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" | "crit" => Ok(Priority::Critical),
            "high" => Ok(Priority::High),
            "medium" | "med" => Ok(Priority::Medium),
            "low" => Ok(Priority::Low),
            _ => Err(format!("Invalid priority: '{}'. Valid values: critical, high, medium, low", s)),
        }
    }
}

fn serialize_datetime<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&dt.to_rfc3339())
}

fn serialize_option_datetime<S>(dt: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => serializer.serialize_str(&dt.to_rfc3339()),
        None => serializer.serialize_none(),
    }
}

fn serialize_option_date<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(d) => serializer.serialize_str(&d.format("%Y-%m-%d").to_string()),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_task(id: u32, title: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            note: None,
            agent: None,
        }
    }

    // -- Status tests --

    #[test]
    fn test_status_display_open() {
        assert_eq!(format!("{}", Status::Open), "open");
    }

    #[test]
    fn test_status_display_done() {
        assert_eq!(format!("{}", Status::Done), "done");
    }

    #[test]
    fn test_status_from_str_open() {
        let s: Status = "open".parse().unwrap();
        assert_eq!(s, Status::Open);
    }

    #[test]
    fn test_status_from_str_done() {
        let s: Status = "done".parse().unwrap();
        assert_eq!(s, Status::Done);
    }

    #[test]
    fn test_status_from_str_case_insensitive() {
        let s: Status = "OPEN".parse().unwrap();
        assert_eq!(s, Status::Open);
        let s2: Status = "Done".parse().unwrap();
        assert_eq!(s2, Status::Done);
    }

    #[test]
    fn test_status_from_str_invalid() {
        let result: Result<Status, _> = "pending".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid status"));
    }

    // -- Priority tests --

    #[test]
    fn test_priority_display_critical() {
        assert_eq!(format!("{}", Priority::Critical), "critical");
    }

    #[test]
    fn test_priority_display_high() {
        assert_eq!(format!("{}", Priority::High), "high");
    }

    #[test]
    fn test_priority_display_medium() {
        assert_eq!(format!("{}", Priority::Medium), "medium");
    }

    #[test]
    fn test_priority_display_low() {
        assert_eq!(format!("{}", Priority::Low), "low");
    }

    #[test]
    fn test_priority_from_str_all_variants() {
        let cases = [
            ("critical", Priority::Critical),
            ("crit", Priority::Critical),
            ("high", Priority::High),
            ("medium", Priority::Medium),
            ("med", Priority::Medium),
            ("low", Priority::Low),
        ];
        for (input, expected) in cases {
            let p: Priority = input.parse().unwrap();
            assert_eq!(p, expected, "failed for input: {}", input);
        }
    }

    #[test]
    fn test_priority_from_str_case_insensitive() {
        let p: Priority = "HIGH".parse().unwrap();
        assert_eq!(p, Priority::High);
    }

    #[test]
    fn test_priority_from_str_invalid() {
        let result: Result<Priority, _> = "urgent".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid priority"));
    }

    // -- Priority ordering tests --

    #[test]
    fn test_priority_ord_descending() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Medium);
        assert!(Priority::Medium < Priority::Low);
    }

    #[test]
    fn test_priority_sorted_descending() {
        let mut priorities = vec![Priority::Low, Priority::Critical, Priority::Medium, Priority::High];
        priorities.sort();
        assert_eq!(priorities, vec![Priority::Critical, Priority::High, Priority::Medium, Priority::Low]);
    }

    // -- TaskFile tests --

    #[test]
    fn test_task_file_new() {
        let tf = TaskFile::new();
        assert_eq!(tf.format_version, 1);
        assert_eq!(tf.next_id, 1);
        assert!(tf.tasks.is_empty());
    }

    #[test]
    fn test_find_task_found() {
        let mut tf = TaskFile::new();
        tf.tasks.push(make_task(1, "First"));
        tf.tasks.push(make_task(2, "Second"));
        let t = tf.find_task(2).unwrap();
        assert_eq!(t.title, "Second");
    }

    #[test]
    fn test_find_task_not_found() {
        let tf = TaskFile::new();
        assert!(tf.find_task(99).is_none());
    }

    #[test]
    fn test_find_task_mut() {
        let mut tf = TaskFile::new();
        tf.tasks.push(make_task(1, "Task"));
        let t = tf.find_task_mut(1).unwrap();
        t.title = "Updated".to_string();
        assert_eq!(tf.tasks[0].title, "Updated");
    }

    #[test]
    fn test_find_task_mut_not_found() {
        let mut tf = TaskFile::new();
        assert!(tf.find_task_mut(42).is_none());
    }

    #[test]
    fn test_remove_task_existing() {
        let mut tf = TaskFile::new();
        tf.tasks.push(make_task(1, "To remove"));
        tf.tasks.push(make_task(2, "Keep"));
        let removed = tf.remove_task(1).unwrap();
        assert_eq!(removed.title, "To remove");
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].id, 2);
    }

    #[test]
    fn test_remove_task_not_found() {
        let mut tf = TaskFile::new();
        assert!(tf.remove_task(99).is_none());
    }

    // -- Serialize tests (via serde_json) --

    #[test]
    fn test_task_serialize_basic() {
        let task = make_task(1, "Test task");
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"title\":\"Test task\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"status\":\"open\""));
        assert!(json.contains("\"priority\":\"medium\""));
        // None fields skipped
        assert!(!json.contains("\"updated\""));
        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"due_date\""));
        assert!(!json.contains("\"project\""));
    }

    #[test]
    fn test_task_serialize_with_optional_fields() {
        let mut task = make_task(1, "Full task");
        task.updated = Some(Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap());
        task.description = Some("A description".to_string());
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
        task.project = Some("myproject".to_string());
        task.tags = vec!["alpha".to_string(), "beta".to_string()];

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"updated\""));
        assert!(json.contains("\"description\":\"A description\""));
        assert!(json.contains("\"due_date\":\"2025-12-31\""));
        assert!(json.contains("\"project\":\"myproject\""));
        assert!(json.contains("\"alpha\""));
    }

    #[test]
    fn test_status_serialize_json() {
        let json = serde_json::to_string(&Status::Open).unwrap();
        assert_eq!(json, "\"open\"");
        let json2 = serde_json::to_string(&Status::Done).unwrap();
        assert_eq!(json2, "\"done\"");
    }

    #[test]
    fn test_priority_serialize_json() {
        assert_eq!(serde_json::to_string(&Priority::Critical).unwrap(), "\"critical\"");
        assert_eq!(serde_json::to_string(&Priority::High).unwrap(), "\"high\"");
        assert_eq!(serde_json::to_string(&Priority::Medium).unwrap(), "\"medium\"");
        assert_eq!(serde_json::to_string(&Priority::Low).unwrap(), "\"low\"");
    }

    #[test]
    fn test_task_clone_and_eq() {
        let t1 = make_task(1, "Task");
        let t2 = t1.clone();
        assert_eq!(t1, t2);
    }

    // -- Private serializer function tests --
    // These test the None branches of the option serializers

    #[test]
    fn test_serialize_option_datetime_none_branch() {
        // Call the private serialize_option_datetime with None
        let none_dt: Option<DateTime<Utc>> = None;
        let json = serde_json::to_string(&none_dt).unwrap();
        // When None, serde uses serialize_none -> JSON null
        assert_eq!(json, "null");
    }

    #[test]
    fn test_serialize_option_date_none_branch() {
        // Call the private serialize_option_date with None
        let none_date: Option<chrono::NaiveDate> = None;
        let json = serde_json::to_string(&none_date).unwrap();
        assert_eq!(json, "null");
    }

    // Force the serialize_option_datetime and serialize_option_date None paths via a custom wrapper
    // The Task struct uses skip_serializing_if which prevents None from reaching the serializer,
    // so we test the underlying functions directly by creating a wrapper struct without skip_serializing_if
    #[derive(serde::Serialize)]
    struct TestOptionDateTime {
        #[serde(serialize_with = "super::serialize_option_datetime")]
        val: Option<DateTime<Utc>>,
    }

    #[derive(serde::Serialize)]
    struct TestOptionDate {
        #[serde(serialize_with = "super::serialize_option_date")]
        val: Option<NaiveDate>,
    }

    #[test]
    fn test_serialize_option_datetime_none_via_wrapper() {
        let wrapper = TestOptionDateTime { val: None };
        let json = serde_json::to_string(&wrapper).unwrap();
        assert_eq!(json, "{\"val\":null}");
    }

    #[test]
    fn test_serialize_option_datetime_some_via_wrapper() {
        let wrapper = TestOptionDateTime {
            val: Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
        };
        let json = serde_json::to_string(&wrapper).unwrap();
        assert!(json.contains("2025-01-01T00:00:00"));
    }

    #[test]
    fn test_serialize_option_date_none_via_wrapper() {
        let wrapper = TestOptionDate { val: None };
        let json = serde_json::to_string(&wrapper).unwrap();
        assert_eq!(json, "{\"val\":null}");
    }

    #[test]
    fn test_serialize_option_date_some_via_wrapper() {
        let wrapper = TestOptionDate {
            val: Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
        };
        let json = serde_json::to_string(&wrapper).unwrap();
        assert!(json.contains("2025-12-31"));
    }

    // -- Recurrence tests --

    #[test]
    fn test_recurrence_parse_simple_intervals() {
        assert_eq!("daily".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 });
        assert_eq!("weekly".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 });
        assert_eq!("monthly".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Monthly, count: 1 });
        assert_eq!("yearly".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Yearly, count: 1 });
    }

    #[test]
    fn test_recurrence_parse_case_insensitive() {
        assert_eq!("WEEKLY".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 });
        assert_eq!("Monthly:3:Thu".parse::<Recurrence>().unwrap(), Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu });
    }

    #[test]
    fn test_recurrence_parse_nth_weekday() {
        let r: Recurrence = "monthly:3:thu".parse().unwrap();
        assert_eq!(r, Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu });
    }

    #[test]
    fn test_recurrence_parse_all_weekdays() {
        for (s, w) in [("mon", Weekday::Mon), ("tue", Weekday::Tue), ("wed", Weekday::Wed),
                        ("thu", Weekday::Thu), ("fri", Weekday::Fri), ("sat", Weekday::Sat), ("sun", Weekday::Sun)] {
            let r: Recurrence = format!("monthly:1:{}", s).parse().unwrap();
            assert_eq!(r, Recurrence::NthWeekday { n: 1, weekday: w });
        }
    }

    #[test]
    fn test_recurrence_parse_invalid() {
        assert!("biweekly".parse::<Recurrence>().is_err());
        assert!("monthly:0:thu".parse::<Recurrence>().is_err());
        assert!("monthly:6:thu".parse::<Recurrence>().is_err());
        assert!("monthly:abc:thu".parse::<Recurrence>().is_err());
        assert!("monthly:1:xyz".parse::<Recurrence>().is_err());
    }

    #[test]
    fn test_recurrence_display() {
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 }.to_string(), "daily");
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 }.to_string(), "weekly");
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Monthly, count: 1 }.to_string(), "monthly");
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Yearly, count: 1 }.to_string(), "yearly");
        assert_eq!(Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu }.to_string(), "monthly:3:thu");
    }

    #[test]
    fn test_recurrence_roundtrip() {
        for s in &["daily", "weekly", "monthly", "yearly", "monthly:1:mon", "monthly:5:fri",
                    "weekly:fri", "weekly:2:mon"] {
            let r: Recurrence = s.parse().unwrap();
            assert_eq!(r.to_string(), *s);
        }
    }

    #[test]
    fn test_weekly_on_parse_shorthand() {
        let r: Recurrence = "weekly:fri".parse().unwrap();
        assert_eq!(r, Recurrence::WeeklyOn { weekday: Weekday::Fri, every_n_weeks: 1 });
    }

    #[test]
    fn test_weekly_on_parse_with_count() {
        let r: Recurrence = "weekly:2:mon".parse().unwrap();
        assert_eq!(r, Recurrence::WeeklyOn { weekday: Weekday::Mon, every_n_weeks: 2 });
    }

    #[test]
    fn test_weekly_on_parse_count_one_explicit() {
        let r: Recurrence = "weekly:1:wed".parse().unwrap();
        assert_eq!(r, Recurrence::WeeklyOn { weekday: Weekday::Wed, every_n_weeks: 1 });
    }

    #[test]
    fn test_weekly_count_still_parses_as_interval() {
        let r: Recurrence = "weekly:2".parse().unwrap();
        assert_eq!(r, Recurrence::Interval { unit: IntervalUnit::Weekly, count: 2 });
    }

    #[test]
    fn test_weekly_on_display_count_1() {
        let r = Recurrence::WeeklyOn { weekday: Weekday::Fri, every_n_weeks: 1 };
        assert_eq!(r.to_string(), "weekly:fri");
    }

    #[test]
    fn test_weekly_on_display_count_gt_1() {
        let r = Recurrence::WeeklyOn { weekday: Weekday::Mon, every_n_weeks: 2 };
        assert_eq!(r.to_string(), "weekly:2:mon");
    }

    #[test]
    fn test_weekly_on_invalid_weekday() {
        let result: Result<Recurrence, _> = "weekly:xyz".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_next_due_date_weekly_on_same_weekday() {
        // 2026-03-06 is a Friday
        let due = NaiveDate::from_ymd_opt(2026, 3, 6).unwrap();
        let next = next_due_date(
            &Recurrence::WeeklyOn { weekday: Weekday::Fri, every_n_weeks: 1 },
            Some(due),
        );
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 3, 13).unwrap());
    }

    #[test]
    fn test_next_due_date_weekly_on_different_weekday() {
        // 2026-03-04 is a Wednesday, next Friday is 2026-03-06
        let due = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap();
        let next = next_due_date(
            &Recurrence::WeeklyOn { weekday: Weekday::Fri, every_n_weeks: 1 },
            Some(due),
        );
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 3, 6).unwrap());
    }

    #[test]
    fn test_next_due_date_biweekly_on_same_weekday() {
        // 2026-03-02 is a Monday, 2 weeks later is 2026-03-16
        let due = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        let next = next_due_date(
            &Recurrence::WeeklyOn { weekday: Weekday::Mon, every_n_weeks: 2 },
            Some(due),
        );
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 3, 16).unwrap());
    }

    #[test]
    fn test_next_due_date_weekly() {
        let due = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 }, Some(due));
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 3, 9).unwrap());
    }

    #[test]
    fn test_next_due_date_monthly_clamp() {
        let due = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Monthly, count: 1 }, Some(due));
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn test_next_due_date_yearly_leap() {
        let due = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Yearly, count: 1 }, Some(due));
        assert_eq!(next, NaiveDate::from_ymd_opt(2025, 2, 28).unwrap());
    }

    #[test]
    fn test_next_due_date_daily() {
        let due = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 }, Some(due));
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 3, 3).unwrap());
    }

    #[test]
    fn test_next_due_date_no_due_date() {
        let today = Local::now().date_naive();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 }, None);
        assert_eq!(next, today + chrono::Duration::days(1));
    }

    #[test]
    fn test_next_due_date_nth_weekday() {
        // 3rd Thu of March 2026 is March 19
        let due = NaiveDate::from_ymd_opt(2026, 3, 19).unwrap();
        let next = next_due_date(&Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu }, Some(due));
        // 3rd Thu of April 2026 is April 16
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 4, 16).unwrap());
    }

    #[test]
    fn test_next_due_date_nth_weekday_skip_month() {
        // 5th Fri of January 2026 is Jan 30
        let due = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        let next = next_due_date(&Recurrence::NthWeekday { n: 5, weekday: Weekday::Fri }, Some(due));
        // February 2026 has no 5th Friday, so skip to next month with one
        // May 2026: 1st Fri is May 1, 5th Fri is May 29
        assert!(next > due);
        assert_eq!(next.weekday(), Weekday::Fri);
    }

    #[test]
    fn test_parse_interval_with_count() {
        assert_eq!("monthly:3".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Monthly, count: 3 });
        assert_eq!("weekly:2".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Weekly, count: 2 });
        assert_eq!("daily:5".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Daily, count: 5 });
        assert_eq!("yearly:2".parse::<Recurrence>().unwrap(), Recurrence::Interval { unit: IntervalUnit::Yearly, count: 2 });
    }

    #[test]
    fn test_display_interval_with_count() {
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Monthly, count: 3 }.to_string(), "monthly:3");
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Weekly, count: 2 }.to_string(), "weekly:2");
        assert_eq!(Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 }.to_string(), "daily");
    }

    #[test]
    fn test_next_due_date_monthly_count_3() {
        let due = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Monthly, count: 3 }, Some(due));
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_next_due_date_daily_count_5() {
        let due = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let next = next_due_date(&Recurrence::Interval { unit: IntervalUnit::Daily, count: 5 }, Some(due));
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 3, 6).unwrap());
    }

    #[test]
    fn test_parse_count_zero_error() {
        assert!("daily:0".parse::<Recurrence>().is_err());
    }

    #[test]
    fn test_roundtrip_with_count() {
        for s in &["daily:3", "weekly:2", "monthly:3", "yearly:2"] {
            let r: Recurrence = s.parse().unwrap();
            assert_eq!(r.to_string(), *s);
        }
    }
}
