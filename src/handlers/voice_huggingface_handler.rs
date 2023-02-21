extern crate base64;
use axum::{extract::Extension, response::IntoResponse};
use uuid::Uuid;
use sqlx::PgPool;
use std::fmt::Write;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use crate::models::voice_generation::{
    AudioInfo,
    BatchUploadParam,
    HugginefaceInfo,
    AudioSimilarityInfo,
    ProsodyUploadParam,
    ProsodyInfo
};
use crate::utils::{response::into_reponse};
use crate::utils::audio_process::{launch_prosody_audio, launch_huggingface_audio, get_system_time, generate_voice_code, remove_silence_audio, similarity_voice_code, path_exists, convert_to_wav, denoise_audio};
use crate::utils::download_audio::{download_template_with_path};
use crate::utils::upload_audio::{upload_audio_path, is_existed_path};
use std::io;
use std::{env, ffi::OsStr};
use serde_json::Value;
use rdkafka::producer::FutureProducer;
// use microservice_utils::{
//     bhuman_micros::role,
//     server::{
//         producer::produce,
//         rd_msg::{RdMessage, RdType, RdSubType},
//         response::{into_response, AxumRes, AxumResult},
//     },
//     jwt::extractor::AuthToken,
//     jwt::auth::TokenRole,
// };
use tasque::TaskQueue;
use async_std::task;

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref S3_BUCKET: String = ensure_var("S3_BUCKET").unwrap();
    static ref S3_REGION: String = ensure_var("S3_REGION").unwrap();
}
/*
pub fn voice_gen_func(params: BatchUploadParam, template_path : String, pool : Arc<PgPool>, producer : Arc<FutureProducer>, user_id : String) {
    task::block_on(async {
        let mut template_voice_code_path = format!("{}.csv", template_path.clone());
        if !path_exists(&*template_voice_code_path) {
            /*
            audio - processing: denoising, removing silence, 
            */
            // convert any audio format to .wav format using ffmpeg
            convert_to_wav(&template_path);

            // denoise
            denoise_audio(&template_path);
            
            //remove silences
            let _res = remove_silence_audio(&template_path);
            println!("Audio file has been removed silences.");

            template_voice_code_path = generate_voice_code(&template_path);
        }

        let mut output_array = HashMap::new();
        let mut similarity_array = HashMap::new();
        let mut target_array = HashMap::new();
        let mut target_output_array = HashMap::new();
        let similarity_threshold = 0.3;

        // generating yaml and inference part
        for index in 0 .. params.transcripts.len() {
            let user_key = params.transcripts[index].clone();
            
            let path = is_existed_path(&params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &user_key).await;
            if path.len() > 0 {
                let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : path, 
                                                                                        similarity: String::from("")};
                output_array.insert(user_key.clone(), audio_similarity_info);

                println!("Thread {} exiting... ", user_key);
                continue;
            } 

            let mut threshold : f64 = 0.0;
        
            let mut query = "SELECT * FROM audio_names WHERE lower(file_path) LIKE ".to_string();
            write!(query, "'%/{}.wav'", user_key.to_lowercase()).unwrap();
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
                    let similarity_value = similarity_voice_code(&template_voice_code_path, &item.voice_code).unwrap();
                    if similarity_value > threshold {
                        threshold = similarity_value;
                        vst_file_path = item.file_path.clone();
                    }
                }
                if threshold > similarity_threshold {
                    let target_path = get_absolute_path(vst_file_path.clone()).unwrap();
                    println!("similarity_value : {} ---> {}", target_path, threshold);
                    target_array.insert(user_key.clone(), target_path);
                    similarity_array.insert(user_key.clone(), threshold);
        
                    let target_output_path = get_absolute_output_path(vst_file_path.clone()).unwrap();
                    target_output_array.insert(user_key.clone(), target_output_path);
                }
            } 
        }

        if target_array.keys().len() > 0 {
            let template_absolute_path = get_absolute_path(template_path).unwrap();
            let huggingface_info : HugginefaceInfo = HugginefaceInfo{target_path : template_absolute_path, 
                                                                        reference_path_list: target_array, 
                                                                        output_list : target_output_array.clone()};
        
            let params_str = serde_json::to_string(&huggingface_info).unwrap();  
            println!("python params: {}", params_str);
        
            launch_huggingface_audio(params_str);
        }

        // uploading generated audio to s3 bucket
        let output_arc_array = Arc::new(Mutex::new(output_array));
        let target_output_arc_array = Arc::new(Mutex::new(target_output_array));
        println!("Started uploading...");
        let mut handles = vec![];
        for index in 0.. params.transcripts.len() {

            let user_key = params.transcripts[index].clone();
            let mut _target_array = target_output_arc_array.lock().unwrap();

            if !_target_array.contains_key(&user_key) {
                let mut _output_array = output_arc_array.lock().unwrap();
                if !_output_array.contains_key(&user_key) {
                    let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : String::from(""), 
                                                                                                similarity: String::from("")};
                    _output_array.insert(user_key.to_string(), audio_similarity_info);
                }
                continue;
            }

            let target_path = _target_array[&user_key].to_string();
            let audio_similarity = similarity_array[&user_key].to_string();
            let template_region = params.template_region.clone();
            let template_bucket = params.template_bucket.clone();
            let template_key = params.template_key.clone();
            let output_bucket = params.output_bucket.clone();
            let output_array_clone = Arc::clone(&output_arc_array);

            handles.push(tokio::spawn(async move {
                thread::sleep(Duration::from_millis(10));
                println!("Thread {} uploading... ", user_key);

                if path_exists(&*target_path) {
                    let s3_path = upload_audio_path(&output_bucket, &template_region, &template_bucket, &template_key, &user_key, &target_path).await;
                    let _res = std::fs::remove_file(target_path);
                    let mut _output_array = output_array_clone.lock().unwrap();
                    let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : s3_path, 
                                                                                         similarity : audio_similarity};
                    _output_array.insert(user_key.clone(), audio_similarity_info);
                }

                //send_status(user_id, format!("{} has been finished.", user_key)).await;
                println!("Thread {} exiting... ", user_key);
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        let mut response_data = HashMap::new();
        response_data.insert("transcripts".to_string(), serde_json::json!(output_arc_array));
        response_data.insert("actor_id".to_string(), serde_json::json!(params.actor_id));

        //to broker
        let message = RdMessage {
            user_id: user_id.to_string(),
            msg_type: RdType::VoiceGen,
            sub_type: RdSubType::Update,
            message: serde_json::json!(response_data),
        };
        let msg_str = serde_json::to_string(&message).unwrap();  
        let _produce = produce(&msg_str, &producer, "bhuman_channel").await;
    });

    println!("Finished - Voice_Gen");
    
}

#[debug_handler]
#[handler(method = "POST", tag = "voice_generation")]
pub async fn generate_audio_huggingface(
    payload: String,
    AuthToken(user_id, role): AuthToken,
    Extension(pool): Extension<Arc<PgPool>>,
    Extension(producer): Extension<Arc<FutureProducer>>,
 ) -> AxumResult<Json<AxumRes<Value>>> {
    role!(TokenRole::User, &role);
    
    println!("payload : {:#?}", payload);
    
    let mut params: BatchUploadParam;
    let response = serde_json::from_str(&payload);
    match response {
        Ok(p) => params = p,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "API params are incorrect!".to_string(),
            });
            return Ok(axum::Json(AxumRes {code: 400, result: ret}));
        }
    };

    if params.template_region.len() == 0 {
        let ret = serde_json::json!({
            "error": "template region is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }
    if params.template_bucket.len() == 0 {
        let ret = serde_json::json!({
            "error": "template bucket is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.template_key.len() == 0 {
        let ret = serde_json::json!({
            "error": "template key is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.transcripts.len() == 0 {
        let ret = serde_json::json!({
            "error": "transcripts are empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    //let start_time : u128 = get_system_time();

    let mut template_path = format!("Names/Temp/{}.wav", encode(format!("{}/{}/{}", params.template_region.clone(), params.template_bucket.clone(), params.template_key.clone()))); 
    if !path_exists(&*template_path) {
        template_path = download_template(&params.template_region, &params.template_bucket, &params.template_key).await.unwrap();
    }
    println!("template_path : {}", template_path);

    if template_path.len() == 0 {
        let ret = serde_json::json!({
            "error": "template audio is not existed!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }
    let file_size = std::fs::metadata(&template_path).unwrap().len();
    if file_size == 0 {
        let ret = serde_json::json!({
            "error": "template audio is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }
    let task_params = params.clone();
    let task_template_path =  template_path.clone();
    let task_pool = pool.clone();
    let task_producer = producer.clone();

    let voice_gen_queue = TaskQueue::new();
    voice_gen_queue.enqueue(move || voice_gen_func(task_params, task_template_path, task_pool, task_producer, user_id));

    let ret = serde_json::json!({
        "status": "success".to_string(),
    });
    return Ok(axum::Json(AxumRes {code: 200, result: ret}));
}
*/
pub async fn generate_audio_huggingface(
    payload: String,
    Extension(pool): Extension<Arc<PgPool>>
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

    if params.template_region.len() == 0 {
        let ret = serde_json::json!({
            "error": "template region is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

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
    let mut cached_output_array = HashMap::new();
    let mut is_all_cached = true;
    for i in 0 .. params.transcripts.len() {
        let user_key = params.transcripts[i].clone();
        
        let path = is_existed_path(&params.output_region, &params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &user_key).await;
        if path.len() == 0 {
            is_all_cached = false;
            break;
        }
        let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : path, similarity: String::from("")};
        cached_output_array.insert(user_key.clone(), audio_similarity_info);
    }
    if is_all_cached == true {
        return into_reponse(200, serde_json::json!(cached_output_array));
    }

    let start_time : u128 = get_system_time();
    let mut template_path = format!("Names/Temp/{}.wav", Uuid::new_v4()); 
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

    let template_voice_code_path = generate_voice_code(&template_path);

    let mut output_array = HashMap::new();
    let mut similarity_array = HashMap::new();
    let mut target_array = HashMap::new();
    let mut target_output_array = HashMap::new();
    let similarity_threshold = 0.3;

    let max_threads_count = 3;
    let mut index = 0;
    while index < params.transcripts.len() {
        let mut handles = vec![];
        for i in index .. max_threads_count + index {
            if i >= params.transcripts.len() {
                break;
            }
            let user_key = params.transcripts[i].clone();
        
            let path = is_existed_path(&params.output_region, &params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &user_key).await;
            if path.len() > 0 {
                let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : path, 
                                                                                          similarity: String::from("")};
                output_array.insert(user_key.clone(), audio_similarity_info);
    
                println!("Thread {} exiting... ", user_key);
                continue;
            } 
    
            let mut threshold : f64 = 0.0;
        
            let mut query = "SELECT * FROM audio_names WHERE lower(file_path) LIKE ".to_string();
            let _res = write!(query, "'%/{}.wav'", user_key.to_lowercase());
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
                    let similarity_value = match similarity_voice_code(&template_voice_code_path, &item.voice_code){
                        Ok(similarity_value) => similarity_value,
                        Err(_e) => 0.0
                    };
                    if similarity_value > threshold {
                        threshold = similarity_value;
                        vst_file_path = item.file_path.clone();
                    }
                }
                if threshold > similarity_threshold {
                    let target_path = match get_absolute_path(vst_file_path.clone()) {
                        Ok(target_path) => target_path,
                        Err(_e) => "".to_string()
                    };
                    println!("similarity_value : {} ---> {}", target_path, threshold);
                    target_array.insert(user_key.clone(), target_path.clone());
                    similarity_array.insert(user_key.clone(), threshold);
        
                    let target_output_path = match get_absolute_output_path(vst_file_path.clone()) {
                        Ok(target_output_path) => target_output_path,
                        Err(_e) => "".to_string()
                    };
                    target_output_array.insert(user_key.clone(), target_output_path.clone());
    
                    let template_absolute_path = match get_absolute_path(template_path.clone()) {
                        Ok(template_absolute_path) => template_absolute_path,
                        Err(_e) => "".to_string()
                    };
                    handles.push(tokio::spawn(async move {
                        thread::sleep(Duration::from_millis(10));
                        let mut reference_value = HashMap::new();
                        reference_value.insert(user_key.clone(), target_path);
                        let mut output_value = HashMap::new();
                        output_value.insert(user_key.clone(), target_output_path);
                        let huggingface_info : HugginefaceInfo = HugginefaceInfo{target_path : template_absolute_path, 
                                                                        reference_path_list: reference_value, 
                                                                        output_list : output_value};
        
                        let params_str = match serde_json::to_string(&huggingface_info) {
                            Ok(params_str) => params_str,
                            Err(_e) => "".to_string()
                        };
                        println!("python params: {}", params_str);
        
                        launch_huggingface_audio(params_str);
                    }));
                   
                }
            } 
        }

        for handle in handles {
            handle.await.unwrap();
        }

        index = index + max_threads_count;
    }

    // uploading generated audio to s3 bucket
    let output_arc_array = Arc::new(Mutex::new(output_array));
    let target_output_arc_array = Arc::new(Mutex::new(target_output_array));
    println!("Started uploading...");
    let mut handles = vec![];
    for index in 0.. params.transcripts.len() {

        let user_key = params.transcripts[index].clone();
        let mut _target_array = match target_output_arc_array.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }; 

        if !_target_array.contains_key(&user_key) {
            let mut _output_array = match output_arc_array.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            }; 
            if !_output_array.contains_key(&user_key) {
                let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : String::from(""), 
                                                                                            similarity: String::from("")};
                _output_array.insert(user_key.to_string(), audio_similarity_info);
            }
            continue;
        }

        let target_path = _target_array[&user_key].to_string();
        let audio_similarity = similarity_array[&user_key].to_string();
        let template_region = params.template_region.clone();
        let template_bucket = params.template_bucket.clone();
        let template_key = params.template_key.clone();
        let output_region = params.output_region.clone();
        let output_bucket = params.output_bucket.clone();
        let output_array_clone = Arc::clone(&output_arc_array);

        handles.push(tokio::spawn(async move {
            thread::sleep(Duration::from_millis(10));
            println!("Thread {} uploading... ", user_key);

            if path_exists(&*target_path) {
                let s3_path = upload_audio_path(&output_region, &output_bucket, &template_region, &template_bucket, &template_key, &user_key, &target_path).await;
                let _res = std::fs::remove_file(target_path);
                let mut _output_array = match output_array_clone.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                }; 
                let audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : s3_path, 
                                                                                        similarity : audio_similarity};
                _output_array.insert(user_key.clone(), audio_similarity_info);
            }

            //send_status(user_id, format!("{} has been finished.", user_key)).await;
            println!("Thread {} exiting... ", user_key);
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
    
    let _res = match std::fs::remove_file(template_path) {
        Ok(_value) => { println!("success to remove temp file"); },
        Err(_e) => { println!("failed to remove temp file"); }
    };

    println!("processing_time => {}", get_system_time() - start_time);

    into_reponse(200, serde_json::json!(output_arc_array))
}

pub fn get_absolute_path(target_path : String) -> Result<String, io::Error> {
    let path = format!("{:#?}/{}", env::current_dir()?.display(), target_path).replace("\"", "");
    Ok(path)
}
pub fn get_absolute_output_path(target_path : String) -> Result<String, io::Error> {
    let path = format!("{:#?}/{}_output.wav", env::current_dir()?.display(), target_path).replace("\"", "");
    Ok(path)
}
/*
#[debug_handler]
#[handler(method = "POST", tag = "voice_generation")]
pub async fn transfer_prosody(
    payload: String,
    AuthToken(user_id, role): AuthToken,
    Extension(pool): Extension<Arc<PgPool>>,
    Extension(producer): Extension<Arc<FutureProducer>>,
 ) -> AxumResult<Json<AxumRes<Value>>> {
    role!(TokenRole::User, &role);
    
    println!("payload : {:#?}", payload);
    
    let mut params: ProsodyUploadParam;
    let response = serde_json::from_str(&payload);
    match response {
        Ok(p) => params = p,
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "API params are incorrect!".to_string(),
            });
            return Ok(axum::Json(AxumRes {code: 400, result: ret}));
        }
    };

    if params.template_region.len() == 0 {
        let ret = serde_json::json!({
            "error": "template region is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }
    if params.template_bucket.len() == 0 {
        let ret = serde_json::json!({
            "error": "template bucket is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.template_key.len() == 0 {
        let ret = serde_json::json!({
            "error": "template key is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.reference_region.len() == 0 {
        let ret = serde_json::json!({
            "error": "reference region is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }
    if params.reference_bucket.len() == 0 {
        let ret = serde_json::json!({
            "error": "reference bucket is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.reference_key.len() == 0 {
        let ret = serde_json::json!({
            "error": "reference key is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    let mut template_path = format!("Names/Temp/{}.wav", encode(format!("{}/{}/{}", params.template_region.clone(), params.template_bucket.clone(), params.template_key.clone()))); 
    if !path_exists(&*template_path) {
        template_path = download_template(&params.template_region, &params.template_bucket, &params.template_key).await.unwrap();
        if template_path.len() == 0 {
            let ret = serde_json::json!({
                "error": "template audio is not existed!".to_string(),
            });
            return Ok(axum::Json(AxumRes {code: 400, result: ret}));
        }
        let file_size = std::fs::metadata(&template_path).unwrap().len();
        if file_size == 0 {
            let ret = serde_json::json!({
                "error": "template audio is empty!".to_string(),
            });
            return Ok(axum::Json(AxumRes {code: 400, result: ret}));
        }

        convert_to_wav(&template_path);

        denoise_audio(&template_path);
    }
    println!("template_path : {}", template_path);

    let mut reference_path = format!("Names/Temp/{}.wav", encode(format!("{}/{}/{}", params.reference_region.clone(), params.reference_bucket.clone(), params.reference_key.clone()))); 
    if !path_exists(&*reference_path) {
        reference_path = download_template(&params.reference_region, &params.reference_bucket, &params.reference_key).await.unwrap();
        if reference_path.len() == 0 {
            let ret = serde_json::json!({
                "error": "reference audio is not existed!".to_string(),
            });
            return Ok(axum::Json(AxumRes {code: 400, result: ret}));
        }
        let file_size = std::fs::metadata(&reference_path).unwrap().len();
        if file_size == 0 {
            let ret = serde_json::json!({
                "error": "reference audio is empty!".to_string(),
            });
            return Ok(axum::Json(AxumRes {code: 400, result: ret}));
        }
        
        convert_to_wav(&reference_path);

        denoise_audio(&reference_path);
    }
    println!("reference_path : {}", reference_path);

    let output_path = get_absolute_output_path(template_path.clone()).unwrap();

    let prosody_info : ProsodyInfo = ProsodyInfo{target_path : get_absolute_path(template_path).unwrap(), 
                                            target_transcript: params.template_transcript, 
                                            reference_path : get_absolute_path(reference_path).unwrap(),
                                            reference_transcript: params.reference_transcript, 
                                            output_path: output_path.clone()};

    let params_str = serde_json::to_string(&prosody_info).unwrap(); 
    println!("python params: {}", params_str);
    
    launch_prosody_audio(encode(params_str));

    if !path_exists(&*output_path) {
        let ret = serde_json::json!({
            "error": "failed to transfer prosody!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }
    
    let s3_path = upload_audio_path(&params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &params.actor_id, &output_path).await;
    println!("s3_path: {}", s3_path);
    let _res = std::fs::remove_file(output_path);

    let ret = serde_json::json!({
        "url": s3_path,
    });
    return Ok(axum::Json(AxumRes {code: 200, result: ret}));
} */
/*
pub async fn generate_audio_huggingface(
    payload: String,
    //AuthToken(user_id, role): AuthToken,
    Extension(pool): Extension<Arc<PgPool>>,
    Extension(producer): Extension<Arc<FutureProducer>>,
 ) -> AxumResult<Json<AxumRes<Value>>> {
    //role!(TokenRole::User, &role);
    
    // let voice_gen_queue = TaskQueue::new();
    // voice_gen_queue.enqueue(move || voice_gen_func());

    println!("payload : {:#?}", payload);
    
    let params: BatchUploadParam = serde_json::from_str(&payload).unwrap();
    
    //to broker
    let user_id = String::from("123");
    let message = RdMessage {
        user_id: user_id.to_string(),
        msg_type: RdType::VoiceGen,
        sub_type: RdSubType::Update,
        message: serde_json::json!(user_id.to_string()),
    };
    let msg_str = serde_json::to_string(&message).unwrap();  
    let produce = produce(&msg_str, &producer, "bhuman_channel").await;
    println!("Redpanda -> user id: {}, status: {:#?}", user_id.to_string(), produce);
    
    if params.template_bucket.len() == 0 {
        let ret = serde_json::json!({
            "error": "template bucket is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.template_key.len() == 0 {
        let ret = serde_json::json!({
            "error": "template key is empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    if params.transcripts.len() == 0 {
        let ret = serde_json::json!({
            "error": "transcripts are empty!".to_string(),
        });
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
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
        return Ok(axum::Json(AxumRes {code: 400, result: ret}));
    }

    let mut template_voice_code_path = format!("{}.csv", template_path.clone());
    if !path_exists(&*template_voice_code_path) {
        /*
        audio - processing: denoising, removing silence, 
        */
        // convert any audio format to .wav format using ffmpeg
        convert_to_wav(&template_path);

        // denoise
        denoise_audio(&template_path);
        
        //remove silences
        let _res = remove_silence_audio(&template_path);
        println!("Audio file has been removed silences.");

        template_voice_code_path = generate_voice_code(&template_path);
    }

    let mut output_array = HashMap::new();
    let mut similarity_array = HashMap::new();
    let mut target_array = HashMap::new();
    let mut target_output_array = HashMap::new();
    let similarity_threshold = 0.3;

    // generating yaml and inference part
    for index in 0 .. params.transcripts.len() {
        let user_key = params.transcripts[index].clone();
        
        let path = is_existed_path(&params.output_bucket, &params.template_region, &params.template_bucket, &params.template_key, &user_key).await;
        if path.len() > 0 {
            let mut audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : path, 
                                                                                      similarity: String::from("")};
            output_array.insert(user_key.clone(), audio_similarity_info);

            println!("Thread {} exiting... ", user_key);
            continue;
        } 

        let mut threshold : f64 = 0.0;
    
        let mut query = "SELECT * FROM audio_names WHERE lower(file_path) LIKE ".to_string();
        write!(query, "'%/{}.wav'", user_key.to_lowercase()).unwrap();
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
                let similarity_value = similarity_voice_code(&template_voice_code_path, &item.voice_code).unwrap();
                if similarity_value > threshold {
                    threshold = similarity_value;
                    vst_file_path = item.file_path.clone();
                }
            }
            if threshold > similarity_threshold {
                let target_path = get_absolute_path(vst_file_path.clone()).unwrap();
                println!("similarity_value : {} ---> {}", target_path, threshold);
                target_array.insert(user_key.clone(), target_path);
                similarity_array.insert(user_key.clone(), threshold);
    
                let target_output_path = get_absolute_output_path(vst_file_path.clone()).unwrap();
                target_output_array.insert(user_key.clone(), target_output_path);
            }
        } 
    }
    if target_array.keys().len() > 0 {
        let template_absolute_path = get_absolute_path(template_path).unwrap();
        let huggingface_info : HugginefaceInfo = HugginefaceInfo{target_path : template_absolute_path, 
                                                                    reference_path_list: target_array, 
                                                                    output_list : target_output_array.clone()};
    
        let params_str = serde_json::to_string(&huggingface_info).unwrap();  
        println!("python params: {}", params_str);
    
        launch_huggingface_audio(params_str);
    }

    // uploading generated audio to s3 bucket
    let output_arc_array = Arc::new(Mutex::new(output_array));
    let target_output_arc_array = Arc::new(Mutex::new(target_output_array));
    println!("Started uploading...");
    let mut handles = vec![];
    for index in 0.. params.transcripts.len() {

        let user_key = params.transcripts[index].clone();
        let mut _target_array = target_output_arc_array.lock().unwrap();

        if !_target_array.contains_key(&user_key) {
            let mut _output_array = output_arc_array.lock().unwrap();
            if !_output_array.contains_key(&user_key) {
                let mut audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : String::from(""), 
                                                                                            similarity: String::from("")};
                _output_array.insert(user_key.to_string(), audio_similarity_info);
            }
            continue;
        }

        let target_path = _target_array[&user_key].to_string();
        let audio_similarity = similarity_array[&user_key].to_string();
        let template_region = params.template_region.clone();
        let template_bucket = params.template_bucket.clone();
        let template_key = params.template_key.clone();
        let output_bucket = params.output_bucket.clone();
        let output_array_clone = Arc::clone(&output_arc_array);

        handles.push(tokio::spawn(async move {
            thread::sleep(Duration::from_millis(10));
            println!("Thread {} uploading... ", user_key);

            if path_exists(&*target_path) {
                let s3_path = upload_audio_path(&output_bucket, &template_region, &template_bucket, &template_key, &user_key, &target_path).await;
                let _res = std::fs::remove_file(target_path);
                let mut _output_array = output_array_clone.lock().unwrap();
                let mut audio_similarity_info : AudioSimilarityInfo = AudioSimilarityInfo{file_path : s3_path, 
                                                                                        similarity : audio_similarity};
                _output_array.insert(user_key.clone(), audio_similarity_info);
            }

            //send_status(user_id, format!("{} has been finished.", user_key)).await;
            println!("Thread {} exiting... ", user_key);
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
    
    println!("processing_time => {}", get_system_time() - start_time);

    Ok(axum::Json(AxumRes {code: 200, result: serde_json::json!(output_arc_array)}))
}*/