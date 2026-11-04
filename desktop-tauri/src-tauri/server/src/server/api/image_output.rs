//! 生图响应格式归一；链接下载不带提供商凭证，编码使用有界解码器。

use std::io::Cursor;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::http::HeaderMap;
use base64::Engine;
use futures::StreamExt;
use serde_json::{json, Value};

use crate::server::core::providers::adapter::adapter_by_id;
use crate::server::core::upstream::usage::RequestTelemetry;
use crate::server::errors::GatewayError;
use crate::server::ServerState;

use super::image_assets;

const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;

fn failure(message: impl Into<String>) -> GatewayError {
    GatewayError::with_status(502, message).with_code("image_generation_error")
}

fn format_of(bytes: &[u8]) -> Result<image::ImageFormat, GatewayError> {
    let format = image::guess_format(bytes).map_err(|_| failure("上游返回的内容不是有效图片"))?;
    if matches!(
        format,
        image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP
    ) {
        Ok(format)
    } else {
        Err(failure(
            "上游返回了不支持的图片编码（仅支持 png、jpeg、webp）",
        ))
    }
}

fn mime_of(format: image::ImageFormat) -> &'static str {
    match format {
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        _ => "image/png",
    }
}

fn encode(
    bytes: Vec<u8>,
    requested: Option<image::ImageFormat>,
    compression: Option<u8>,
) -> Result<(Vec<u8>, &'static str), GatewayError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(failure("生成图片超过 32 MiB 上限"));
    }
    let actual = format_of(&bytes)?;
    let format = requested.unwrap_or(actual);
    if compression.is_some() && format != image::ImageFormat::Jpeg {
        return Err(GatewayError::bad_request(
            "output_compression 目前仅支持 output_format=jpeg",
        ));
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), actual);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| failure("生成图片解码失败或超过尺寸上限"))?;
    if actual == format && compression.is_none() {
        return Ok((bytes, mime_of(format)));
    }
    let mut out = Cursor::new(Vec::new());
    if format == image::ImageFormat::Jpeg {
        image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut out,
            compression.unwrap_or(90).max(1),
        )
        .encode_image(&decoded.to_rgb8())
        .map_err(|_| failure("生成图片 JPEG 编码失败"))?;
    } else {
        decoded
            .write_to(&mut out, format)
            .map_err(|_| failure("生成图片编码失败"))?;
    }
    let bytes = out.into_inner();
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(failure("生成图片超过 32 MiB 上限"));
    }
    Ok((bytes, mime_of(format)))
}

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, _, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 198 && matches!(b, 18 | 19))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return public_address(IpAddr::V4(v4));
            }
            let segments = ip.segments();
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && segments[0] & 0xfe00 != 0xfc00
                && segments[0] & 0xffc0 != 0xfe80
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
                && segments[0] & 0xe000 == 0x2000
        }
    }
}

pub(super) async fn download(
    link: &str,
    proxy: Option<&crate::server::core::proxies::ResolvedProxy>,
    system_proxy: bool,
) -> Result<Vec<u8>, GatewayError> {
    let mut url = url::Url::parse(link).map_err(|_| failure("上游返回了无效图片链接"))?;
    for _ in 0..=4 {
        if !matches!(url.scheme(), "https" | "http")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(failure("上游图片链接的协议或凭证不符合安全要求"));
        }
        let host = url
            .host_str()
            .ok_or_else(|| failure("上游图片链接缺少主机"))?;
        let port = url.port_or_known_default().unwrap_or(443);
        let mut addresses: Vec<SocketAddr> =
            if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
                vec![SocketAddr::new(ip, port)]
            } else {
                tokio::net::lookup_host((host, port))
                    .await
                    .map_err(|_| failure("无法解析上游图片下载主机"))?
                    .collect()
            };
        addresses.sort();
        addresses.dedup();
        if addresses.is_empty()
            || addresses
                .iter()
                .any(|address| !public_address(address.ip()))
        {
            return Err(failure("拒绝从本机或内网地址下载上游图片"));
        }
        let client = crate::server::core::egress::image_download_client(
            proxy,
            system_proxy,
            host,
            &addresses,
        )
        .map_err(failure)?;
        let response = client
            .get(url.clone())
            .send()
            .await
            .map_err(|_| failure("下载上游生成图片失败（未携带账号凭证）"))?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| failure("图片下载跳转缺少 Location"))?;
            url = url
                .join(location)
                .map_err(|_| failure("图片下载跳转地址无效"))?;
            continue;
        }
        if !response.status().is_success() {
            return Err(failure(format!(
                "下载生成图片失败（HTTP {}）",
                response.status().as_u16()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
        {
            return Err(failure("生成图片超过 32 MiB 下载上限"));
        }
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| failure("生成图片下载中断"))?;
            if bytes.len().saturating_add(chunk.len()) > MAX_IMAGE_BYTES {
                return Err(failure("生成图片超过 32 MiB 下载上限"));
            }
            bytes.extend_from_slice(&chunk);
        }
        return Ok(bytes);
    }
    Err(failure("生成图片下载跳转次数超限"))
}

fn base_url(headers: &HeaderMap, port: u16) -> Result<url::Url, GatewayError> {
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| format!("127.0.0.1:{port}"));
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| *value == "https")
        .unwrap_or("http");
    let url = url::Url::parse(&format!("{scheme}://{host}"))
        .map_err(|_| GatewayError::bad_request("无法从 Host 构造图片下载地址"))?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(GatewayError::bad_request("Host 不是有效的网关地址"));
    }
    Ok(url)
}

pub(super) async fn normalize(
    state: &ServerState,
    payload: &Value,
    response: Value,
    headers: &HeaderMap,
    telemetry: &Arc<RequestTelemetry>,
) -> Result<Value, GatewayError> {
    let budget = Duration::from_millis(crate::server::config::timeout_settings().body_ms());
    let normalize = normalize_inner(state, payload, response, headers, telemetry);
    let timed = async {
        tokio::time::timeout(budget, normalize)
            .await
            .map_err(|_| failure("生成图片下载或编码超时，上游已生成，不会重新请求生图"))?
    };
    match telemetry.cancel_token() {
        Some(token) => tokio::select! {
            result = timed => result,
            _ = token.cancelled() => Err(crate::server::core::upstream::cancellation::cancelled_error()),
        },
        None => timed.await,
    }
}

async fn normalize_inner(
    state: &ServerState,
    payload: &Value,
    response: Value,
    headers: &HeaderMap,
    telemetry: &Arc<RequestTelemetry>,
) -> Result<Value, GatewayError> {
    let data = response
        .get("data")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| failure("上游没有返回图片数据"))?;
    let n = payload.get("n").and_then(Value::as_u64).unwrap_or(1) as usize;
    if data.len() < n {
        return Err(failure(format!(
            "请求 {n} 张图片，上游仅返回 {} 张，未重复调用生图",
            data.len()
        )));
    }
    let snapshot = telemetry.snapshot();
    let session = state.store().get_session_by_id(&snapshot.account_id);
    let proxy = session.as_ref().and_then(|session| {
        crate::server::core::proxies::ResolvedProxy::from_json(&session.proxy)
            .ok()
            .flatten()
    });
    let system_proxy = snapshot
        .provider
        .as_deref()
        .and_then(adapter_by_id)
        .is_some_and(|adapter| adapter.system_proxy_when_unset())
        && proxy.is_none();
    let output_format = payload
        .get("output_format")
        .and_then(Value::as_str)
        .and_then(|format| match format {
            "png" => Some(image::ImageFormat::Png),
            "jpeg" => Some(image::ImageFormat::Jpeg),
            "webp" => Some(image::ImageFormat::WebP),
            _ => None,
        });
    let compression = payload
        .get("output_compression")
        .and_then(Value::as_u64)
        .map(|value| value as u8);
    let want_url = payload.get("response_format").and_then(Value::as_str) == Some("url");
    let mut out = Vec::with_capacity(n);
    let mut assets = Vec::new();
    let mut asset_indices = Vec::new();
    let mut actual_format = None;
    for item in data.iter().take(n) {
        let mut entry = json!({});
        if let Some(prompt) = item.get("revised_prompt").and_then(Value::as_str) {
            entry["revised_prompt"] = json!(prompt);
        }
        let b64 = item
            .get("b64_json")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty());
        let link = item
            .get("url")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty());
        if want_url && b64.is_none() && output_format.is_none() && compression.is_none() {
            let link = link.ok_or_else(|| failure("上游图片条目缺少 url 或 b64_json"))?;
            let parsed = url::Url::parse(link).map_err(|_| failure("上游返回了无效图片链接"))?;
            if !matches!(parsed.scheme(), "https" | "http")
                || !parsed.username().is_empty()
                || parsed.password().is_some()
            {
                return Err(failure("上游返回的图片链接不符合安全要求"));
            }
            entry["url"] = json!(link);
        } else {
            let bytes = if let Some(b64) = b64 {
                if b64.len() > MAX_IMAGE_BYTES * 4 / 3 + 4 {
                    return Err(failure("上游内联图片超过 32 MiB 上限"));
                }
                base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|_| failure("上游生成图片不是有效 base64"))?
            } else {
                download(
                    link.ok_or_else(|| failure("上游图片条目缺少 url 或 b64_json"))?,
                    proxy.as_ref(),
                    system_proxy,
                )
                .await?
            };
            let (bytes, mime) =
                tokio::task::spawn_blocking(move || encode(bytes, output_format, compression))
                    .await
                    .map_err(|_| failure("生成图片编码任务未完成"))??;
            actual_format = Some(mime.trim_start_matches("image/").to_string());
            if want_url {
                asset_indices.push(out.len());
                assets.push((bytes, mime));
            } else {
                entry["b64_json"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
            }
        }
        out.push(entry);
    }
    if !assets.is_empty() {
        let base = base_url(headers, state.port)?;
        let ids = image_assets::store(assets)?;
        for (index, id) in asset_indices.into_iter().zip(ids) {
            let link = base
                .join(&format!("/v1/images/files/{id}"))
                .map_err(|_| failure("无法构造生成图片的下载链接"))?;
            out[index]["url"] = json!(link.as_str());
        }
    }
    let mut result = json!({
        "created": response.get("created").and_then(Value::as_i64).filter(|created| *created > 0).unwrap_or_else(|| crate::server::logging::now_ms() / 1000),
        "data": out,
    });
    for key in [
        "usage",
        "points_consumed",
        "quality",
        "background",
        "size",
        "output_format",
    ] {
        if let Some(value) = response.get(key).filter(|value| !value.is_null()) {
            result[key] = value.clone();
        }
    }
    if let Some(format) = actual_format {
        result["output_format"] = json!(format);
    }
    Ok(result)
}
