use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const GRN: &str = "\x1b[1;32m";
const BLU: &str = "\x1b[1;34m";
const YEL: &str = "\x1b[33m";
const RED: &str = "\x1b[1;31m";

#[derive(Debug, Deserialize)]
struct ModelInfo {
    display_name: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EffortInfo {
    level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceInfo {
    current_dir: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContextWindowInfo {
    used_percentage: Option<f64>,
    total_input_tokens: Option<f64>,
    total_tokens: Option<f64>,
    context_window_size: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct LimitInfo {
    used_percentage: Option<f64>,
    resets_at: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RateLimitsInfo {
    five_hour: Option<LimitInfo>,
    seven_day: Option<LimitInfo>,
}

#[derive(Debug, Deserialize)]
struct InputData {
    model: Option<ModelInfo>,
    effort: Option<EffortInfo>,
    workspace: Option<WorkspaceInfo>,
    cwd: Option<String>,
    context_window: Option<ContextWindowInfo>,
    rate_limits: Option<RateLimitsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CachedRateLimits {
    five_hour_used: Option<i64>,
    five_hour_resets_at: Option<i64>,
    seven_day_used: Option<i64>,
    seven_day_resets_at: Option<i64>,
    updated_at: Option<i64>,
}

fn meter(used_pct: i64) -> &'static str {
    let left = 100 - used_pct;
    if left >= 50 {
        GRN
    } else if left >= 20 {
        YEL
    } else {
        RED
    }
}

fn fmt_tokens(val: f64) -> String {
    let num = val.round() as i64;
    if num >= 1_000_000 {
        format!("{}M", num / 1_000_000)
    } else if num >= 1_000 {
        format!("{}k", num / 1_000)
    } else {
        format!("{num}")
    }
}

fn countdown(target_epoch: i64) -> Option<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let diff = target_epoch - now;
    if diff <= 0 {
        return None;
    }
    if diff < 60 {
        return Some("<1m".to_string());
    }
    let d = diff / 86400;
    let h = (diff % 86400) / 3600;
    let m = (diff % 3600) / 60;
    if d > 0 {
        Some(format!("{d}d{h}h"))
    } else if h > 0 {
        Some(format!("{h}h{m}m"))
    } else {
        Some(format!("{m}m"))
    }
}

fn shorten_path(dir: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let p = if !home.is_empty() && dir.starts_with(&home) {
        format!("~{}", &dir[home.len()..])
    } else {
        dir.to_string()
    };

    if p == "~" || p.is_empty() {
        return p;
    }

    let components: Vec<&str> = p.split('/').filter(|c| !c.is_empty()).collect();
    if p.starts_with('~') {
        if components.len() > 3 {
            let last_two = &components[components.len() - 2..];
            format!("…/{}/{}", last_two[0], last_two[1])
        } else {
            p
        }
    } else if components.len() > 2 {
        let last_two = &components[components.len() - 2..];
        format!("…/{}/{}", last_two[0], last_two[1])
    } else {
        p
    }
}

fn get_git_branch(mut dir: &Path) -> Option<String> {
    loop {
        let git_entry = dir.join(".git");
        if git_entry.is_dir() {
            let head_file = git_entry.join("HEAD");
            return parse_git_head(&head_file);
        } else if git_entry.is_file() {
            if let Ok(content) = std::fs::read_to_string(&git_entry)
                && let Some(line) = content.lines().next()
                && let Some(rel_or_abs) = line.strip_prefix("gitdir:")
            {
                let gitdir = rel_or_abs.trim();
                let target: PathBuf = if Path::new(gitdir).is_absolute() {
                    gitdir.into()
                } else {
                    dir.join(gitdir)
                };
                return parse_git_head(&target.join("HEAD"));
            }
            return None;
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    None
}

fn parse_git_head(head_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(head_path).ok()?;
    let line = content.lines().next()?.trim();
    if let Some(branch) = line.strip_prefix("ref: refs/heads/") {
        let mut b = branch.to_string();
        if b.chars().count() > 20 {
            let truncated: String = b.chars().take(19).collect();
            b = format!("{truncated}…");
        }
        Some(b)
    } else if line.len() >= 7 {
        let short_sha: String = line.chars().take(7).collect();
        Some(short_sha)
    } else {
        None
    }
}

/// (used_percentage, resets_at)
type QuotaState = Option<(i64, Option<i64>)>;

fn sync_rate_limits(incoming: Option<RateLimitsInfo>) -> (QuotaState, QuotaState) {
    let home = std::env::var("HOME").unwrap_or_default();
    let cache_dir = Path::new(&home).join(".cache").join("claude");
    let cache_file = cache_dir.join("ratelimits.json");
    let lock_file_path = cache_dir.join("ratelimits.lock");

    let _ = std::fs::create_dir_all(&cache_dir);
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_file_path);

    if let Ok(ref lk) = lock {
        unsafe {
            libc::flock(lk.as_raw_fd(), libc::LOCK_EX);
        }
    }

    let mut cached: CachedRateLimits = std::fs::read_to_string(&cache_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut dirty = false;

    if let Some(ref inc) = incoming {
        if let Some(ref fh) = inc.five_hour
            && let Some(used) = fh.used_percentage
        {
            let used_i = used.round() as i64;
            let reset_i = fh.resets_at.map(|r| r.round() as i64);

            if reset_i != cached.five_hour_resets_at && reset_i.is_some() {
                cached.five_hour_used = Some(used_i);
                cached.five_hour_resets_at = reset_i;
                dirty = true;
            } else {
                let cur_used = cached.five_hour_used.unwrap_or(0);
                cached.five_hour_used = Some(used_i.max(cur_used));
                if reset_i.is_some() {
                    cached.five_hour_resets_at = reset_i;
                }
                dirty = true;
            }
        }

        if let Some(ref sd) = inc.seven_day
            && let Some(used) = sd.used_percentage
        {
            let used_i = used.round() as i64;
            let reset_i = sd.resets_at.map(|r| r.round() as i64);

            if reset_i != cached.seven_day_resets_at && reset_i.is_some() {
                cached.seven_day_used = Some(used_i);
                cached.seven_day_resets_at = reset_i;
                dirty = true;
            } else {
                let cur_used = cached.seven_day_used.unwrap_or(0);
                cached.seven_day_used = Some(used_i.max(cur_used));
                if reset_i.is_some() {
                    cached.seven_day_resets_at = reset_i;
                }
                dirty = true;
            }
        }
    }

    if dirty {
        cached.updated_at = Some(now);
        let tmp_file = cache_dir.join(format!("ratelimits.json.tmp.{}", std::process::id()));
        if let Ok(json_str) = serde_json::to_string(&cached)
            && std::fs::write(&tmp_file, json_str).is_ok()
        {
            let _ = std::fs::rename(&tmp_file, &cache_file);
        }
    }

    if let Ok(ref lk) = lock {
        unsafe {
            libc::flock(lk.as_raw_fd(), libc::LOCK_UN);
        }
    }

    let fh_out = cached
        .five_hour_used
        .map(|u| (u, cached.five_hour_resets_at));
    let sd_out = cached
        .seven_day_used
        .map(|u| (u, cached.seven_day_resets_at));
    (fh_out, sd_out)
}

fn main() {
    let mut raw = String::new();
    if io::stdin().read_to_string(&mut raw).is_err() || raw.trim().is_empty() {
        return;
    }

    let data: InputData = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(_) => return,
    };

    let model = data
        .model
        .as_ref()
        .and_then(|m| m.display_name.clone().or_else(|| m.id.clone()))
        .unwrap_or_default();

    let effort = data.effort.as_ref().and_then(|e| e.level.clone());

    let ws = data.workspace.as_ref();
    let raw_dir = ws
        .and_then(|w| w.current_dir.as_deref())
        .or_else(|| ws.and_then(|w| w.cwd.as_deref()))
        .or(data.cwd.as_deref())
        .unwrap_or("");

    let short_dir = shorten_path(raw_dir);

    let branch = if !raw_dir.is_empty() {
        get_git_branch(Path::new(raw_dir))
    } else {
        None
    };

    let cw = data.context_window.as_ref();
    let ctx_pct = cw.and_then(|c| c.used_percentage).map(|p| p.round() as i64);

    let ctx_size = if let Some(c) = cw {
        let cur = c.total_input_tokens.or(c.total_tokens);
        let max = c.context_window_size;
        match (cur, max) {
            (Some(cur_val), Some(max_val)) => {
                Some(format!("{}/{}", fmt_tokens(cur_val), fmt_tokens(max_val)))
            }
            (Some(cur_val), None) => Some(fmt_tokens(cur_val)),
            _ => None,
        }
    } else {
        None
    };

    let (five_hour, seven_day) = sync_rate_limits(data.rate_limits);

    // Row 1 Construction
    let mut segs = Vec::new();

    if !model.is_empty() {
        if let Some(eff) = effort {
            segs.push(format!("{GRN}{model}{RESET} {DIM}{eff}{RESET}"));
        } else {
            segs.push(format!("{GRN}{model}{RESET}"));
        }
    }

    if !short_dir.is_empty() {
        if let Some(br) = branch {
            segs.push(format!("{BLU}{short_dir}{RESET} {DIM}({br}){RESET}"));
        } else {
            segs.push(format!("{BLU}{short_dir}{RESET}"));
        }
    }

    let sep = format!(" {DIM}·{RESET} ");
    let line1 = segs.join(&sep);

    // Row 2 Construction
    let mut quota_segs = Vec::new();

    if let Some((used, reset_opt)) = five_hour {
        let remaining = (100 - used).max(0);
        let m = meter(used);
        let reset_str = reset_opt.and_then(countdown);
        if let Some(r) = reset_str {
            quota_segs.push(format!("{m}5h {remaining}%{RESET} {DIM}({r}){RESET}"));
        } else {
            quota_segs.push(format!("{m}5h {remaining}%{RESET}"));
        }
    }

    if let Some((used, reset_opt)) = seven_day {
        let remaining = (100 - used).max(0);
        let m = meter(used);
        let reset_str = reset_opt.and_then(countdown);
        if let Some(r) = reset_str {
            quota_segs.push(format!("{m}7d {remaining}%{RESET} {DIM}({r}){RESET}"));
        } else {
            quota_segs.push(format!("{m}7d {remaining}%{RESET}"));
        }
    }

    if let Some(pct) = ctx_pct {
        let m = meter(pct);
        if let Some(size) = ctx_size {
            quota_segs.push(format!("{m}ctx {pct}%{RESET} {DIM}({size}){RESET}"));
        } else {
            quota_segs.push(format!("{m}ctx {pct}%{RESET}"));
        }
    }

    let line2 = quota_segs.join(&sep);

    if !line2.is_empty() {
        print!("{line1}\n{line2}");
    } else {
        print!("{line1}");
    }
    let _ = io::stdout().flush();
}
