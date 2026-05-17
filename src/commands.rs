#[cfg(all(target_arch = "wasm32", debug_assertions))]
use crate::preview_data::{
    browser_preview_action, browser_preview_inventory, browser_preview_resource_detail,
    preview_service_catalog,
};
use js_sys::{Function, Promise, Reflect};
use serde::{Serialize, de::DeserializeOwned};
#[cfg(all(target_arch = "wasm32", debug_assertions))]
use serde_json::json;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// Invoke a Tauri command and deserialize its JavaScript response into `T`.
///
/// # Errors
///
/// Returns an error when the Tauri bridge is unavailable, the command rejects,
/// or the response cannot be deserialized into the requested type.
pub async fn invoke_command<T>(command: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    invoke_command_internal(command, JsValue::UNDEFINED, None).await
}

/// Invoke a Tauri command with JSON-serializable arguments.
///
/// # Errors
///
/// Returns an error when arguments cannot be serialized, the Tauri bridge is
/// unavailable, the command rejects, or the response cannot be deserialized.
pub async fn invoke_command_with_args<T, A>(command: &str, args: &A) -> Result<T, String>
where
    T: DeserializeOwned,
    A: Serialize,
{
    let preview_args = serde_json::to_value(args).map_err(|err| err.to_string())?;
    let args = serde_wasm_bindgen::to_value(args).map_err(|err| err.to_string())?;

    invoke_command_internal(command, args, Some(preview_args)).await
}

async fn invoke_command_internal<T>(
    command: &str,
    args: JsValue,
    preview_args: Option<serde_json::Value>,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let (this_arg, invoke) = match tauri_invoke() {
        Ok(command_bridge) => command_bridge,
        Err(message) => {
            if let Some(preview_result) = browser_preview_result(command, preview_args.as_ref()) {
                return preview_result;
            }

            return Err(message);
        }
    };
    let promise = invoke
        .call2(&this_arg, &JsValue::from_str(command), &args)
        .map_err(|value| js_error(&value))?;
    let promise = promise
        .dyn_into::<Promise>()
        .map_err(|_| "Tauri invoke did not return a Promise.".to_owned())?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|value| js_error(&value))?;

    serde_wasm_bindgen::from_value(value).map_err(|err| err.to_string())
}

fn tauri_invoke() -> Result<(JsValue, Function), String> {
    let window = web_sys::window().ok_or_else(|| "No browser window is available.".to_owned())?;
    let tauri = Reflect::get(window.as_ref(), &JsValue::from_str("__TAURI__"))
        .map_err(|value| js_error(&value))?;

    if tauri.is_null() || tauri.is_undefined() {
        return Err("Tauri APIs are unavailable. Start the app with `cargo tauri dev`.".to_owned());
    }

    let core =
        Reflect::get(&tauri, &JsValue::from_str("core")).map_err(|value| js_error(&value))?;
    let invoke =
        Reflect::get(&core, &JsValue::from_str("invoke")).map_err(|value| js_error(&value))?;
    let invoke = invoke
        .dyn_into::<Function>()
        .map_err(|_| "window.__TAURI__.core.invoke is not a function.".to_owned())?;

    Ok((core, invoke))
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_result<T>(
    command: &str,
    args: Option<&serde_json::Value>,
) -> Option<Result<T, String>>
where
    T: DeserializeOwned,
{
    if !is_local_browser_preview() {
        return None;
    }

    let value = match command {
        "floci_health" => json!({
            "ok": true,
            "url": "http://localhost:4566/_floci/health",
            "status": 200,
            "floci_version": "preview",
            "health_status": "running",
            "body": {
                "status": "running",
                "version": "preview",
                "services": {
                    "s3": "running",
                    "dynamodb": "running",
                    "sqs": "available",
                    "lambda": "available"
                },
                "edition": "local-preview"
            },
            "error": null
        }),
        "service_catalog" => preview_service_catalog(),
        "service_inventory" => browser_preview_inventory(args)?,
        "service_resource_detail" => browser_preview_resource_detail(args)?,
        "service_execute_action" => browser_preview_action(args)?,
        _ => return None,
    };

    Some(serde_json::from_value(value).map_err(|err| err.to_string()))
}

#[cfg(not(all(target_arch = "wasm32", debug_assertions)))]
fn browser_preview_result<T>(
    _command: &str,
    _args: Option<&serde_json::Value>,
) -> Option<Result<T, String>>
where
    T: DeserializeOwned,
{
    None
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn is_local_browser_preview() -> bool {
    let Ok(hostname) = web_sys::window()
        .map(|window| window.location().hostname())
        .unwrap_or_else(|| Err(JsValue::NULL))
    else {
        return false;
    };

    matches!(hostname.as_str(), "localhost" | "127.0.0.1" | "::1")
}

fn js_error(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "JavaScript interop failed.".to_owned())
}
