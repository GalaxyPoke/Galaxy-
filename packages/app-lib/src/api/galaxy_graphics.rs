//! Galaxy宝可梦: 画质档位 (graphics presets) on the home page.
//!
//! A preset rewrites the video keys of the instance's `options.txt` and the
//! shader switch in `config/iris.properties`; every other line is kept. The
//! modpack sync leaves both files to the player (`options.txt` is INITIAL, the
//! Iris / Sodium files are `PLAYER_PREFERENCE_FILES` in the sync engine), so
//! the next launch keeps the choice.
//!
//! When the current settings match no preset (the player tuned them in game),
//! they are copied to `<instance>\.galaxy-graphics\` first, and the `custom`
//! preset puts that copy back.

use crate::api::galaxy::{find_game_root, instance_dir_for};
use serde::Serialize;
use std::path::{Path, PathBuf};

const OPTIONS_FILE: &str = "options.txt";
const BACKUP_DIR: &str = ".galaxy-graphics";
/// Used by 高画质 when no shader pack is selected yet.
const DEFAULT_SHADER_PACK: &str = "ComplementaryReimagined_r5.3.zip";

struct Preset {
    id: &'static str,
    options: &'static [(&'static str, &'static str)],
    shaders: bool,
}

/// The main server sends 8 chunks (`view-distance=8`), so more render
/// distance than 10 only costs frames.
const PRESETS: &[Preset] = &[
    Preset {
        id: "low",
        options: &[
            ("renderDistance", "6"),
            ("simulationDistance", "5"),
            ("graphicsMode", "0"),
            ("particles", "2"),
            ("entityShadows", "false"),
            ("renderClouds", "\"false\""),
            ("biomeBlendRadius", "0"),
            ("mipmapLevels", "2"),
            ("ao", "false"),
            ("entityDistanceScaling", "0.75"),
        ],
        shaders: false,
    },
    Preset {
        id: "balanced",
        options: &[
            ("renderDistance", "8"),
            ("simulationDistance", "6"),
            ("graphicsMode", "1"),
            ("particles", "1"),
            ("entityShadows", "true"),
            ("renderClouds", "\"fast\""),
            ("biomeBlendRadius", "2"),
            ("mipmapLevels", "4"),
            ("ao", "true"),
            ("entityDistanceScaling", "1.0"),
        ],
        shaders: false,
    },
    Preset {
        id: "high",
        options: &[
            ("renderDistance", "10"),
            ("simulationDistance", "8"),
            ("graphicsMode", "1"),
            ("particles", "0"),
            ("entityShadows", "true"),
            ("renderClouds", "\"true\""),
            ("biomeBlendRadius", "3"),
            ("mipmapLevels", "4"),
            ("ao", "true"),
            ("entityDistanceScaling", "1.25"),
        ],
        shaders: true,
    },
];

/// Keys that decide which preset the current settings are.
const DETECT_KEYS: &[&str] = &["renderDistance", "graphicsMode", "particles"];

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalaxyGraphicsState {
    /// `low` / `balanced` / `high`; `None` for the player's own settings.
    pub preset: Option<String>,
    pub options_found: bool,
    pub render_distance: Option<u32>,
    pub shaders_enabled: bool,
    pub shader_pack: Option<String>,
    /// At least one shader pack is installed (高画质 turns shaders on).
    pub shader_pack_available: bool,
    /// Own settings saved before the last switch to a preset.
    pub has_backup: bool,
}

fn other(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::OtherError(message.into()).as_error()
}

fn instance_dir() -> crate::Result<PathBuf> {
    let root = find_game_root()
        .ok_or_else(|| other("没有找到游戏目录（.minecraft）"))?;
    Ok(instance_dir_for(&root))
}

fn iris_path(dir: &Path) -> PathBuf {
    dir.join("config").join("iris.properties")
}

fn read_text(path: &Path) -> crate::Result<Option<String>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(
            String::from_utf8_lossy(&bytes)
                .trim_start_matches('\u{feff}')
                .to_string(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(other(format!(
            "读取 {} 失败：{error}",
            path.display()
        ))),
    }
}

/// Writes next to the target, then renames over it.
fn write_atomic(path: &Path, text: &str) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| other(format!("创建 {} 失败：{error}", parent.display())))?;
    }
    let temp = path.with_extension("galaxy-tmp");
    std::fs::write(&temp, text.as_bytes())
        .and_then(|()| std::fs::rename(&temp, path))
        .map_err(|error| {
            let _ = std::fs::remove_file(&temp);
            other(format!("写入 {} 失败：{error}", path.display()))
        })
}

fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// Replaces `key<sep>value` lines in place and appends missing keys; every
/// other line is kept as it is.
fn set_values(text: &str, sep: char, values: &[(&str, String)]) -> String {
    let newline = newline_of(text);
    let mut done = vec![false; values.len()];
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            let key = line.split(sep).next().unwrap_or("").trim();
            match values.iter().position(|(wanted, _)| *wanted == key) {
                Some(index) if line.contains(sep) && !line.trim_start().starts_with('#') => {
                    done[index] = true;
                    format!("{key}{sep}{}", values[index].1)
                }
                _ => line.to_string(),
            }
        })
        .collect();
    for (index, (key, value)) in values.iter().enumerate() {
        if !done[index] {
            lines.push(format!("{key}{sep}{value}"));
        }
    }
    let mut out = lines.join(newline);
    out.push_str(newline);
    out
}

fn get_value<'a>(text: &'a str, sep: char, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        if line.trim_start().starts_with('#') {
            return None;
        }
        let (name, value) = line.split_once(sep)?;
        (name.trim() == key).then(|| value.trim())
    })
}

/// Java properties value escaping (what `Properties.store` produces).
fn escape_property(value: &str) -> String {
    let mut out = String::new();
    for (index, c) in value.chars().enumerate() {
        match c {
            '\\' | ':' | '=' | '#' | '!' => {
                out.push('\\');
                out.push(c);
            }
            ' ' if index == 0 => out.push_str("\\ "),
            _ => out.push(c),
        }
    }
    out
}

fn unescape_property(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn shader_packs(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir.join("shaderpacks")) else {
        return Vec::new();
    };
    let mut packs: Vec<String> = entries
        .flatten()
        .filter(|entry| {
            let path = entry.path();
            path.is_dir()
                || path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        })
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect();
    packs.sort();
    packs
}

fn detect(options: &str, shaders: bool) -> Option<&'static str> {
    PRESETS
        .iter()
        .find(|preset| {
            preset.shaders == shaders
                && DETECT_KEYS.iter().all(|key| {
                    let wanted = preset
                        .options
                        .iter()
                        .find(|(name, _)| name == key)
                        .map(|(_, value)| *value);
                    wanted.is_some() && get_value(options, ':', key) == wanted
                })
        })
        .map(|preset| preset.id)
}

fn state_of(dir: &Path) -> crate::Result<GalaxyGraphicsState> {
    let options = read_text(&dir.join(OPTIONS_FILE))?;
    let iris = read_text(&iris_path(dir))?.unwrap_or_default();
    let shaders_enabled = get_value(&iris, '=', "enableShaders") == Some("true");
    let shader_pack = get_value(&iris, '=', "shaderPack")
        .map(unescape_property)
        .filter(|pack| !pack.is_empty());
    Ok(GalaxyGraphicsState {
        preset: options
            .as_deref()
            .and_then(|text| detect(text, shaders_enabled))
            .map(str::to_string),
        options_found: options.is_some(),
        render_distance: options
            .as_deref()
            .and_then(|text| get_value(text, ':', "renderDistance"))
            .and_then(|value| value.parse().ok()),
        shaders_enabled,
        shader_pack,
        shader_pack_available: !shader_packs(dir).is_empty(),
        has_backup: dir.join(BACKUP_DIR).join(OPTIONS_FILE).is_file(),
    })
}

/// The current settings and which preset they match.
pub fn current() -> crate::Result<GalaxyGraphicsState> {
    state_of(&instance_dir()?)
}

fn backup(dir: &Path) -> crate::Result<()> {
    let target = dir.join(BACKUP_DIR);
    std::fs::create_dir_all(&target)
        .map_err(|error| other(format!("创建备份目录失败：{error}")))?;
    for (source, name) in [
        (dir.join(OPTIONS_FILE), OPTIONS_FILE),
        (iris_path(dir), "iris.properties"),
    ] {
        if source.is_file() {
            std::fs::copy(&source, target.join(name))
                .map_err(|error| other(format!("备份 {name} 失败：{error}")))?;
        }
    }
    Ok(())
}

fn restore(dir: &Path) -> crate::Result<()> {
    let source = dir.join(BACKUP_DIR);
    let options = read_text(&source.join(OPTIONS_FILE))?
        .ok_or_else(|| other("没有保存过之前的设置"))?;
    write_atomic(&dir.join(OPTIONS_FILE), &options)?;
    if let Some(iris) = read_text(&source.join("iris.properties"))? {
        write_atomic(&iris_path(dir), &iris)?;
    }
    Ok(())
}

fn apply_preset(dir: &Path, preset: &Preset) -> crate::Result<()> {
    let options_path = dir.join(OPTIONS_FILE);
    let options = read_text(&options_path)?
        .ok_or_else(|| other("还没有游戏设置文件，请先启动一次游戏"))?;
    let iris_file = iris_path(dir);
    let iris = read_text(&iris_file)?.unwrap_or_default();
    let shaders_now = get_value(&iris, '=', "enableShaders") == Some("true");
    if detect(&options, shaders_now).is_none() {
        backup(dir)?;
    }

    let values: Vec<(&str, String)> = preset
        .options
        .iter()
        .map(|(key, value)| (*key, (*value).to_string()))
        .collect();
    write_atomic(&options_path, &set_values(&options, ':', &values))?;

    let packs = shader_packs(dir);
    let selected = get_value(&iris, '=', "shaderPack")
        .map(unescape_property)
        .filter(|pack| packs.contains(pack));
    let mut iris_values = Vec::new();
    if preset.shaders {
        let pack = selected
            .or_else(|| {
                packs
                    .iter()
                    .find(|pack| pack.as_str() == DEFAULT_SHADER_PACK)
                    .cloned()
            })
            .or_else(|| packs.first().cloned());
        match pack {
            Some(pack) => {
                iris_values.push(("enableShaders", "true".to_string()));
                iris_values.push(("shaderPack", escape_property(&pack)));
            }
            None => iris_values.push(("enableShaders", "false".to_string())),
        }
    } else {
        iris_values.push(("enableShaders", "false".to_string()));
    }
    if !iris.is_empty() || preset.shaders {
        write_atomic(&iris_file, &set_values(&iris, '=', &iris_values))?;
    }
    Ok(())
}

/// Applies `low` / `balanced` / `high`, or `custom` to put the saved own
/// settings back. Refused while the game runs: it keeps its settings in
/// memory and writes them back.
#[tracing::instrument]
pub async fn apply(preset: &str) -> crate::Result<GalaxyGraphicsState> {
    if !crate::api::process::get_all().await?.is_empty() {
        return Err(other("请先关闭游戏再切换画质"));
    }
    let dir = instance_dir()?;
    if preset == "custom" {
        restore(&dir)?;
    } else {
        let spec = PRESETS
            .iter()
            .find(|spec| spec.id == preset)
            .ok_or_else(|| other("没有这个画质档位"))?;
        apply_preset(&dir, spec)?;
    }
    tracing::info!(preset, "Applied a graphics preset");
    state_of(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTIONS: &str = "version:3955\r\nrenderDistance:6\r\ngraphicsMode:0\r\nparticles:0\r\nlang:zh_cn\r\nresourcePacks:[\"vanilla\",\"file/IA.zip\"]\r\n";

    #[test]
    fn values_are_replaced_in_place_and_other_lines_kept() {
        let out = set_values(
            OPTIONS,
            ':',
            &[("renderDistance", "8".into()), ("ao", "true".into())],
        );
        assert!(out.contains("renderDistance:8\r\n"));
        assert!(out.contains("resourcePacks:[\"vanilla\",\"file/IA.zip\"]\r\n"));
        assert!(out.contains("lang:zh_cn\r\n"));
        assert!(out.ends_with("ao:true\r\n"));
        assert_eq!(out.matches("renderDistance").count(), 1);
    }

    #[test]
    fn presets_are_detected() {
        assert_eq!(detect(OPTIONS, false), None);
        let low: Vec<(&str, String)> = PRESETS[0]
            .options
            .iter()
            .map(|(k, v)| (*k, (*v).to_string()))
            .collect();
        let text = set_values(OPTIONS, ':', &low);
        assert_eq!(detect(&text, false), Some("low"));
        assert_eq!(detect(&text, true), None);
    }

    #[test]
    fn apply_backs_up_own_settings_and_restores_them() {
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join(OPTIONS_FILE), OPTIONS).unwrap();
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(
            iris_path(dir),
            "#comment\nenableShaders=false\nshaderPack=Missing.zip\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("shaderpacks")).unwrap();
        std::fs::write(dir.join("shaderpacks").join(DEFAULT_SHADER_PACK), "zip").unwrap();

        let high = PRESETS.iter().find(|p| p.id == "high").unwrap();
        apply_preset(dir, high).unwrap();
        let state = state_of(dir).unwrap();
        assert_eq!(state.preset.as_deref(), Some("high"));
        assert!(state.shaders_enabled);
        assert_eq!(state.shader_pack.as_deref(), Some(DEFAULT_SHADER_PACK));
        assert!(state.has_backup);
        let iris = std::fs::read_to_string(iris_path(dir)).unwrap();
        assert!(iris.starts_with("#comment\n"));

        // A preset → preset switch keeps the backup of the own settings.
        let low = PRESETS.iter().find(|p| p.id == "low").unwrap();
        apply_preset(dir, low).unwrap();
        assert!(!state_of(dir).unwrap().shaders_enabled);
        let saved = std::fs::read_to_string(dir.join(BACKUP_DIR).join(OPTIONS_FILE)).unwrap();
        assert_eq!(saved, OPTIONS);

        restore(dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join(OPTIONS_FILE)).unwrap(), OPTIONS);
        assert_eq!(state_of(dir).unwrap().preset, None);
    }

    #[test]
    fn property_values_round_trip() {
        let name = "My Pack: v1=2.zip";
        assert_eq!(unescape_property(&escape_property(name)), name);
        assert_eq!(escape_property(" a"), "\\ a");
    }
}
