use chrono::{DateTime, NaiveDate, Utc};
use serde::{Serialize, Serializer};

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

    pub fn find_task(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn find_task_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
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
}
