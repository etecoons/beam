use crate::types::{FrontendConfig, FileListResponse};
use crate::utils::encode_path;

pub async fn fetch_config() -> Result<FrontendConfig, String> {
    let res = gloo_net::http::Request::get("/api/auth/config")
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        return Err(format!("Failed to fetch config: HTTP {}", res.status()));
    }
    
    let config: FrontendConfig = res.json().await.map_err(|e| e.to_string())?;
    Ok(config)
}



pub async fn verify_pin_api(pin: &str) -> Result<bool, String> {
    let res = gloo_net::http::Request::post("/api/auth/verify-pin")
        .json(&serde_json::json!({ "pin": pin }))
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if res.status() == 200 {
        Ok(true)
    } else if res.status() == 429 {
        Err("Too many PIN verification attempts. Please wait before trying again.".to_string())
    } else {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("Authentication failed");
        Err(err_msg.to_string())
    }
}

pub async fn logout_api() -> Result<(), String> {
    let _ = gloo_net::http::Request::post("/api/auth/logout")
        .send()
        .await;
    Ok(())
}

pub async fn fetch_files() -> Result<FileListResponse, String> {
    let res = gloo_net::http::Request::get("/api/files")
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        return Err(format!("HTTP {}", res.status()));
    }
    
    let list: FileListResponse = res.json().await.map_err(|e| e.to_string())?;
    Ok(list)
}

pub async fn delete_file_api(file_path: &str) -> Result<(), String> {
    let encoded_path = encode_path(file_path);
    let url = format!("/api/files/delete/{}", encoded_path);
    
    let res = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to delete item");
        return Err(err_msg.to_string());
    }
    
    Ok(())
}

pub async fn rename_file_api(file_path: &str, new_name: &str) -> Result<(), String> {
    let encoded_path = encode_path(file_path);
    let url = format!("/api/files/rename/{}", encoded_path);
    
    let res = gloo_net::http::Request::put(&url)
        .json(&serde_json::json!({ "newName": new_name }))
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to rename item");
        return Err(err_msg.to_string());
    }
    
    Ok(())
}

pub fn download_file(file_path: &str) {
    let encoded_path = encode_path(file_path);
    let url = format!("/api/files/download/{}", encoded_path);
    let window = web_sys::window().unwrap();
    let _ = window.open_with_url_and_target(&url, "_blank");
}

pub async fn init_upload(filename: &str, file_size: u64, batch_id: &str) -> Result<String, String> {
    let url = "/api/upload/init";
    let body = serde_json::json!({
        "filename": filename.replace('\\', "/"),
        "fileSize": file_size
    });
    
    let res = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/json")
        .header("X-Batch-ID", batch_id)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("details")
            .or_else(|| err_json.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Upload initialization failed");
        return Err(err_msg.to_string());
    }
    
    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let upload_id = data.get("uploadId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing uploadId in response".to_string())?;
        
    Ok(upload_id.to_string())
}

pub async fn upload_chunk(upload_id: &str, batch_id: &str, data: Vec<u8>) -> Result<f64, String> {
    let url = format!("/api/upload/chunk/{}", upload_id);
    
    let res = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/octet-stream")
        .header("X-Batch-ID", batch_id)
        .body(data)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        return Err(format!("HTTP {} {}", res.status(), res.status_text()));
    }
    
    let data_json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let progress = data_json.get("progress")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
        
    Ok(progress)
}

pub async fn cancel_upload(upload_id: &str) -> Result<(), String> {
    let url = format!("/api/upload/cancel/{}", upload_id);
    let _ = gloo_net::http::Request::post(&url)
        .send()
        .await;
    Ok(())
}
