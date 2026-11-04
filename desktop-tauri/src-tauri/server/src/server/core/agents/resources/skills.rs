use std::{fs, path::{Component, Path, PathBuf}};
use serde_json::{json, Value};
use super::super::files;
use super::paths::ResourcePaths;

pub fn list(paths: &ResourcePaths) -> Result<Value, String> {
    let mut skills = Vec::new();
    for (root_index, root) in paths.skills.iter().enumerate() {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("读取 Skill 目录失败：{error}")),
        };
        for entry in entries {
            if skills.len() >= 1000 { return Err("Skill 数量超过 1000，请缩小目录范围".into()); }
            let entry = entry.map_err(|error| error.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            let linked = metadata.file_type().is_symlink();
            let doc = if path.is_dir() { path.join("SKILL.md") } else if path.extension().is_some_and(|ext| ext == "md") { path.clone() } else { continue; };
            if !doc.is_file() { continue; }
            let bytes = files::read_limited(&doc, 1024 * 1024)?;
            let content = files::text(bytes.as_deref())?;
            let description = frontmatter(&content, "description");
            let label = frontmatter(&content, "name").unwrap_or_else(|| name.trim_end_matches(".md").to_string());
            skills.push(json!({ "id": format!("{root_index}:{name}"), "name": label, "description": description, "path": doc,
                "linked": linked, "revision": super::revision(bytes.as_deref()) }));
        }
    }
    skills.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    Ok(json!({ "roots": paths.skills, "items": skills }))
}

pub fn doc(paths: &ResourcePaths, id: &str) -> Result<PathBuf, String> {
    let (root, name) = id.split_once(':').ok_or("Skill id 无效")?;
    let root = root.parse::<usize>().ok().and_then(|index| paths.skills.get(index)).ok_or("Skill 目录不存在")?;
    safe_name(name)?;
    let target = root.join(name);
    let doc = if target.is_dir() { target.join("SKILL.md") } else if name.ends_with(".md") { target } else { target.join("SKILL.md") };
    Ok(doc)
}

pub fn change(paths: &ResourcePaths, id: &str, content: &str, delete: bool) -> Result<files::Change, String> {
    let doc = doc(paths, id)?;
    super::reject_links(&doc)?;
    let mut change = files::Change::load(doc)?;
    if delete {
        if change.before.is_none() { return Err("Skill 不存在".into()); }
        change.after = None;
    } else {
        if content.len() > 1024 * 1024 { return Err("Skill 内容超过 1 MB".into()); }
        if frontmatter(content, "name").is_none() || frontmatter(content, "description").is_none() { return Err("SKILL.md 需要 name 与 description 的 YAML frontmatter".into()); }
        change.after = Some(content.as_bytes().to_vec());
    }
    Ok(change)
}

fn safe_name(name: &str) -> Result<(), String> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some()
        || name.starts_with('.') || name.contains(['\\', '/', ':']) || name.chars().any(char::is_control) {
        return Err("Skill 名称不能包含路径或隐藏目录".into());
    }
    Ok(())
}

fn frontmatter(content: &str, field: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" { return None; }
    for line in lines {
        if line.trim() == "---" { break; }
        if let Some(value) = line.strip_prefix(&format!("{field}:")) {
            let value = value.trim().trim_matches(['\'', '"']);
            if !value.is_empty() { return Some(value.to_string()); }
        }
    }
    None
}
