use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn save_assistant_image(app: AppHandle, id: String) -> Result<Value, String> {
    let (bytes, mime) = agent2api_server::server::core::assistant_images::read(&id, None)
        .ok_or("助手图片不存在或已随会话删除")?;
    let extension = match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => return Err("不支持的助手图片类型".into()),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let target = app.dialog().file().set_title("保存图片")
            .set_file_name(format!("image-{}.{extension}", &id[..8]))
            .add_filter("图片", &[extension]).blocking_save_file();
        let Some(target) = target else { return Ok(json!({ "canceled": true })); };
        let path = target.into_path().map_err(|error| format!("保存路径无效：{error}"))?;
        std::fs::write(&path, bytes).map_err(|error| format!("保存图片失败：{error}"))?;
        Ok(json!({ "saved": true, "path": path.display().to_string() }))
    }).await.map_err(|error| format!("保存图片任务失败：{error}"))?
}
