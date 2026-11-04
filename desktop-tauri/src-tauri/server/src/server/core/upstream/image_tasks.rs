//! ModelScope API-Inference 官方异步 Images 协议：创建任务后只轮询，绝不再次提交。

use std::{sync::Arc, time::Duration};
use futures::StreamExt;
use serde_json::{json, Value};
use super::{images::ImageFailure, request::TransportRequest, usage::RequestTelemetry};
use crate::server::core::egress;

pub(super) async fn collect(payload: &Value, transport: &TransportRequest, telemetry: &Arc<RequestTelemetry>) -> Result<Value, ImageFailure> {
    let id = payload.get("task_id").and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 128 && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
        .ok_or_else(|| ImageFailure::invalid("图片任务未返回有效的 task_id，上游可能已消耗额度"))?;
    let base = url::Url::parse(&transport.url).map_err(|_| ImageFailure::invalid("图片任务地址无效"))?;
    if base.scheme() != "https" || !matches!(base.host_str(), Some("api-inference.modelscope.cn" | "api-inference.modelscope.ai")) {
        return Err(ImageFailure::invalid("图片任务只能在 ModelScope 官方推理域轮询"));
    }
    let target = base.join(&format!("/v1/tasks/{id}")).map_err(|_| ImageFailure::invalid("图片任务地址无法生成"))?;
    let client = egress::client_without_redirects(transport.proxy.as_ref());
    loop {
        let mut request = client.get(target.clone()).timeout(Duration::from_secs(30));
        for (name, value) in &transport.headers {
            if !name.eq_ignore_ascii_case("x-modelscope-async-mode") { request = request.header(name, value); }
        }
        let response = request.header("X-ModelScope-Task-Type", "image_generation").send().await
            .map_err(|error| ImageFailure::invalid(format!("图片任务 {id} 查询失败：{error}；未重新提交任务")))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(ImageFailure::invalid(format!("图片任务 {id} 查询返回 HTTP {status}，未重新提交任务")));
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| ImageFailure::invalid("图片任务状态传输中断"))?;
            if bytes.len().saturating_add(chunk.len()) > 1024 * 1024 { return Err(ImageFailure::invalid("图片任务状态响应过大")); }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| ImageFailure::invalid("图片任务状态不是有效 JSON"))?;
        match value["task_status"].as_str() {
            Some("SUCCEED") => {
                let images = value["output_images"].as_array().filter(|images| !images.is_empty()).ok_or_else(|| ImageFailure::invalid("图片任务完成但未返回图片"))?;
                let data = images.iter().map(|image| {
                    let url = image.as_str().filter(|url| url.starts_with("https://")).ok_or_else(|| ImageFailure::invalid("图片任务返回无效图片地址"))?;
                    Ok(json!({ "url": url }))
                }).collect::<Result<Vec<_>, ImageFailure>>()?;
                let mut result = json!({ "created": crate::server::logging::now_ms() / 1000, "data": data, "task_id": id });
                if let Some(usage) = value.get("usage") { telemetry.report_usage(usage); result["usage"] = usage.clone(); }
                return Ok(result);
            }
            Some("FAILED" | "CANCELED" | "CANCELLED") => {
                let message = value.get("message").or_else(|| value.get("error_message")).and_then(Value::as_str).unwrap_or("上游图片任务失败");
                return Err(ImageFailure::invalid(format!("图片任务 {id} 失败：{message}")));
            }
            Some("PENDING" | "RUNNING" | "WAITING") => tokio::time::sleep(Duration::from_secs(5)).await,
            _ => return Err(ImageFailure::invalid(format!("图片任务 {id} 返回未知状态，未重新提交任务"))),
        }
    }
}
