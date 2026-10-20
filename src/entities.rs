//! Project / Task / Note helpers — encrypted JSON leaves on conventional paths.
//!
//! Tree layout (see alpha-docs/DRIVE-LAYOUT.md):
//! - `meta/space.json`
//! - `projects/{id}/project.json`  (kind: project)
//! - `projects/{id}/home/…`        (optional narrative folder)
//! - `tasks/{id}/task.json`        (kind: task)
//! - `notes/{id}/note.json`        (kind: note)
//! - `accounts/{id}/account.json`  (kind: account) — Admin JS client first
//! - `ledger/{id}/entry.json`      (kind: ledger)
//! - `invoices/{id}/invoice.json`  (kind: invoice)
//! - `activities/{id}/activity.json` (kind: activity)
//! - `files/…` `media/…`           (blobs)
//!
//! Public shop catalog is **not** on private Drive — see ADR-accounting-commerce.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub schema: String,
    pub id: String,
    pub slug: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive: Option<ProjectDrive>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail_list_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectDrive {
    /// Narrative / “about this project” folder (not the only way files belong).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_prefix: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub project_ids: Vec<String>,
    /// Deprecated singular from alpha.task/1 — normalized into `project_ids` on read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Optional deadline (schedule lives on `alpha.moment/1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_at: Option<String>,
    /// Legacy alias — folded into `deadline_at` on normalize.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
}

impl Task {
    pub fn normalize(&mut self) {
        if self.project_ids.is_empty() {
            if let Some(p) = self.project_id.take() {
                self.project_ids.push(p);
            }
        } else {
            self.project_id = None;
        }
        // Map legacy alpha.task/1 statuses → /3
        self.status = match self.status.as_str() {
            "todo" => "open".into(),
            "doing" => "inProgress".into(),
            "done" => "closed".into(),
            "cancelled" => "canceled".into(),
            other => other.to_string(),
        };
        if self.deadline_at.is_none() {
            self.deadline_at = self.due_at.take();
        } else {
            self.due_at = None;
        }
        if self.schema == "alpha.task/1" || self.schema == "alpha.task/2" {
            self.schema = "alpha.task/3".into();
        }
    }

    pub fn links_project(&self, id: &str) -> bool {
        self.project_ids.iter().any(|p| p == id)
    }
}

/// Calendar / Planning slice — optional `task_id` (null = free calendar block).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Moment {
    pub schema: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub start_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignees: Vec<String>,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Moment {
    pub fn normalize(&mut self) {
        self.schema = "alpha.moment/1".into();
        if self.status.is_empty() {
            self.status = "planned".into();
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Note {
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub project_ids: Vec<String>,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Note {
    pub fn normalize(&mut self) {
        self.schema = "alpha.note/1".into();
    }

    pub fn links_project(&self, id: &str) -> bool {
        self.project_ids.iter().any(|p| p == id)
    }
}

pub fn new_id() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in title.chars() {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "project".into()
    } else {
        trimmed
    }
}

/// Approximate RFC3339 UTC without pulling in chrono (good enough for v0).
pub fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let tod = secs % 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Howard Hinnant civil-from-days (proleptic Gregorian).
fn civil_from_days(mut z: i64) -> (i32, u32, u32) {
    z += 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub fn project_path(id: &str) -> String {
    format!("projects/{id}/project.json")
}

pub fn project_home_prefix(id: &str) -> String {
    format!("projects/{id}/home")
}

pub fn task_path(id: &str) -> String {
    format!("tasks/{id}/task.json")
}

pub fn moment_path(id: &str) -> String {
    format!("moments/{id}/moment.json")
}

pub fn note_path(id: &str) -> String {
    format!("notes/{id}/note.json")
}

pub async fn create_project(
    base: &str,
    token: &str,
    root_key: &str,
    title: &str,
    slug: Option<&str>,
) -> Result<Project, Box<dyn std::error::Error>> {
    let id = new_id();
    let slug = slug.map(|s| s.to_string()).unwrap_or_else(|| slugify(title));
    let created = now_rfc3339();
    let home = project_home_prefix(&id);
    let project = Project {
        schema: "alpha.project/1".into(),
        id: id.clone(),
        slug: slug.clone(),
        title: title.to_string(),
        description: None,
        created_at: created.clone(),
        updated_at: Some(created),
        drive: Some(ProjectDrive {
            home_prefix: Some(home.clone()),
            private_prefix: Some(home),
            public_prefix: None,
        }),
        mail_list_id: None,
        site_url: None,
    };
    let body = serde_json::to_vec_pretty(&project)?;
    client::put_path(base, token, root_key, &project_path(&id), "project", &body).await?;
    Ok(project)
}

pub async fn get_project(
    base: &str,
    token: &str,
    root_key: &str,
    id: &str,
) -> Result<Project, Box<dyn std::error::Error>> {
    let bytes = client::get_path(base, token, root_key, &project_path(id)).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn list_projects(
    base: &str,
    token: &str,
    root_key: &str,
) -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let dir = client::list_dir(base, token, root_key).await?;
    let mut out = Vec::new();
    for (path, child) in dir.children {
        if child.kind == "project" || path.ends_with("/project.json") {
            match client::get_path(base, token, root_key, &path).await {
                Ok(bytes) => {
                    if let Ok(p) = serde_json::from_slice::<Project>(&bytes) {
                        out.push(p);
                    }
                }
                Err(_) => continue,
            }
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(out)
}

pub async fn create_task(
    base: &str,
    token: &str,
    root_key: &str,
    project_ids: &[String],
    title: &str,
) -> Result<Task, Box<dyn std::error::Error>> {
    for pid in project_ids {
        let _ = get_project(base, token, root_key, pid).await?;
    }
    let id = new_id();
    let created = now_rfc3339();
    let task = Task {
        schema: "alpha.task/3".into(),
        id: id.clone(),
        project_ids: project_ids.to_vec(),
        project_id: None,
        title: title.to_string(),
        description: None,
        status: "open".into(),
        priority: Some("none".into()),
        labels: Vec::new(),
        milestone_id: None,
        created_at: created.clone(),
        updated_at: Some(created),
        deadline_at: None,
        due_at: None,
    };
    let body = serde_json::to_vec_pretty(&task)?;
    client::put_path(base, token, root_key, &task_path(&id), "task", &body).await?;
    Ok(task)
}

pub async fn get_task(
    base: &str,
    token: &str,
    root_key: &str,
    id: &str,
) -> Result<Task, Box<dyn std::error::Error>> {
    let bytes = client::get_path(base, token, root_key, &task_path(id)).await?;
    let mut task: Task = serde_json::from_slice(&bytes)?;
    task.normalize();
    Ok(task)
}

pub async fn put_task(
    base: &str,
    token: &str,
    root_key: &str,
    task: &Task,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut t = task.clone();
    t.normalize();
    t.schema = "alpha.task/3".into();
    t.updated_at = Some(now_rfc3339());
    let body = serde_json::to_vec_pretty(&t)?;
    client::put_path(base, token, root_key, &task_path(&t.id), "task", &body).await?;
    Ok(())
}

pub async fn create_moment(
    base: &str,
    token: &str,
    root_key: &str,
    task_id: Option<&str>,
    title: Option<&str>,
    start_at: &str,
    end_at: Option<&str>,
) -> Result<Moment, Box<dyn std::error::Error>> {
    if let Some(tid) = task_id {
        let _ = get_task(base, token, root_key, tid).await?;
    }
    let id = new_id();
    let created = now_rfc3339();
    let moment = Moment {
        schema: "alpha.moment/1".into(),
        id: id.clone(),
        task_id: task_id.map(|s| s.to_string()),
        title: title.map(|s| s.to_string()),
        start_at: start_at.to_string(),
        end_at: end_at.map(|s| s.to_string()),
        assignees: Vec::new(),
        status: "planned".into(),
        created_at: created.clone(),
        updated_at: Some(created),
    };
    let body = serde_json::to_vec_pretty(&moment)?;
    client::put_path(base, token, root_key, &moment_path(&id), "moment", &body).await?;
    Ok(moment)
}

pub async fn list_moments(
    base: &str,
    token: &str,
    root_key: &str,
    task_id: Option<&str>,
) -> Result<Vec<Moment>, Box<dyn std::error::Error>> {
    let dir = client::list_dir(base, token, root_key).await?;
    let mut out = Vec::new();
    for (path, child) in dir.children {
        if child.kind == "moment" || path.ends_with("/moment.json") {
            match client::get_path(base, token, root_key, &path).await {
                Ok(bytes) => {
                    if let Ok(mut m) = serde_json::from_slice::<Moment>(&bytes) {
                        m.normalize();
                        if task_id.map(|t| m.task_id.as_deref() == Some(t)).unwrap_or(true) {
                            out.push(m);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }
    out.sort_by(|a, b| a.start_at.cmp(&b.start_at));
    Ok(out)
}

pub async fn list_tasks(
    base: &str,
    token: &str,
    root_key: &str,
    project_id: Option<&str>,
) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let dir = client::list_dir(base, token, root_key).await?;
    let mut out = Vec::new();
    for (path, child) in dir.children {
        if child.kind == "task" || path.ends_with("/task.json") {
            match client::get_path(base, token, root_key, &path).await {
                Ok(bytes) => {
                    if let Ok(mut t) = serde_json::from_slice::<Task>(&bytes) {
                        t.normalize();
                        if project_id.map(|p| t.links_project(p)).unwrap_or(true) {
                            out.push(t);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(out)
}

pub async fn create_note(
    base: &str,
    token: &str,
    root_key: &str,
    project_ids: &[String],
    title: &str,
    body: &str,
) -> Result<Note, Box<dyn std::error::Error>> {
    for pid in project_ids {
        let _ = get_project(base, token, root_key, pid).await?;
    }
    let id = new_id();
    let created = now_rfc3339();
    let note = Note {
        schema: "alpha.note/1".into(),
        id: id.clone(),
        project_ids: project_ids.to_vec(),
        title: title.to_string(),
        body: body.to_string(),
        labels: Vec::new(),
        created_at: created.clone(),
        updated_at: Some(created),
    };
    let bytes = serde_json::to_vec_pretty(&note)?;
    client::put_path(base, token, root_key, &note_path(&id), "note", &bytes).await?;
    Ok(note)
}

pub async fn get_note(
    base: &str,
    token: &str,
    root_key: &str,
    id: &str,
) -> Result<Note, Box<dyn std::error::Error>> {
    let bytes = client::get_path(base, token, root_key, &note_path(id)).await?;
    let mut note: Note = serde_json::from_slice(&bytes)?;
    note.normalize();
    Ok(note)
}

pub async fn put_note(
    base: &str,
    token: &str,
    root_key: &str,
    note: &Note,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut n = note.clone();
    n.normalize();
    n.schema = "alpha.note/1".into();
    n.updated_at = Some(now_rfc3339());
    let bytes = serde_json::to_vec_pretty(&n)?;
    client::put_path(base, token, root_key, &note_path(&n.id), "note", &bytes).await?;
    Ok(())
}

pub async fn list_notes(
    base: &str,
    token: &str,
    root_key: &str,
    project_id: Option<&str>,
) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    let dir = client::list_dir(base, token, root_key).await?;
    let mut out = Vec::new();
    for (path, child) in dir.children {
        if child.kind == "note" || path.ends_with("/note.json") {
            match client::get_path(base, token, root_key, &path).await {
                Ok(bytes) => {
                    if let Ok(mut n) = serde_json::from_slice::<Note>(&bytes) {
                        n.normalize();
                        if project_id.map(|p| n.links_project(p)).unwrap_or(true) {
                            out.push(n);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(out)
}

pub async fn init_space(
    base: &str,
    token: &str,
    root_key: &str,
    name: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let doc = json!({
        "schema": "alpha.space/1",
        "name": name,
        "created_at": now_rfc3339(),
        "note": "v0 family root capability — one key on all devices"
    });
    let body = serde_json::to_vec_pretty(&doc)?;
    client::put_path(base, token, root_key, "meta/space.json", "blob", &body).await?;
    Ok(doc)
}
