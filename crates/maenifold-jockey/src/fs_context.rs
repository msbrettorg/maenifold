use crate::types::*;
use std::collections::VecDeque;
use std::io::Read;

pub(crate) fn resolve_attach_path(cwd: &str, raw: &str) -> std::path::PathBuf {
    let input = raw.trim();
    let expanded = if input == "~" || input.starts_with("~/") {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        if input == "~" {
            home
        } else {
            format!("{home}{}", &input[1..])
        }
    } else {
        input.to_string()
    };
    let candidate = std::path::PathBuf::from(expanded);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        std::path::PathBuf::from(cwd).join(candidate)
    };
    joined.canonicalize().unwrap_or(joined)
}

fn workspace_root(cwd: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(cwd)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(cwd))
}

pub(crate) fn is_within_workspace(path: &std::path::Path, cwd: &str) -> bool {
    path.starts_with(workspace_root(cwd))
}

pub(crate) fn relative_or_abs(path: &std::path::Path, cwd: &str) -> String {
    let cwd_path = std::path::Path::new(cwd);
    if let Ok(relative) = path.strip_prefix(cwd_path) {
        return relative.to_string_lossy().to_string();
    }
    path.to_string_lossy().to_string()
}

pub(crate) fn should_skip_name(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    matches!(name, "node_modules" | "dist" | "build" | "target")
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    if bytes.contains(&0) {
        return true;
    }
    let mut suspicious = 0usize;
    for b in bytes {
        let is_control = (*b < 0x09) || (*b > 0x0D && *b < 0x20);
        if is_control {
            suspicious += 1;
        }
    }
    suspicious * 10 > bytes.len()
}

pub(crate) fn read_text_snippet(
    path: &std::path::Path,
    limit: usize,
) -> Result<(String, usize, bool), String> {
    if limit == 0 {
        return Err("attachment budget exhausted".to_string());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let read_limit = limit.saturating_add(1) as u64;
    let mut reader = file.take(read_limit);
    let mut bytes = Vec::with_capacity(limit.min(4096).saturating_add(1));
    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let inspect = bytes.len().min(4096);
    if looks_binary(&bytes[..inspect]) {
        return Err("binary file skipped".to_string());
    }
    let truncated = bytes.len() > limit;
    let take = if truncated { limit } else { bytes.len() };
    let mut text = String::from_utf8_lossy(&bytes[..take]).to_string();
    if truncated {
        text.push_str("\n...[truncated]");
    }
    Ok((text, take, truncated))
}

pub(crate) fn collect_dir_files(
    root: &std::path::Path,
    max_files: usize,
    max_depth: usize,
) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0usize));

    while let Some((dir, depth)) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut dirs = Vec::new();
        let mut local_files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip_name(&name) {
                continue;
            }
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                if depth < max_depth {
                    dirs.push(path);
                }
                continue;
            }
            if ft.is_file() {
                local_files.push(path);
            }
        }

        dirs.sort();
        local_files.sort();

        for file in local_files {
            files.push(file);
            if files.len() >= max_files {
                return files;
            }
        }

        for child in dirs {
            queue.push_back((child, depth + 1));
        }
    }

    files
}

pub(crate) async fn attach_file_context(
    cwd: String,
    raw_ref: String,
    budget: usize,
) -> Result<(String, String, usize), String> {
    tokio::task::spawn_blocking(move || {
        let path = resolve_attach_path(&cwd, &raw_ref);
        if !is_within_workspace(&path, &cwd) {
            return Err(format!("path is outside workspace: {}", raw_ref));
        }
        if !path.exists() {
            return Err(format!("file not found: {}", raw_ref));
        }
        if path.is_dir() {
            return Err(format!("path is directory (use @dir:): {}", raw_ref));
        }
        let per_file_budget = budget.min(ATTACH_MAX_FILE_BYTES);
        let (snippet, used, _) = read_text_snippet(&path, per_file_budget)?;
        let label = relative_or_abs(&path, &cwd);
        Ok((format!("file:{label}"), snippet, used))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub(crate) async fn attach_dir_context(
    cwd: String,
    raw_ref: String,
    budget: usize,
) -> Result<(String, String, usize), String> {
    if budget == 0 {
        return Err("attachment budget exhausted".to_string());
    }
    tokio::task::spawn_blocking(move || {
        let normalized_ref = if let Some(prefix) = raw_ref.strip_suffix("/**") {
            prefix
        } else {
            &raw_ref
        };
        let path = resolve_attach_path(&cwd, normalized_ref);
        if !is_within_workspace(&path, &cwd) {
            return Err(format!("path is outside workspace: {}", raw_ref));
        }
        if !path.exists() {
            return Err(format!("directory not found: {}", raw_ref));
        }
        if !path.is_dir() {
            return Err(format!("path is file (use @file:): {}", raw_ref));
        }

        let files = collect_dir_files(&path, ATTACH_MAX_DIR_FILES, ATTACH_MAX_DIR_DEPTH);
        if files.is_empty() {
            return Err(format!("directory is empty or unreadable: {}", raw_ref));
        }

        let label = relative_or_abs(&path, &cwd);
        let mut body = format!(
            "Directory: {label}\nFiles (sampled up to {}):",
            ATTACH_MAX_DIR_FILES
        );
        let mut used = body.len();
        for p in &files {
            let rel = relative_or_abs(p, &cwd);
            body.push_str(&format!("\n- {rel}"));
        }
        used += files
            .iter()
            .map(|p| relative_or_abs(p, &cwd).len() + 3)
            .sum::<usize>();
        body.push_str("\n\nContents:");

        for p in files {
            if used >= budget {
                body.push_str("\n...[directory attachment truncated by budget]");
                break;
            }
            let remaining = (budget - used).min(ATTACH_MAX_FILE_BYTES);
            let Ok((snippet, consumed, _)) = read_text_snippet(&p, remaining) else {
                continue;
            };
            if snippet.trim().is_empty() {
                continue;
            }
            let rel = relative_or_abs(&p, &cwd);
            body.push_str(&format!("\n\n### {rel}\n{snippet}"));
            used += consumed + rel.len() + 8;
        }

        Ok((format!("dir:{label}"), body, used.min(budget)))
    })
    .await
    .map_err(|e| e.to_string())?
}
