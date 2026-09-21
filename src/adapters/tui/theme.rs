//! Glyph set and a small palette. Detection is adapted from herdr-sidebar
//! 0.13.0 (MIT): it asks whether a Nerd Font is installed, not whether the
//! terminal is using one.

#[cfg(any(not(target_os = "macos"), test))]
use std::time::Duration;

use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconMode {
    Ascii,
    Nerd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconSet {
    pub play: &'static str,
    pub search: &'static str,
    pub package: &'static str,
}

pub const ASCII_ICONS: IconSet = IconSet {
    play: ">",
    search: "/",
    package: "#",
};

/// Nerd Font codicons. Mono builds are typically one cell; `pad_icon` still
/// clamps them to the gutter so a two-cell glyph cannot shift hit-testing.
pub const NERD_ICONS: IconSet = IconSet {
    play: "\u{eb2c}",
    search: "\u{ea6d}",
    package: "\u{eb29}",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub accent: Color,
    pub text: Color,
    pub muted: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub footer: Color,
    pub keycap_bg: Color,
    pub keycap_fg: Color,
}

/// Useful subset of herdr-sidebar's VS Code palette, with an explicit
/// selection foreground so DarkGray + Reset cannot wash out on light terminals.
pub const PALETTE: Palette = Palette {
    accent: Color::Rgb(0x00, 0x78, 0xd4),
    text: Color::Reset,
    muted: Color::Rgb(0x6b, 0x6b, 0x6b),
    selection_bg: Color::DarkGray,
    selection_fg: Color::White,
    footer: Color::Rgb(0x6b, 0x6b, 0x6b),
    keycap_bg: Color::Rgb(0x32, 0x36, 0x3d),
    keycap_fg: Color::Rgb(0xc9, 0xce, 0xd6),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub mode: IconMode,
    pub icons: IconSet,
    pub palette: Palette,
}

impl Theme {
    pub fn ascii() -> Self {
        Self {
            mode: IconMode::Ascii,
            icons: ASCII_ICONS,
            palette: PALETTE,
        }
    }

    pub fn nerd() -> Self {
        Self {
            mode: IconMode::Nerd,
            icons: NERD_ICONS,
            palette: PALETTE,
        }
    }

    pub fn resolve(env: Option<&str>, nerd_installed: bool) -> Self {
        match resolve_icon_mode(env, nerd_installed) {
            IconMode::Ascii => Self::ascii(),
            IconMode::Nerd => Self::nerd(),
        }
    }

    pub fn magnifier_cols(self) -> u16 {
        self.icons.search.width().clamp(1, 2) as u16
    }
}

pub fn resolve_icon_mode(env: Option<&str>, nerd_installed: bool) -> IconMode {
    match env
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("ascii") => IconMode::Ascii,
        Some("nerd") => IconMode::Nerd,
        _ if nerd_installed => IconMode::Nerd,
        _ => IconMode::Ascii,
    }
}

pub fn pad_icon(glyph: &str, cols: usize) -> String {
    let width = glyph.width();
    if cols == 0 {
        return String::new();
    }
    if width > cols {
        return pad_icon(ASCII_ICONS.play, cols);
    }
    if width == cols {
        return glyph.to_string();
    }
    format!("{glyph}{}", " ".repeat(cols - width))
}

pub const FOOTER_HELP: &str = "[j][k] [h][l] [enter] [/]";

/// Nerd Fonts register under several spellings. Adapted from herdr-sidebar 0.13.0.
pub fn output_mentions_nerd_font(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("nerd font")
        || t.contains("nerdfont")
        || t.contains(" nf ")
        || t.contains("nf-")
        || t.contains("nf_")
}

pub fn font_dirs_mention_nerd_font(dirs: impl IntoIterator<Item = std::path::PathBuf>) -> bool {
    dirs.into_iter().any(|dir| {
        std::fs::read_dir(dir).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| output_mentions_nerd_font(&entry.file_name().to_string_lossy()))
        })
    })
}

pub fn nerd_font_installed() -> bool {
    use std::sync::OnceLock;
    static PROBE: OnceLock<bool> = OnceLock::new();
    *PROBE.get_or_init(probe_nerd_font)
}

pub fn probe_nerd_font() -> bool {
    #[cfg(windows)]
    {
        let keys = [
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
            r"HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
        ];
        keys.iter().any(|key| {
            let mut cmd = std::process::Command::new("reg");
            cmd.args(["query", key]);
            bounded_output(&mut cmd)
                .is_some_and(|out| output_mentions_nerd_font(&String::from_utf8_lossy(&out)))
        })
    }
    #[cfg(target_os = "macos")]
    {
        macos_font_dirs_mention_nerd_font()
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let mut cmd = std::process::Command::new("fc-list");
        bounded_output(&mut cmd)
            .is_some_and(|out| output_mentions_nerd_font(&String::from_utf8_lossy(&out)))
    }
}

#[cfg(any(not(target_os = "macos"), test))]
fn bounded_output(cmd: &mut std::process::Command) -> Option<Vec<u8>> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let started = std::time::Instant::now();
    let timeout = Duration::from_millis(400);
    let mut stdout = child.stdout.take()?;
    let (sender, receiver) = std::sync::mpsc::channel();
    // Drain while the command is running: fc-list can exceed the pipe capacity.
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let result = std::io::Read::read_to_end(&mut stdout, &mut buf).map(|_| buf);
        let _ = sender.send(result);
    });
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                return receiver
                    .recv_timeout(timeout.saturating_sub(started.elapsed()))
                    .ok()?
                    .ok();
            }
            Ok(Some(_)) => return None,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_font_dirs_mention_nerd_font() -> bool {
    let mut dirs = vec![
        std::path::PathBuf::from("/Library/Fonts"),
        std::path::PathBuf::from("/System/Library/Fonts"),
        std::path::PathBuf::from("/Network/Library/Fonts"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(std::path::PathBuf::from(home).join("Library/Fonts"));
    }
    font_dirs_mention_nerd_font(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn font_probe_drains_output_larger_than_a_pipe_buffer() {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "head -c 1048576 /dev/zero; printf 'Nerd Font'"]);
        let output = bounded_output(&mut command).expect("large font output should not time out");
        assert!(output.ends_with(b"Nerd Font"));
    }

    #[cfg(unix)]
    #[test]
    fn font_probe_still_times_out_and_reaps_the_child() {
        let mut command = std::process::Command::new("sleep");
        command.arg("5");
        let start = std::time::Instant::now();
        assert!(bounded_output(&mut command).is_none());
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn env_ascii_and_nerd_win_over_the_heuristic() {
        assert_eq!(resolve_icon_mode(Some("ascii"), true), IconMode::Ascii);
        assert_eq!(resolve_icon_mode(Some("nerd"), false), IconMode::Nerd);
        assert_eq!(resolve_icon_mode(Some("nope"), true), IconMode::Nerd);
        assert_eq!(resolve_icon_mode(None, false), IconMode::Ascii);
    }

    #[test]
    fn pad_icon_keeps_the_gutter_width() {
        assert_eq!(pad_icon(">", 2), "> ");
        assert_eq!(pad_icon("#", 2).width(), 2);
        assert_eq!(pad_icon("too-wide-glyph", 2), "> ");
    }

    #[test]
    fn nerd_font_spellings_match_without_scanning_the_host() {
        assert!(output_mentions_nerd_font(
            "JetBrainsMono Nerd Font: style=Regular"
        ));
        assert!(output_mentions_nerd_font("FiraCodeNerdFont-Regular.ttf"));
        assert!(output_mentions_nerd_font(
            "CaskaydiaCove NF Mono (TrueType)"
        ));
        assert!(!output_mentions_nerd_font("Menlo Regular"));
    }

    #[test]
    fn font_dir_probe_uses_the_injected_directory() {
        let root = std::env::temp_dir().join(format!("herdr-npm-font-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        std::fs::write(root.join("FiraCodeNerdFont-Regular.ttf"), []).unwrap();
        assert!(font_dirs_mention_nerd_font([root.clone()]));
        let _ = std::fs::remove_dir_all(&root);
    }
}
