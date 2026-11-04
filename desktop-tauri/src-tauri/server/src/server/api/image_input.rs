//! 图片编辑输入归一；上传只进内存，外链复用无凭证、DNS 固定的安全下载器。

use std::io::Cursor;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::extract::{FromRequest, Multipart, Request};
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::server::errors::GatewayError;
use crate::server::ServerState;

pub(super) const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_INLINE_BYTES: usize = 14 * 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_IMAGES: usize = 16;

fn invalid(message: impl Into<String>) -> GatewayError {
    GatewayError::bad_request(message).with_code("invalid_image_input")
}

fn image_field(name: &str) -> bool {
    matches!(name, "image" | "image[]" | "images" | "images[]")
        || name
            .strip_prefix("image[")
            .and_then(|name| name.strip_suffix(']'))
            .is_some_and(|index| {
                !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
            })
}

pub(super) async fn parse(request: Request, state: &ServerState) -> Result<Value, GatewayError> {
    let (parts, body) = request.into_parts();
    let content_type = parts
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let bytes = to_bytes(body, MAX_REQUEST_BYTES)
        .await
        .map_err(|_| GatewayError::with_status(413, "图片编辑请求总大小不得超过 16 MiB"))?;
    if content_type.starts_with("application/json") {
        return serde_json::from_slice(&bytes).map_err(|_| invalid("图片编辑请求必须是有效 JSON"));
    }
    if !content_type.starts_with("multipart/form-data") {
        return Err(invalid(
            "图片编辑请使用 multipart/form-data，或 JSON images[].image_url",
        ));
    }
    let request = Request::from_parts(parts, Body::from(bytes));
    let mut multipart = Multipart::from_request(request, state)
        .await
        .map_err(|_| invalid("无法解析 multipart/form-data，请检查 boundary"))?;
    let mut payload = Map::new();
    let mut images = Vec::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| invalid("图片上传表单不完整"))?
    {
        let name = field.name().unwrap_or("").to_string();
        if matches!(name.as_str(), "mask" | "input_fidelity") {
            return Err(invalid(format!(
                "当前参考图编辑通道不支持 {name}，请在 prompt 中描述修改范围"
            )));
        }
        if image_field(&name) {
            if images.len() >= MAX_IMAGES {
                return Err(invalid("图片编辑最多支持 16 张参考图"));
            }
            let mut data = Vec::new();
            while let Some(chunk) = field.chunk().await.map_err(|_| invalid("参考图上传中断"))?
            {
                if data.len().saturating_add(chunk.len()) > MAX_IMAGE_BYTES {
                    return Err(GatewayError::with_status(413, "单张参考图不得超过 10 MiB"));
                }
                data.extend_from_slice(&chunk);
            }
            let mime = mime_of(&data)?;
            images.push(json!({ "image_url": format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(data)) }));
        } else {
            if !super::images::parameter_supported(&name) {
                return Err(invalid(format!("图片编辑不支持参数 {name}")));
            }
            if payload.contains_key(&name) {
                return Err(invalid(format!("参数 {name} 不能重复提交")));
            }
            let text = field
                .text()
                .await
                .map_err(|_| invalid(format!("参数 {name} 必须是 UTF-8 文本")))?;
            let value = if matches!(
                name.as_str(),
                "n" | "output_compression" | "partial_images" | "stream"
            ) {
                serde_json::from_str(&text).map_err(|_| invalid(format!("参数 {name} 格式无效")))?
            } else {
                Value::String(text)
            };
            payload.insert(name, value);
        }
    }
    payload.insert("images".to_string(), Value::Array(images));
    Ok(Value::Object(payload))
}

pub(super) async fn prepare(payload: &mut Value) -> Result<(), GatewayError> {
    let timeout = Duration::from_millis(crate::server::config::timeout_settings().body_ms());
    tokio::time::timeout(timeout, prepare_inner(payload))
        .await
        .map_err(|_| invalid("参考图读取超时，尚未发起图片编辑请求"))?
}

async fn prepare_inner(payload: &mut Value) -> Result<(), GatewayError> {
    let images = payload
        .get("images")
        .and_then(Value::as_array)
        .filter(|images| !images.is_empty() && images.len() <= MAX_IMAGES)
        .ok_or_else(|| invalid("images 必须包含 1 到 16 张参考图"))?;
    let mut prepared = Vec::with_capacity(images.len());
    let mut total = 0usize;
    for image in images {
        let object = image
            .as_object()
            .ok_or_else(|| invalid("images 每项必须是 {image_url: ...} 对象"))?;
        if object.keys().any(|key| key != "image_url") {
            return Err(invalid("参考图仅支持 image_url，不支持 file_id 或其它字段"));
        }
        let link = object
            .get("image_url")
            .and_then(Value::as_str)
            .filter(|link| !link.trim().is_empty())
            .ok_or_else(|| invalid("参考图缺少 image_url"))?;
        let bytes = if let Some(rest) = link.strip_prefix("data:") {
            let (meta, encoded) = rest
                .split_once(',')
                .ok_or_else(|| invalid("参考图 data URL 无效"))?;
            if !meta.starts_with("image/") || !meta.ends_with(";base64") {
                return Err(invalid("参考图必须使用 data:image/...;base64 格式"));
            }
            if encoded.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
                return Err(GatewayError::with_status(413, "单张参考图不得超过 10 MiB"));
            }
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| invalid("参考图 base64 无效"))?
        } else {
            let url = url::Url::parse(link).map_err(|_| invalid("参考图 URL 无效"))?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(invalid("参考图只接受无凭证的 HTTP(S) URL"));
            }
            let cached = url
                .path()
                .strip_prefix("/v1/images/files/")
                .and_then(super::image_assets::read);
            if let Some((bytes, _)) = cached {
                bytes.to_vec()
            } else {
                super::image_output::download(link, None, false)
                    .await
                    .map_err(|error| invalid(format!("参考图下载失败：{}", error.message)))?
            }
        };
        let url = validate_bytes(bytes).await?;
        total = total.saturating_add(url.len());
        if total > MAX_INLINE_BYTES {
            return Err(GatewayError::with_status(
                413,
                "参考图内联数据总大小不得超过 14 MiB",
            ));
        }
        prepared.push(json!({ "image_url": url }));
    }
    payload["images"] = Value::Array(prepared);
    let size = serde_json::to_vec(payload)
        .map_err(|_| invalid("图片编辑请求无法序列化"))?
        .len();
    if size > MAX_INLINE_BYTES {
        return Err(GatewayError::with_status(
            413,
            "图片编辑内联请求总大小不得超过 14 MiB",
        ));
    }
    Ok(())
}

async fn validate_bytes(bytes: Vec<u8>) -> Result<String, GatewayError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(GatewayError::with_status(413, "单张参考图不得超过 10 MiB"));
    }
    tokio::task::spawn_blocking(move || {
        inspect_bytes(&bytes)?;
        let mime = mime_of(&bytes)?;
        Ok(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    })
    .await
    .map_err(|_| invalid("参考图解码任务失败"))?
}

pub(super) fn mime_of(bytes: &[u8]) -> Result<&'static str, GatewayError> {
    match image::guess_format(bytes) {
        Ok(image::ImageFormat::Png) => Ok("image/png"),
        Ok(image::ImageFormat::Jpeg) => Ok("image/jpeg"),
        Ok(image::ImageFormat::WebP) => Ok("image/webp"),
        _ => Err(invalid("参考图仅支持 PNG、JPEG、WebP")),
    }
}

pub(super) fn inspect_bytes(bytes: &[u8]) -> Result<(u32, u32), GatewayError> {
    if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
        return Err(invalid("图片为空或超过 32 MiB"));
    }
    mime_of(bytes)?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| invalid("图片格式无法识别"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| invalid("图片解码失败或边长超过 8192"))?;
    Ok((decoded.width(), decoded.height()))
}
