extern crate base64;
use base64::{encode};
use axum::{
    extract::{Extension, /*TypedHeader*/},
    //headers::{authorization::Bearer, Authorization},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use sqlx::PgPool;
use std::fmt::Write;
use uuid::Uuid;
use std::collections::HashMap;

use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use crate::models::voice_generation::{
    AudioInfo,
    BatchUploadParam,
};
use crate::utils::{/*jwt::jwt_auth, */ response::into_reponse};
use crate::utils::audio_process::{launch_inference_audio, launch_normalizing_audio, generate_yaml, get_system_time, generate_voice_code, remove_silence_audio, similarity_voice_code, path_exists, convert_to_wav, denoise_audio};
use crate::utils::download_audio::{download_template};
use crate::utils::upload_audio::{upload_audio, is_existed};
use std::io;
use std::{env, ffi::OsStr};

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref S3_BUCKET: String = ensure_var("S3_BUCKET").unwrap();
    static ref S3_REGION: String = ensure_var("S3_REGION").unwrap();
}

// API
#[debug_handler]
pub async fn generate_audio(
    payload: String,
    //cookies: TypedHeader<Authorization<Bearer>>,
    Extension(pool): Extension<Arc<PgPool>>
) -> impl IntoResponse {
    /*
    let _user_id: String;
    let res = jwt_auth(cookies).await;
    match res {
        Ok(v) => _user_id = v,
        Err(e) => {
            let ret = serde_json::json!({
                "error": format!("{:?}", e),
            });
            return into_reponse(404, ret);
        }
    };
*/
    println!("payload : {:#?}", payload);

    let params: BatchUploadParam = serde_json::from_str(&payload).unwrap();

    if params.template_bucket.len() == 0 {
        let ret = serde_json::json!({
            "error": "template bucket is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    if params.template_key.len() == 0 {
        let ret = serde_json::json!({
            "error": "template key is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    if params.transcripts.len() == 0 {
        let ret = serde_json::json!({
            "error": "transcripts are empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let start_time : u128 = get_system_time();

    let mut template_path = format!("Names/Temp/{}.wav", encode(params.template_key.clone())); 
    if !path_exists(&*template_path) {
        template_path = download_template(params.template_bucket.clone(), params.template_key.clone()).await.unwrap();
    }
    println!("template_path : {}", template_path);

    if template_path.len() == 0 {
        let ret = serde_json::json!({
            "error": "template audio is not existed!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let mut template_voice_code_path = format!("{}.csv", template_path.clone());
    if !path_exists(&*template_voice_code_path) {
        /*
        audio - processing: denoising, removing silence, 
        */
        // convert any audio format to .wav format using ffmpeg
        convert_to_wav(template_path.clone());

        // denoise
        denoise_audio(template_path.clone());
        
        //remove silences
        let _res = remove_silence_audio(template_path.clone());
        println!("Audio file has been removed silences.");

        template_voice_code_path = generate_voice_code(template_path.clone());
    }

    println!("multi-threading has been started ========================================================== ");
    let output_array = Arc::new(Mutex::new(HashMap::new()));
    let target_array = Arc::new(Mutex::new(HashMap::new()));
    let max_threads_count = 30;
    let mut index = 0;
    let similarity_threshold = 0.7;

    // generating yaml and inference part
    while index < params.transcripts.len() {
        let mut handles = vec![];

        for i in index .. max_threads_count + index {
            if i >= params.transcripts.len() {
                break;
            }
            let user_key = params.transcripts[i].clone();
            let pool = Arc::clone(&pool);
            let template_code_path = template_voice_code_path.clone();
            let template_key = params.template_key.clone();
            let template_bucket = params.template_bucket.clone();
            let template_audio_path = template_path.clone();
            let output_array_clone = Arc::clone(&output_array);
            let target_array_clone = Arc::clone(&target_array);

            let path = is_existed(template_bucket.clone(), template_key.clone(), user_key.clone()).await;
            if path.len() > 0 {
                let mut _output_array = output_array_clone.lock().unwrap();
                _output_array.insert(user_key.clone(), path);
                println!("Thread {} exiting... ", user_key);
                continue;
            } 

            handles.push(tokio::spawn(async move {
                thread::sleep(Duration::from_millis(10));
                println!("Thread {} starting... ", user_key);
    
                let mut threshold : f64 = 0.0;
                let target_path = format!("{}", Uuid::new_v4()); 
        
                let mut query = "SELECT * FROM audio_names WHERE lower(file_path) LIKE ".to_string();
                write!(query, "'%{}.wav'", user_key.to_lowercase()).unwrap();
                let str_query: &str = &query[..];
        
                let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*pool).await.unwrap();
                println!("records: {}", name_records.len());
    
                if name_records.len() > 0 {
                    let mut vst_file_path = String::new();
    
                    for item in name_records.iter() {
                        let voice_path = item.voice_code.clone();
                        if !path_exists(&*voice_path){
                            continue;
                        }
                        let similarity_value = similarity_voice_code(template_code_path.clone(), item.voice_code.clone()).unwrap();
                        if similarity_value > threshold {
                            threshold = similarity_value;
                            vst_file_path = item.file_path.clone();
                        }
                    }
                    println!("similarity_value : {} ---> {}", vst_file_path, threshold);

                    if threshold > similarity_threshold {
                        let yaml_path = generate_yaml(vst_file_path.clone(), template_audio_path.clone(), target_path.clone()).unwrap();
                        launch_inference_audio(yaml_path);

                        let mut _target_array = target_array_clone.lock().unwrap();
                        _target_array.insert(user_key.clone(), target_path);

                    } 
                } 
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        index = index + max_threads_count;
    }
    
    // normalizing part
    println!("Started normalizing...");
    for i in 0 .. params.transcripts.len() {
        let user_key = params.transcripts[i].clone();
        let mut _target_array = target_array.lock().unwrap();
        if !_target_array.contains_key(&user_key) {
            continue;
        }

        let target_path = _target_array[&user_key].to_string();
        let yaml_path = generate_yaml_path(target_path.to_string()).unwrap();
        if path_exists(&*yaml_path) {
            launch_normalizing_audio(yaml_path.clone());
            
            break;
        }
    }

    // uploading generated audio to s3 bucket
    println!("Started uploading...");
    index = 0;
    while index < params.transcripts.len() {
        let mut handles = vec![];

        for i in index .. max_threads_count + index {
            if i >= params.transcripts.len() {
                break;
            }
            let user_key = params.transcripts[i].clone();
            let mut _target_array = target_array.lock().unwrap();

            if !_target_array.contains_key(&user_key) {
                continue;
            }
            let target_path = _target_array[&user_key].to_string();
            let template_key = params.template_key.clone();
            let output_array_clone = Arc::clone(&output_array);
            
            handles.push(tokio::spawn(async move {
                thread::sleep(Duration::from_millis(10));
                println!("Thread {} uploading... ", user_key);

                let yaml_path = generate_yaml_path(target_path.clone()).unwrap();
                if path_exists(&*yaml_path) {
                    let _res = std::fs::remove_file(yaml_path);
                }
                
                let vst_path = generate_vst_path(target_path.clone());

                if path_exists(&*vst_path) {
                    let s3_path = upload_audio(template_key.clone(), user_key.clone(), vst_path.clone()).await;
                    let _res = std::fs::remove_file(vst_path);
                    let mut _output_array = output_array_clone.lock().unwrap();
                    _output_array.insert(user_key.clone(), s3_path);
                }

                println!("Thread {} exiting... ", user_key);
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        index = index + max_threads_count;
    }

    println!("multi-threading has been finished ===============================================");
    ////////////////////////////////////////////////////////////////////////////////////////////////////////
/*    let mut output_array = HashMap::new();

    for user_key in params.transcripts.clone().into_iter() {
        let mut threshold : f64 = 0.0;
        let mut target_path = format!("{}", Uuid::new_v4()); 
        let mut s3_path = String::from("");

        //get similar names from database
        let i = &*pool;
        let mut query = "SELECT * FROM audio_names WHERE lower(file_path) LIKE ".to_string();
        write!(query, "'%{}%'", user_key.to_lowercase()).unwrap();
        let str_query: &str = &query[..];
        println!("query: {}", str_query);

        let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*i).await.unwrap();
        if name_records.len() > 0 {
            let mut vst_file_path = String::new();

            for item in name_records.iter() {
                let voice_path = item.voice_code.clone();
                if !path_exists(&*voice_path){
                    continue;
                }
                let similarity_value = similarity_voice_code(template_voice_code_path.clone(), item.voice_code.clone()).unwrap();
                if similarity_value > threshold {
                    threshold = similarity_value;
                    vst_file_path = item.file_path.clone();
                }
            
            }
            println!("similarity_value : {} ---> {}", vst_file_path, threshold);
            
            if threshold > 0.65 {
                vst_generate_audio(vst_file_path, template_path.replace("Names/", ""), target_path.clone());
                let _res = std::fs::remove_file(format!("VST/{}.yaml", target_path));
                target_path = format!("VST/custom_test/{}_gen.wav", target_path);
                s3_path = upload_audio(params.template_key.clone(), user_key.clone(), target_path.clone()).await;
                let _res = std::fs::remove_file(&target_path);

            } else {
                s3_path = String::from("");
            }
        } else {
            s3_path = String::from("");
        }

        output_array.insert(user_key, s3_path);
    } 
*/
    println!("processing_time => {}", get_system_time() - start_time);

    into_reponse(200, serde_json::json!(output_array))
}
pub fn generate_yaml_path(target_path : String) -> Result<String, io::Error> {
    let yaml_path = format!("{:#?}/VST/{}.yaml", env::current_dir()?.display(), target_path.clone()).replace("\"", "");
    Ok(yaml_path)
}

pub fn generate_vst_path(target_path : String) -> String {
    let vst_path = format!("VST/custom_test/{}_gen.wav", target_path);
    vst_path
}