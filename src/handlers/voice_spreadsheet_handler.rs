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
    AudioProcessInfo,
    AudioPauseInfo,
    Silence,
    AudioDetectPauseInfo
};
use crate::utils::{/*jwt::jwt_auth, */ response::into_reponse};
use crate::utils::audio_process::{replace_audio, extract_audio, launch_inference_audio, launch_normalizing_audio, generate_yaml, get_system_time, generate_voice_code, remove_silence_audio, get_silences_audio, similarity_voice_code, path_exists, convert_to_wav, denoise_audio, adjust_amplitude_audio};
use crate::utils::download_audio::{download_template, download_template_with_path};
use crate::utils::upload_audio::{upload_audio, is_existed, is_existed_template, is_existed_audio, upload_template, upload_asset};
use std::io;
use std::env;

pub async fn process_audio(
    payload: String,
    Extension(_pool): Extension<Arc<PgPool>>
) -> impl IntoResponse {
    println!("process_audio -> payload : {:#?}", payload);
    let params: AudioProcessInfo;
    let response = serde_json::from_str(&payload);
    match response {
        Ok(p) => params = p,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "API params are missing!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };

    if params.template_bucket.len() == 0 ||  params.template_key.len() == 0 || params.template_region.len() == 0 {
        let ret = serde_json::json!({
            "error": "template must be existed!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let mut file_extension = String::from("wav");
    if params.denoise == true && params.silence_removal == false && params.amplitude_equalize == false {
        file_extension = String::from("webm");
        let template_s3_path = is_existed_template(&params.output_region, &params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &file_extension).await;
        if template_s3_path.len() > 0 {
            let mut response_data = HashMap::new();
            response_data.insert("template".to_string(), template_s3_path);
            response_data.insert("audio_assets".to_string(), String::from(""));
            return into_reponse(200, serde_json::json!(response_data));
        }
        let mut template_path = format!("Names/Temp/{}_detect.{}", Uuid::new_v4(), &file_extension); 
        template_path = match download_template_with_path(&params.template_region, &params.template_bucket, &params.template_key, &template_path).await {
            Ok(template_path) => template_path,
            Err(_e) => {
                let ret = serde_json::json!({
                    "error": "Failed to download template audio!".to_string(),
                });
                return into_reponse(400, ret);
            }
        };
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
        let audio_path = extract_audio(&template_path);
        denoise_audio(&audio_path);
        // replace processed audio with audio in video
        replace_audio(&template_path, &audio_path);

        // upload to s3 bucket
        let template_s3_path = upload_template(&params.output_region, &params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &template_path, &file_extension).await;
       
        let _res = match std::fs::remove_file(template_path) {
            Ok(_value) => { println!("success to remove temp file"); },
            Err(_e) => { println!("failed to remove temp file"); }
        };

        let mut response_data = HashMap::new();
        response_data.insert("template".to_string(), template_s3_path);
        response_data.insert("audio_assets".to_string(), String::from(""));
        return into_reponse(200, serde_json::json!(response_data));
    }
    let mut template_path = format!("Names/Temp/{}_detect.{}", Uuid::new_v4(), &file_extension); 
    template_path = match download_template_with_path(&params.template_region, &params.template_bucket, &params.template_key, &template_path).await {
        Ok(template_path) => template_path,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "Failed to download template audio!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };
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

    // convert any format into standard .wav
    convert_to_wav(&template_path);

    // denoise
    denoise_audio(&template_path);

    println!("multi-threading has been started ========================================================== ");
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

            let path = is_existed_audio(&output_region, &output_bucket, &template_region, &template_bucket, &template_key, &audio_region, &audio_bucket, &audio_key).await;
            if path.len() > 0 {
                let mut _output_array = match output_array_clone.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                }; 

                _output_array.insert(audio_key.clone(), path);
                continue;
            } 
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
                    convert_to_wav(&audio_path);

                    // denoise
                    denoise_audio(&audio_path);

                    //remove silences
                    if params.silence_removal {
                        remove_silence_audio(&audio_path);
                        println!("Audio file has been removed silences.");
                    }

                    //adjust amplitude
                    if params.amplitude_equalize {
                        if !adjust_amplitude_audio(&template_path_clone, &audio_path) {
                            let mut _output_array = match output_array_clone.lock() {
                                Ok(guard) => guard,
                                Err(poisoned) => poisoned.into_inner(),
                            }; 
                            _output_array.insert(audio_key.clone(), String::from(""));
                            return;
                        }
                        println!("Audio file's amplitude has been adjusted.");
                    }
                } else {
                    let mut _output_array = match output_array_clone.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    }; 
                    _output_array.insert(audio_key.clone(), String::from(""));
                    return;
                }

                let audio_s3_path = upload_asset(&output_region, &output_bucket, &template_region, &template_bucket, &template_key, &audio_region, &audio_bucket, &audio_key, &audio_path).await;
                let _res = match std::fs::remove_file(audio_path) {
                    Ok(_value) => { println!("success to remove temp file"); },
                    Err(_e) => { println!("failed to remove temp file"); }
                };

                let mut _output_array = match output_array_clone.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                }; 
                _output_array.insert(audio_key.clone(), audio_s3_path);
                
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

    let mut response_data = HashMap::new();
    response_data.insert("template".to_string(), String::from(""));
    response_data.insert("audio_assets".to_string(), serde_json::json!(output_array).to_string());

    into_reponse(200, serde_json::json!(response_data))
}
#[debug_handler]
pub async fn detect_pauses(
    payload: String,
    Extension(pool): Extension<Arc<PgPool>>
) -> impl IntoResponse {
    println!("payload : {:#?}", payload);

    let params: AudioPauseInfo;
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
    
    if params.audio_bucket.len() == 0 {
        let ret = serde_json::json!({
            "error": "audio bucket is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }
    if params.audio_key.len() == 0 {
        let ret = serde_json::json!({
            "error": "audio key is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let mut query = "SELECT * FROM audio_detect_pauses WHERE s3_path = ".to_string();
    write!(query, "'{}'", format!("{}/{}/{}", &params.audio_region, &params.audio_bucket, &params.audio_key)).unwrap();
    let str_query: &str = &query[..];
    match sqlx::query_as::<_,AudioDetectPauseInfo>(str_query).fetch_all(&*pool).await {
        Ok(user_records) => {
            let mut result_array = Vec::new();
            if user_records.len() > 0 {
                for item in user_records.iter() {
                    result_array.push(item);
                }
                match serde_json::from_str(&result_array[0].pauses) {
                    Ok(results) => {
                        return into_reponse(200, results);
                    },
                    Err(_e) => {}
                };
            }
        },
        Err(_e) => {}
    }
   

    let mut audio_path = format!("Names/Temp/{}_detect.wav", Uuid::new_v4()); 

    audio_path = match download_template_with_path(&params.audio_region, &params.audio_bucket, &params.audio_key, &audio_path).await {
        Ok(audio_path) => audio_path,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "Failed to download template audio!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };

    println!("audio_path : {}", audio_path);
    if audio_path.len() == 0 {
        let ret = serde_json::json!({
            "error": "audio is not existed!".to_string(),
        });
        return into_reponse(400, ret);
    }
    let file_size = match std::fs::metadata(&audio_path) {
        Ok(value) => value.len(),
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "Failed to read template audio!".to_string(),
            });
            return into_reponse(400, ret);
        }
    };
    if file_size == 0 {
        return into_reponse(400, serde_json::json!({"error": "audio is empty!".to_string()}));
    }

    convert_to_wav(&audio_path);

    denoise_audio(&audio_path);

    let silences : Vec<Silence> = get_silences_audio(&audio_path);
    let mut output_array = Vec::new();

    for i in 0..silences.len() {
        let start_time = (silences[i].start_time * 1000.0) as i64;
        let end_time = (silences[i].end_time * 1000.0) as i64;
        let mut array = Vec::new();
        array.push(start_time);
        array.push(end_time);
        output_array.push(array);
    }

    let _res = match std::fs::remove_file(audio_path) {
        Ok(_value) => { println!("success to remove temp file"); },
        Err(_e) => { println!("failed to remove temp file"); }
    };
    
    match sqlx::query_as!(
        AudioDetectPauseInfo,
        r#"INSERT INTO audio_detect_pauses (s3_path, pauses) VALUES ($1, $2) RETURNING *"#,
        format!("{}/{}/{}", &params.audio_region, &params.audio_bucket, &params.audio_key),
        serde_json::json!(output_array).to_string()
    )
    .fetch_one(&*pool).await {
        Ok(results) => {
            println!("detect pauses have been inserted in database.");
        },
        Err(_e) => {}
    }
    
    into_reponse(200, serde_json::json!(output_array))
}
// API
#[debug_handler]
pub async fn generate_audio(
    payload: String,
    Extension(pool): Extension<Arc<PgPool>>,
) -> impl IntoResponse {

    println!("payload : {:#?}", payload);

    let params: BatchUploadParam;
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

    let mut template_path = format!("Names/Temp/{}.wav", encode(format!("{}/{}", params.template_bucket.clone(), params.template_key.clone()))); 
    if !path_exists(&*template_path) {
        template_path = download_template(&params.template_region, &params.template_bucket, &params.template_key).await.unwrap();
    }
    println!("template_path : {}", template_path);

    if template_path.len() == 0 {
        let ret = serde_json::json!({
            "error": "template audio is not existed!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let mut template_voice_code_path = format!("{}.csv", template_path);
    if !path_exists(&*template_voice_code_path) {
        /*
        audio - processing: denoising, removing silence, 
        */
        // convert any audio format to .wav format using ffmpeg
        convert_to_wav(&template_path);

        // denoise
        denoise_audio(&template_path);
        
        //remove silences
        remove_silence_audio(&template_path);
        println!("Audio file has been removed silences.");

        template_voice_code_path = generate_voice_code(&template_path);
    }

    println!("multi-threading has been started ========================================================== ");
    let output_array = Arc::new(Mutex::new(HashMap::new()));
    let target_array = Arc::new(Mutex::new(HashMap::new()));
    let max_threads_count = 30;
    let mut index = 0;
    let similarity_threshold = 0.65;

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

            let path = is_existed(&template_bucket, &template_key, &user_key).await;
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
                        let similarity_value = similarity_voice_code(&template_code_path, &item.voice_code).unwrap();
                        if similarity_value > threshold {
                            threshold = similarity_value;
                            vst_file_path = item.file_path.clone();
                        }
                    }
                    println!("similarity_value : {} ---> {}", vst_file_path, threshold);

                    if threshold > similarity_threshold {
                        let yaml_path = generate_yaml(&vst_file_path, &template_audio_path, &target_path).unwrap();
                        launch_inference_audio(&yaml_path);

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
        let yaml_path = generate_yaml_path(&target_path).unwrap();
        if path_exists(&*yaml_path) {
            launch_normalizing_audio(&yaml_path);
            
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
            let template_bucket = params.template_bucket.clone();
            let template_key = params.template_key.clone();
            let output_array_clone = Arc::clone(&output_array);
            
            handles.push(tokio::spawn(async move {
                thread::sleep(Duration::from_millis(10));
                println!("Thread {} uploading... ", user_key);

                let yaml_path = generate_yaml_path(&target_path).unwrap();
                if path_exists(&*yaml_path) {
                    let _res = std::fs::remove_file(yaml_path);
                }
                
                let vst_path = generate_vst_path(&target_path);

                if path_exists(&*vst_path) {
                    let s3_path = upload_audio(&template_bucket, &template_key, &user_key, &vst_path).await;
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

pub fn generate_yaml_path(target_path : &String) -> Result<String, io::Error> {
    let yaml_path = format!("{:#?}/VST/{}.yaml", env::current_dir()?.display(), target_path).replace("\"", "");
    Ok(yaml_path)
}

pub fn generate_vst_path(target_path : &String) -> String {
    let vst_path = format!("VST/custom_test/{}_gen.wav", target_path);
    vst_path
}