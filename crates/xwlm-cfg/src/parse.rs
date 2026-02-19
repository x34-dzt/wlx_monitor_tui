use std::fs;
use std::path::Path;

use crate::Compositor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedPosition {
    pub x: i32,
    pub y: i32,
}

pub fn get_saved_monitor_position(
    compositor: Compositor,
    config_path: &str,
    monitor_name: &str,
) -> Option<SavedPosition> {
    if !Path::new(config_path).exists() {
        return None;
    }

    let content = fs::read_to_string(config_path).ok()?;

    match compositor {
        Compositor::Hyprland => parse_hyprland_position(&content, monitor_name),
        Compositor::Sway => parse_sway_position(&content, monitor_name),
        _ => None,
    }
}

fn parse_hyprland_position(
    content: &str,
    monitor_name: &str,
) -> Option<SavedPosition> {
    let mut found_position: Option<SavedPosition> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and non-monitor lines
        if line.starts_with('#') || !line.starts_with("monitor") {
            continue;
        }

        // Strip "monitor" prefix and any surrounding whitespace/equals signs
        let line = line["monitor".len()..]
            .trim_start_matches([' ', '='])
            .trim();

        // Split into comma-separated parts
        let parts: Vec<&str> = line.split(',').map(|p| p.trim()).collect();

        // Must start with the target monitor name
        if parts.first().copied() != Some(monitor_name) {
            continue;
        }

        // Skip disabled entries, but don't stop — a later entry might re-enable it
        if parts.contains(&"disable") {
            continue;
        }

        // Position is the third field (index 2), formatted as "XxY"
        if let Some(pos_str) = parts.get(2)
            && let Some((x, y)) = parse_xy_position(pos_str)
        {
            found_position = Some(SavedPosition { x, y });
        }
    }

    found_position
}

fn parse_sway_position(
    content: &str,
    monitor_name: &str,
) -> Option<SavedPosition> {
    let mut current_output: Option<String> = None;
    let mut in_output_block = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("output") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                current_output = Some(parts[1].to_string());
                in_output_block = line.contains('{');
            }
        }

        if in_output_block && line.contains('}') {
            in_output_block = false;
            current_output = None;
        }

        // Only parse position lines for the target output
        if current_output.as_deref() != Some(monitor_name) {
            continue;
        }

        if line.starts_with("pos ") || line.contains(" pos ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for i in 0..parts.len() {
                if parts[i] == "pos"
                    && i + 2 < parts.len()
                    && let (Ok(x), Ok(y)) = (
                        parts[i + 1].parse::<i32>(),
                        parts[i + 2].parse::<i32>(),
                    )
                {
                    return Some(SavedPosition { x, y });
                }
            }
        }
    }

    None
}

/// Parses a position string in "XxY" format (e.g. "1920x0", "-1920x0").
fn parse_xy_position(s: &str) -> Option<(i32, i32)> {
    let (x_str, y_str) = s.split_once('x')?;
    let x = x_str.trim().parse::<i32>().ok()?;
    let y = y_str.trim().parse::<i32>().ok()?;
    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hyprland_position_basic() {
        let content = r#"
monitor = HDMI-A-1, 2560x1440@59, 1920x0, 1
monitor = eDP-1, 1920x1080@144, 0x0, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "HDMI-A-1"),
            Some(SavedPosition { x: 1920, y: 0 })
        );
    }

    #[test]
    fn test_parse_hyprland_position_with_disable() {
        let content = r#"
monitor = HDMI-A-1, 2560x1440@59, 1920x0, 1
monitor = eDP-1, 1920x1080@144, 0x0, 1
monitor = eDP-1, disable
"#;
        // Last non-disabled entry wins
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
    }

    #[test]
    fn test_parse_hyprland_position_only_disable() {
        let content = r#"
monitor = HDMI-A-1, 2560x1440@59, 1920x0, 1
monitor = eDP-1, disable
"#;
        assert_eq!(parse_hyprland_position(content, "eDP-1"), None);
    }

    #[test]
    fn test_parse_hyprland_position_not_found() {
        let content = r#"
monitor = HDMI-A-1, 2560x1440@59, 1920x0, 1
"#;
        assert_eq!(parse_hyprland_position(content, "eDP-1"), None);
    }

    #[test]
    fn test_parse_xy_position() {
        assert_eq!(parse_xy_position("0x0"), Some((0, 0)));
        assert_eq!(parse_xy_position("1920x0"), Some((1920, 0)));
        assert_eq!(parse_xy_position("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_xy_position("invalid"), None);
        assert_eq!(parse_xy_position("0"), None);
    }

    #[test]
    fn test_real_world_config() {
        let content = r#"
monitor = HDMI-A-1, 2560x1440@59, 1920x0, 1
monitor = eDP-1, 1920x1080@144, 0x0, 1
monitor = eDP-1, disable

workspace = 1, monitor:HDMI-A-1
workspace = 2, monitor:eDP-1
"#;
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "HDMI-A-1"),
            Some(SavedPosition { x: 1920, y: 0 })
        );
    }

    #[test]
    fn test_weird_config() {
        let content = r#"
# This is a comment
monitor=,preferred,auto,1
monitor = DP-1, 1920x1080@60, 0x0, 1
monitor = DP-2, 1920x1080@60, 1920x0, 1
monitor = HDMI-A-1, disable
monitor = HDMI-A-2, 3840x2160@60, 3840x0, 1.5, transform, 1
monitor = eDP-1, 1920x1080@144, 5760x0, 1

# Another comment
monitor=fake,1920x1080,0x0,1
"#;
        assert_eq!(
            parse_hyprland_position(content, "DP-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "DP-2"),
            Some(SavedPosition { x: 1920, y: 0 })
        );
        assert_eq!(parse_hyprland_position(content, "HDMI-A-1"), None);
        assert_eq!(
            parse_hyprland_position(content, "HDMI-A-2"),
            Some(SavedPosition { x: 3840, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 5760, y: 0 })
        );
    }

    #[test]
    fn test_disable_before_enable() {
        let content = r#"
monitor = eDP-1, disable
monitor = eDP-1, 1920x1080@144, 0x0, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
    }

    #[test]
    fn test_multiple_disables_and_enables() {
        let content = r#"
monitor = eDP-1, 1920x1080@144, 100x200, 1
monitor = eDP-1, disable
monitor = eDP-1, 1920x1080@144, 300x400, 1
monitor = eDP-1, disable
monitor = eDP-1, 1920x1080@144, 500x600, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 500, y: 600 })
        );
    }

    #[test]
    fn test_negative_positions() {
        let content = r#"
monitor = DP-1, 1920x1080@60, -1920x0, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "DP-1"),
            Some(SavedPosition { x: -1920, y: 0 })
        );
    }

    #[test]
    fn test_large_positions() {
        let content = r#"
monitor = DP-1, 1920x1080@60, 10000x5000, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "DP-1"),
            Some(SavedPosition { x: 10000, y: 5000 })
        );
    }

    #[test]
    fn test_empty_config() {
        assert_eq!(parse_hyprland_position("", "eDP-1"), None);
    }

    #[test]
    fn test_commented_monitor_lines() {
        let content = r#"
# monitor = eDP-1, 1920x1080@144, 0x0, 1
#monitor = DP-1, 1920x1080@60, 1920x0, 1
monitor = eDP-1, 1920x1080@144, 100x100, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 100, y: 100 })
        );
        assert_eq!(parse_hyprland_position(content, "DP-1"), None);
    }

    #[test]
    fn test_monitor_with_special_chars() {
        let content = r#"
monitor = HDMI-A-1, 1920x1080@60, 0x0, 1
monitor = HDMI-A-2, 1920x1080@60, 1920x0, 1
"#;
        assert_eq!(
            parse_hyprland_position(content, "HDMI-A-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "HDMI-A-2"),
            Some(SavedPosition { x: 1920, y: 0 })
        );
    }

    #[test]
    fn test_whitespace_variations() {
        let content = r#"
monitor=eDP-1,1920x1080@144,0x0,1
monitor = DP-1 , 1920x1080@60 , 1920x0 , 1
monitor  =  HDMI-A-1  ,  1920x1080@60  ,  3840x0  ,  1
"#;
        assert_eq!(
            parse_hyprland_position(content, "eDP-1"),
            Some(SavedPosition { x: 0, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "DP-1"),
            Some(SavedPosition { x: 1920, y: 0 })
        );
        assert_eq!(
            parse_hyprland_position(content, "HDMI-A-1"),
            Some(SavedPosition { x: 3840, y: 0 })
        );
    }
}
