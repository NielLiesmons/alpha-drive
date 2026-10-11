//! Project / Task helpers — encrypted JSON leaves on conventional paths.
//!
//! Tree layout (v0, flat root dir names with `/` separators):
//! - `meta/space.json`
//! - `projects/{id}/project.json`  (kind: project)
//! - `tasks/{id}/task.json`        (kind: task)
//! - `files/…`                     (kind: blob)

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_prefix: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub schema: String,
    pub id: String,
    pub project_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
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
    // Format as …Z via days since epoch — keep it simple: unix timestamp string
    // clients accept; prefer proper UTC date.
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

pub fn task_path(id: &str) -> String {
    format!("tasks/{id}/task.json")
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
    let project = Project {
        schema: "alpha.project/1".into(),
        id: id.clone(),
        slug: slug.clone(),
        title: title.to_string(),
        description: None,
        created_at: created.clone(),
        updated_at: Some(created),
        drive: Some(ProjectDrive {
            private_prefix: Some(format!("projects/{id}/private")),
            public_prefix: Some(format!("projects/{id}/public")),
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
    project_id: &str,
    title: &str,
) -> Result<Task, Box<dyn std::error::Error>> {
    // Ensure project exists (and is decryptable with this key).
    let _ = get_project(base, token, root_key, project_id).await?;
    let id = new_id();
    let created = now_rfc3339();
    let task = Task {
        schema: "alpha.task/1".into(),
        id: id.clone(),
        project_id: project_id.to_string(),
        title: title.to_string(),
        description: None,
        status: "todo".into(),
        milestone_id: None,
        created_at: created.clone(),
        updated_at: Some(created),
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
    Ok(serde_json::from_slice(&bytes)?)
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
                    if let Ok(t) = serde_json::from_slice::<Task>(&bytes) {
                        if project_id.map(|p| p == t.project_id).unwrap_or(true) {
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
