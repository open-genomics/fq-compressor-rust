// =============================================================================
// fqc - System memory detection
// =============================================================================

// =============================================================================
// System Memory Detection
// =============================================================================

/// Get available system memory in MB.
pub fn get_available_memory_mb() -> usize {
    #[cfg(target_os = "linux")]
    {
        get_available_memory_linux()
    }
    #[cfg(target_os = "macos")]
    {
        get_available_memory_macos()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        8192 // Default fallback
    }
}

#[cfg(target_os = "linux")]
fn get_available_memory_linux() -> usize {
    let host_mb = read_proc_mem_available_mb().unwrap_or(8192);
    match get_cgroup_available_memory_mb() {
        Some(cgroup_mb) => host_mb.min(cgroup_mb.max(1)),
        None => host_mb,
    }
}

#[cfg(target_os = "linux")]
fn read_proc_mem_available_mb() -> Option<usize> {
    let contents = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb = rest.split_whitespace().next()?.parse::<usize>().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_cgroup_available_memory_mb() -> Option<usize> {
    for (limit_path, usage_path) in [
        ("/sys/fs/cgroup/memory.max", "/sys/fs/cgroup/memory.current"),
        (
            "/sys/fs/cgroup/memory/memory.limit_in_bytes",
            "/sys/fs/cgroup/memory/memory.usage_in_bytes",
        ),
    ] {
        match std::fs::read_to_string(limit_path) {
            Ok(limit) => {
                let limit_bytes = parse_cgroup_memory_value(&limit)?;
                let usage_bytes = std::fs::read_to_string(usage_path)
                    .ok()
                    .and_then(|raw| parse_cgroup_memory_value(&raw))
                    .unwrap_or(0);
                return Some(cgroup_available_mb(limit_bytes, usage_bytes));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn parse_cgroup_memory_value(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "max" {
        return None;
    }
    raw.parse::<u64>().ok().filter(|&value| value > 0)
}

#[cfg(target_os = "linux")]
fn cgroup_available_mb(limit_bytes: u64, usage_bytes: u64) -> usize {
    let available_bytes = limit_bytes.saturating_sub(usage_bytes);
    usize::try_from(available_bytes / (1024 * 1024))
        .unwrap_or(usize::MAX)
        .max(1)
}

#[cfg(target_os = "macos")]
fn get_available_memory_macos() -> usize {
    // Use sysctl to get physical memory
    use std::process::Command;
    if let Ok(output) = Command::new("sysctl").arg("-n").arg("hw.memsize").output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return (bytes / (1024 * 1024)) as usize;
            }
        }
    }
    8192
}

#[cfg(all(test, target_os = "linux"))]
mod linux_memory_tests {
    use super::{cgroup_available_mb, parse_cgroup_memory_value};

    #[test]
    fn parses_cgroup_limit_and_usage_values() {
        assert_eq!(parse_cgroup_memory_value(" 214748364800\n"), Some(214_748_364_800));
        assert_eq!(parse_cgroup_memory_value("max\n"), None);
        assert_eq!(parse_cgroup_memory_value(""), None);
        assert_eq!(parse_cgroup_memory_value("not-a-number"), None);
    }

    #[test]
    fn cgroup_available_subtracts_usage_and_saturates() {
        let gib = 1024_u64 * 1024 * 1024;
        assert_eq!(cgroup_available_mb(200 * gib, 10 * gib), 190 * 1024);
        assert_eq!(cgroup_available_mb(1, 2), 1);
    }
}
