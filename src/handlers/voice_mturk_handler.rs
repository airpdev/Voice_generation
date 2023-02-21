use axum::{
    extract::{ContentLengthLimit, Multipart, Query,  Extension},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use sqlx::PgPool;
use crate::models::voice_generation::{
    MturkIdInfo,
    MturkAudioInfo,
    MturkProcessInfo,
    SimilarityInfo,
    MturkUploadInfo,
    MturkLoginInfo,
    MturkSignupInfo,
    MturkPaypalInfo,
    MturkPaymentInfo,
    MturkUserInfo,
    MturkFullUserInfo
};
use std::sync::{Arc};
use crate::utils::{response::into_reponse};
use std::fmt::Write;
use serde_json::{Value};
use crate::utils::download_audio::{download_s3_mturk};
use std::{env, ffi::OsStr};
use rusoto_s3::{S3Client, S3, PutObjectRequest};
use rusoto_core::{Region};
use std::str::FromStr;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use uuid::Uuid;
use crate::utils::audio_process::{replace_audio, extract_audio, denoise_audio};
use crate::utils::upload_audio::{clear_whisper_cache};

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref ASSETS_BUCKET: String = ensure_var("ASSETS_BUCKET").unwrap();
    static ref ASSETS_REGION: String = ensure_var("ASSETS_REGION").unwrap();
}

pub const SUCCESS : &str = "0";
pub const PENDING : &str = "2";
pub const FAILED : &str = "1";

#[debug_handler]
pub async fn fetch_mturk_data(payload: String,
                              Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkIdInfo;
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

    if params.mturk_id.len() == 0 {
        let ret = serde_json::json!({
            "error": "mturk_id is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let mut query = "SELECT * FROM audio_mturk WHERE mturk_id = ".to_string();
    write!(query, "'{}'", params.mturk_id).unwrap();
    let str_query: &str = &query[..];
    let audio_records = sqlx::query_as::<_,MturkAudioInfo>(str_query).fetch_all(&*pool).await.unwrap();
    let mut output_array = Vec::new();
    if audio_records.len() > 0 {
        for item in audio_records.iter() {
            output_array.push(item);
        }
    }

    into_reponse(200, serde_json::json!(output_array))
}

#[debug_handler]
pub async fn process_mturk_data(payload: String,
                              Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkProcessInfo;
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
    
    if params.mturk_id.len() == 0 {
        let ret = serde_json::json!({
            "error": "mturk_id is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }
    if params.transcript.len() == 0 {
        let ret = serde_json::json!({
            "error": "transcript is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }
    if params.transcript_id.len() == 0 {
        let ret = serde_json::json!({
            "error": "transcript_id is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }
    if params.file_path.len() == 0 {
        let ret = serde_json::json!({
            "error": "file_path is empty!".to_string(),
        });
        return into_reponse(400, ret);
    }

    println!("initialize status into pending");
    add_mturk_data(&params, pool.clone(), PENDING).await; 
    
    clear_whisper_cache(&params.s3_bucket, &params.s3_key).await; 

    println!("checking transcript");
    let response = get_validation_check(&params).await.to_string();
    println!("response: {}", response);
    if response.len() == 0 {
        add_mturk_data(&params, pool.clone(), FAILED).await;
        let ret = serde_json::json!({
            "error": "media is invalid!".to_string(),
        });
        return into_reponse(400, ret);
    }
    let json_data : SimilarityInfo =  parse_mturk_whisper(response);
    if json_data.transcript.len() == 0 {
        add_mturk_data(&params, pool.clone(), FAILED).await;
        let ret = serde_json::json!({
            "error": "media not supported!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let whisper_transcript = json_data.transcript.replace(&[' ', '(', ')', ',', '\"', '.', ';', ':', '\''][..], "").to_lowercase();
    let trancript = format!("hi{}", params.transcript).to_lowercase();
    println!("{} - {}", whisper_transcript, trancript);
    if whisper_transcript.eq(&trancript) == false {
        println!("checking similarity");
        let response = post_validation_check(&params).await.to_string();
        if response.len() == 0 {
            add_mturk_data(&params, pool.clone(), FAILED).await;
            let ret = serde_json::json!({
                "error": "trancript is not matched!".to_string(),
            });
            return into_reponse(400, ret);
        }
        
        let json_data : SimilarityInfo =  parse_mturk_whisper(response);
        if json_data.transcript.len() == 0 {
            add_mturk_data(&params, pool.clone(), FAILED).await;
            let ret = serde_json::json!({
                "error": "media not supported!".to_string(),
            });
            return into_reponse(400, ret);
        }

        let mut flag = true;
        for item in json_data.alignments.iter() {
            if item.s < 0.5 {
                flag = false;
            }
        }
    
        if flag == false {
            println!("status into failed");
            add_mturk_data(&params, pool.clone(), FAILED).await;
            let ret = serde_json::json!({
                "error": "trancript is not matched!".to_string(),
            });
            return into_reponse(400, ret);
        }
    } else {
        println!("same transcript!");
    }
    
    let mut response = false;
    match download_s3_mturk(&params, pool.clone()).await {
        Ok(_flag) => {
            response = true;
            println!("status into success");
        }
        Err(_e) => {
            println!("status into failed");
        }
    }
    if response == true {
        add_mturk_data(&params, pool.clone(), SUCCESS).await;
        let ret = serde_json::json!({
            "success": "succeed to process!".to_string(),
        });
        return into_reponse(200, ret);
    } else {
        add_mturk_data(&params, pool.clone(), FAILED).await;
        let ret = serde_json::json!({
            "error": "downloading is failed.".to_string(),
        });
        return into_reponse(400, ret);
    }

}
pub fn parse_mturk_whisper(response: String) -> SimilarityInfo {
    let response_list: Vec<&str> = response.split("\n").collect();
    let similarity_data = response_list[response_list.len() - 2];
    let similarity_data = format!(r#"{}"#, similarity_data);
    let json_array: Vec<Value> = serde_json::from_str(&similarity_data).unwrap();
    let similarity_data = serde_json::to_string(&json_array[json_array.len() - 1]).unwrap();

    let json_data : SimilarityInfo;
    let response = serde_json::from_str(&similarity_data);
    match response {
        Ok(p) => json_data = p,
        Err(_e) => {
            json_data = SimilarityInfo{transcript: String::from(""), whisper_transcript:  String::from(""), alignments: Vec::new()};
        }
    };

    json_data
}
pub async fn get_validation_check(params: &MturkProcessInfo) -> String {
	let url = format!("https://whisper.dev.bhuman.ai/{}/{}", params.s3_bucket, format!("{}.wav", params.s3_key));
    let client = reqwest::Client::new();
	let response = client.get(url)
						.send()
						.await
						.unwrap();
    match response.text().await {
        Ok(result) => {
            println!("result: {}", result);
            result
        }
        Err(_e) => {
            println!("result: {}", _e.to_string());
            "".to_string()
        }
    }
}
pub async fn post_validation_check(params: &MturkProcessInfo) -> String {
	let url = format!("https://whisper.dev.bhuman.ai/{}/{}", params.s3_bucket, format!("{}.wav", params.s3_key));
    let client = reqwest::Client::new();
	let response = client.post(url)
                        .body(format!("Hi, {}", params.transcript))
						.send()
						.await
						.unwrap();
    match response.text().await {
        Ok(result) => {
            println!("result: {}", result);
            result
        }
        Err(_e) => {
            println!("result: {}", _e.to_string());
            "".to_string()
        }
    }
}
pub async fn add_mturk_data(params: &MturkProcessInfo, pool : Arc<PgPool>, status: &str) {
    let i = &*pool;
    let recs = sqlx::query_as!(
        MturkAudioInfo,
        r#"SELECT * FROM audio_mturk WHERE mturk_id = $1 And transcript_id = $2"#,
        params.mturk_id,
        params.transcript_id
    )
    .fetch_one(&*i)
    .await;
    if recs.is_ok() {
        sqlx::query_as!(
            MturkAudioInfo,
            r#"UPDATE audio_mturk SET file_path = $1, status = $2, duration = $3 WHERE mturk_id = $4 And transcript_id = $5 RETURNING *"#,
            params.file_path,
            status,
            params.duration,
            params.mturk_id,
            params.transcript_id
        )
        .fetch_one(&*i)
        .await
        .unwrap();
        println!("Audio file has been updated in database.");
    } else {
        sqlx::query_as!(
            MturkAudioInfo,
            r#"INSERT INTO audio_mturk (mturk_id, transcript, transcript_id, file_path, status, duration) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"#,
            params.mturk_id,
            params.transcript,
            params.transcript_id,
            params.file_path,
            status,
            params.duration
        )
        .fetch_one(&*i)
        .await
        .unwrap();
        println!("Audio file has been inserted in database.");
    }
}

#[debug_handler]
pub async fn upload_mturk_s3(
    params: Query<MturkUploadInfo>,
    ContentLengthLimit(mut multipart): ContentLengthLimit<Multipart, { 2500 * 1024 * 1024 }>
) -> impl IntoResponse {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let video_path = format!("Names/Temp/{}.webm", Uuid::new_v4()); 
        let bytes = field.bytes().await.unwrap().to_vec().clone();  
        let mut reader: &[u8] = &bytes;
        // Create a file 
        let mut out = File::create(&video_path).expect("failed to create file");
        //Copy data to the file
        io::copy(&mut reader, &mut out).expect("failed to copy content");
        let audio_path = extract_audio(&video_path);
        denoise_audio(&audio_path);
        // replace processed audio with audio in video
        replace_audio(&video_path, &audio_path);

        let video_file = File::open(&video_path);
        let mut video_file = match video_file {
            Ok(video_file) => video_file,
            Err(error) => panic!("Problem opening the file: {:?}", error),
        };
        let mut buffer = Vec::new();
        video_file.read_to_end(&mut buffer).unwrap();

        let audio_file = File::open(&audio_path);
        let mut audio_file = match audio_file {
            Ok(audio_file) => audio_file,
            Err(error) => panic!("Problem opening the file: {:?}", error),
        };
        let mut audio_buffer = Vec::new();
        audio_file.read_to_end(&mut audio_buffer).unwrap();

        let _res = std::fs::remove_file(&video_path);
        let _res = std::fs::remove_file(&audio_path);

        // upload to s3 bucket
        let s3 = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());

        let s3_name = format!("{}.{}", params.transcript, "webm");
        let mut s3_path = format!("Names/Mturk/{}/{}", params.mturk_id, s3_name);
        let result = s3.put_object(PutObjectRequest {
                                    key: s3_path.clone(),
                                    content_type: Some("*".to_string()),
                                    content_disposition: Some(format!("inline; filename={}", s3_name)),
                                    content_length: Some(buffer.len() as i64),
                                    body: Some(buffer.into()),
                                    bucket: ASSETS_BUCKET.to_string(),
                                    acl: Some("public-read".to_string()),
                                    ..Default::default()
                                    }).await;
        match result {
            Ok(success) => { 
                println!("Success: {:?}", success);
                
            },
            Err(error) => {
                println!("Failure: {:?}", error);
                s3_path = String::from("");
            }
        }

        // uploading audio
        let s3_audio_name = format!("{}.{}", s3_name, "wav");
        let mut s3_audio_path = format!("Names/Mturk/{}/{}", params.mturk_id, s3_audio_name);
        let result = s3.put_object(PutObjectRequest {
                                    key: s3_audio_path.clone(),
                                    content_type: Some("*".to_string()),
                                    content_disposition: Some(format!("inline; filename={}", s3_audio_name)),
                                    content_length: Some(audio_buffer.len() as i64),
                                    body: Some(audio_buffer.into()),
                                    bucket: ASSETS_BUCKET.to_string(),
                                    acl: Some("public-read".to_string()),
                                    ..Default::default()
                                    }).await;
        match result {
            Ok(success) => { 
                println!("Success: {:?}", success);
                
            },
            Err(error) => {
                println!("Failure: {:?}", error);
                s3_audio_path = String::from("");
            }
        }

        if s3_path.len() > 0 {
            let ret = serde_json::json!({
                "transcript_id" : params.transcript_id,
                "url": format!("https://{}.s3.amazonaws.com/{}", ASSETS_BUCKET.to_string(), s3_path),
            });
            return into_reponse(200, ret);
        } else {
            let ret = serde_json::json!({
                "transcript_id" : params.transcript_id,
                "url": String::from(""),
            });
            return into_reponse(400, ret);
        }
    } else {
        let ret = serde_json::json!({
            "transcript_id" : params.transcript_id,
            "url": String::from(""),
        });
        return into_reponse(400, ret);
    }
}

#[debug_handler]
pub async fn login_mturk(payload: String,
                         Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkLoginInfo;
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

    let recs = sqlx::query_as!(
        MturkUserInfo,
        r#"SELECT * FROM audio_mturk_users WHERE mturk_id = $1 And password = $2"#,
        params.mturk_id,
        params.password
    )
    .fetch_one(&*pool)
    .await;
    if recs.is_ok() {
        let ret = serde_json::json!({
            "status": "sccuess".to_string(),
        });
        return into_reponse(200, ret);
    } 
    let ret = serde_json::json!({
        "error": "mturk id or password is invalid!".to_string(),
    });
    return into_reponse(400, ret);
}

#[debug_handler]
pub async fn signup_mturk(payload: String,
                         Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkSignupInfo;
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

    let recs = sqlx::query_as!(
        MturkUserInfo,
        r#"SELECT * FROM audio_mturk_users WHERE mturk_id = $1"#,
        params.mturk_id
    )
    .fetch_one(&*pool)
    .await;

    if recs.is_ok() {
        let ret = serde_json::json!({
            "error": "mturk id is already existed!".to_string(),
        });
        return into_reponse(400, ret);
    } 

    let recs = sqlx::query_as!(
        MturkUserInfo,
        r#"INSERT INTO audio_mturk_users (mturk_id, password, paypal, total_payment) VALUES ($1, $2, $3, $4) RETURNING *"#,
        params.mturk_id,
        params.password,
        params.paypal,
        0
    )
    .fetch_one(&*pool)
    .await;

    if recs.is_ok() {
        let ret = serde_json::json!({
            "status": "success".to_string(),
        });
        return into_reponse(200, ret);
    }

    let ret = serde_json::json!({
        "error": "failed to signup.".to_string(),
    });
    return into_reponse(400, ret);
}

#[debug_handler]
pub async fn get_mturk_user(payload: String,
                         Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkIdInfo;
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

    let mut query = "SELECT * FROM audio_mturk_users WHERE mturk_id = ".to_string();
    write!(query, "'{}'", params.mturk_id).unwrap();
    let str_query: &str = &query[..];
    let user_records = sqlx::query_as::<_,MturkUserInfo>(str_query).fetch_all(&*pool).await.unwrap();
    let mut output_array = Vec::new();
    if user_records.len() > 0 {
        for item in user_records.iter() {
            output_array.push(item);
        }
        return into_reponse(200, serde_json::json!(output_array[0]))
    }

    let ret = serde_json::json!({
        "error": "failed to get mturk user info.".to_string(),
    });
    return into_reponse(400, ret);

    
}

#[debug_handler]
pub async fn set_paypal(payload: String,
                         Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkPaypalInfo;
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

    let recs = sqlx::query_as!(
        MturkUserInfo,
        r#"SELECT * FROM audio_mturk_users WHERE mturk_id = $1"#,
        params.mturk_id
    )
    .fetch_one(&*pool)
    .await;
    if recs.is_ok() {
        sqlx::query_as!(
            MturkUserInfo,
            r#"UPDATE audio_mturk_users SET paypal = $1 WHERE mturk_id = $2 RETURNING *"#,
            params.paypal,
            params.mturk_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        let ret = serde_json::json!({
            "status": "success".to_string(),
        });
        return into_reponse(200, ret);
    } 
    
    
    let ret = serde_json::json!({
        "error": "mturk user isn't existed!.".to_string(),
    });
    return into_reponse(400, ret);
}

#[debug_handler]
pub async fn set_payment(payload: String,
                         Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: MturkPaymentInfo;
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
    
    let user_info = sqlx::query_as!(
        MturkUserInfo,
        r#"SELECT * FROM audio_mturk_users WHERE mturk_id = $1"#,
        params.mturk_id
    )
    .fetch_one(&*pool)
    .await;

    match user_info {
        Ok(user_info) => {
            sqlx::query_as!(
                MturkUserInfo,
                r#"UPDATE audio_mturk_users SET total_payment = $1 WHERE mturk_id = $2 RETURNING *"#,
                user_info.total_payment + params.payment_amount,
                params.mturk_id
            )
            .fetch_one(&*pool)
            .await
            .unwrap();
    
            let ret = serde_json::json!({
                "status": "success".to_string(),
            });
            return into_reponse(200, ret);
        }, 
        Err(_e) => {
            let ret = serde_json::json!({
                "error": "mturk user isn't existed!.".to_string(),
            });
            return into_reponse(400, ret);
        }
    };
}

#[debug_handler]
pub async fn fetch_users_info(payload: String,
                         Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    let mut query = "select a.mturk_id as mturk_id, a.password as password, count(b.id) as total_records, a.paypal as paypal, a.total_payment as total_payment from audio_mturk_users a left outer join (select * from audio_mturk where status = '0') b on a.mturk_id = b.mturk_id group by a.mturk_id, a.password, a.paypal, a.total_payment".to_string();
    let str_query: &str = &query[..];
    let user_records = sqlx::query_as::<_,MturkFullUserInfo>(str_query).fetch_all(&*pool).await.unwrap();
    let mut output_array = Vec::new();
    if user_records.len() > 0 {
        for item in user_records.iter() {
            output_array.push(item);
        }
        
    } 
    return into_reponse(200, serde_json::json!(output_array))
}