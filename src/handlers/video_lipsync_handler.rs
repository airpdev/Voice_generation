extern crate base64;
use base64::{encode};
use axum::{extract::Extension, Json, response::IntoResponse};
use axum_macros::debug_handler;
use openapi_rs::openapi_proc_macro::handler;
use okapi::openapi3::RefOr;
use openapi_rs::gen::OpenApiGenerator;
use uuid::Uuid;
use sqlx::PgPool;
use std::fmt::Write;
use std::fs::Metadata;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use crate::models::voice_generation::{
    LipsyncInfo,
    LipsyncInputInfo
};
use crate::utils::{response::into_reponse};
use crate::utils::audio_process::{path_exists, convert_to_wav, denoise_audio};
use crate::utils::video_process::{launch_lipsync_generate};
use crate::utils::download_audio::{download_template_with_path};
use crate::utils::upload_audio::{upload_audio_path, is_existed_lipsync_video, upload_video_lipsync};
use std::io;
use std::{env, ffi::OsStr};
use serde_json::Value;
use rdkafka::producer::FutureProducer;
use tasque::TaskQueue;
use async_std::task;

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref S3_BUCKET: String = ensure_var("S3_BUCKET").unwrap();
    static ref S3_REGION: String = ensure_var("S3_REGION").unwrap();
}

pub async fn generate_video_lipsync(
    payload: String,
    Extension(pool): Extension<Arc<PgPool>>
) -> impl IntoResponse {
    println!("payload : {:#?}", payload);

    let params: LipsyncInfo;
    let response = serde_json::from_str(&payload);
    match response {
        Ok(p) => params = p,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "API params are incorrect!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };

    /*
    let mut cached_output_array = HashMap::new();
    let mut is_all_cached = true;
    for i in 0 .. params.audio_assets.len() {
        let audio_asset_key = params.audio_assets[i].clone();
        
        let path = is_existed_lipsync_video(&params.output_region, &params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &params.audio_region, &params.audio_bucket, &audio_asset_key).await;
        if path.len() == 0 {
            is_all_cached = false;
            break;
        }
        cached_output_array.insert(audio_asset_key.clone(), path);
    }
    if is_all_cached == true {
        return into_reponse(200, serde_json::json!(cached_output_array));
    }
*/
    let mut template_path = format!("Names/Temp/{}.webm", Uuid::new_v4()); 
    template_path = match download_template_with_path(&params.template_region, &params.template_bucket, &params.template_key, &template_path).await {
        Ok(template_path) => template_path,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "Failed to download template audio!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };
    println!("template_path : {}", template_path);

    let file_size = match std::fs::metadata(&template_path) {
        Ok(value) => value.len(),
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "Failed to read template audio!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };

    if file_size == 0 {
        let ret = serde_json::json!({
            "error": "template audio is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let output_array = Arc::new(Mutex::new(HashMap::new()));
    let max_threads_count = 3;
    let mut index = 0;
    while index < params.audio_assets.len() {
        let mut handles = vec![];
        for i in index .. max_threads_count + index {
            if i >= params.audio_assets.len() {
                break;
            }
        
            let output_region = params.output_region.clone();
            let output_bucket = params.output_bucket.clone();
            let template_key = params.template_key.clone();
            let template_region = params.template_region.clone();
            let template_bucket = params.template_bucket.clone();
            let audio_region = params.audio_region.clone();
            let audio_bucket = params.audio_bucket.clone();
            let audio_key = params.audio_assets[i].clone();
            let output_array_clone = Arc::clone(&output_array);
            let template_path_clone = template_path.clone();

            /*
            let path = is_existed_lipsync_video(&params.output_region, &params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &params.audio_region, &params.audio_bucket, &audio_key).await;
            if path.len() > 0 {
                let mut _output_array = match output_array_clone.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                }; 

                _output_array.insert(audio_key.clone(), path);
                continue;
            } */
            handles.push(tokio::spawn(async move {
                thread::sleep(Duration::from_millis(10));

                let mut audio_path = format!("Names/Temp/{}_process.wav", Uuid::new_v4()); 
                audio_path = download_template_with_path(&audio_region, &audio_bucket, &audio_key, &audio_path).await.unwrap();

                let file_size = match std::fs::metadata(&audio_path) {
                    Ok(value) => value.len(),
                    Err(_e) => 0,
                };
                if file_size > 0 {
                    // convert any audio format to .wav format using ffmpeg
                    //convert_to_wav(&audio_path);

                    // denoise
                    //denoise_audio(&audio_path);

                    let template_absolute_path = match get_absolute_path(template_path_clone.clone()) {
                        Ok(template_absolute_path) => template_absolute_path,
                        Err(_e) => "".to_string()
                    };
                    let audio_absolute_path = match get_absolute_path(audio_path.clone()) {
                        Ok(audio_absolute_path) => audio_absolute_path,
                        Err(_e) => "".to_string()
                    };
                    let mut output_path = format!("Names/Temp/{}_lipsync.mp4", Uuid::new_v4()); 
                    let output_absolute_path = match get_absolute_path(output_path.clone()) {
                        Ok(output_absolute_path) => output_absolute_path,
                        Err(_e) => "".to_string()
                    };
                    let params : LipsyncInputInfo = LipsyncInputInfo{model : String::from("wav2lip.pth"), 
                                                                    video: template_absolute_path, 
                                                                    audio : audio_absolute_path,
                                                                    output: output_absolute_path};

                    let params_str = serde_json::to_string(&params).unwrap(); 
                    println!("python params: {}", params_str);

                    launch_lipsync_generate(encode(params_str));
                    
                    let video_s3_path = upload_video_lipsync(&output_region, &output_bucket, &template_region, &template_bucket, &template_key, &audio_region, &audio_bucket, &audio_key, &output_path).await;
                    let _res = match std::fs::remove_file(output_path) {
                        Ok(_value) => { println!("success to remove temp file"); },
                        Err(_e) => { println!("failed to remove temp file"); }
                    };

                    let mut _output_array = match output_array_clone.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    }; 
                    _output_array.insert(audio_key.clone(), video_s3_path);

                } else {
                    let mut _output_array = match output_array_clone.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    }; 
                    _output_array.insert(audio_key.clone(), String::from(""));
                    return;
                }
            }));    
        }

        for handle in handles {
            handle.await.unwrap();
        }

        index = index + max_threads_count;
    }
    
    let _res = match std::fs::remove_file(template_path) {
        Ok(_value) => { println!("success to remove temp file"); },
        Err(_e) => { println!("failed to remove temp file"); }
    };

    into_reponse(200, serde_json::json!(output_array))
}

pub fn get_absolute_path(target_path : String) -> Result<String, io::Error> {
    let path = format!("{:#?}/{}", env::current_dir()?.display(), target_path).replace("\"", "");
    Ok(path)
}
pub fn get_absolute_output_path(target_path : String) -> Result<String, io::Error> {
    let path = format!("{:#?}/{}_output.wav", env::current_dir()?.display(), target_path).replace("\"", "");
    Ok(path)
}
