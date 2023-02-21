extern crate hound;
extern crate base64;
use axum::{
    extract::{Extension, /*TypedHeader*/},
    //headers::{authorization::Bearer, Authorization},
    response::IntoResponse,
};
use sqlx::PgPool;
use std::fmt::Write;
use crate::utils::{/*jwt::jwt_auth, */ response::into_reponse};
use base64::{encode};
use std::{sync::Arc, str::FromStr};
use rusoto_credential::{EnvironmentProvider, ProvideAwsCredentials};
use rusoto_core::{Region};
use rusoto_s3::{GetObjectRequest, ListObjectsV2Request, S3Client, S3};
use tokio::{fs::File, io};
use std::fs;
use std::fmt;

use crate::utils::audio_process::{write_csv, read_csv, similarity_voice_code, denoise_audio, generate_voice_code, generate_voice_code_separate, remove_silence_audio, is_check_wav, convert_to_wav};
use crate::models::voice_generation::{
    AudioInfo, VoiceCodeParam, AudioTrashInfo, MturkProcessInfo
};
use std::{env, ffi::OsStr};

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref ASSETS_BUCKET: String = ensure_var("ASSETS_BUCKET").unwrap();
    static ref ASSETS_REGION: String = ensure_var("ASSETS_REGION").unwrap();
}
pub async fn download_template(_region: &String, _bucket: &String, _key : &String) -> Result<String> {
    let template_path = format!("Names/Temp/{}.wav", encode(format!("{}/{}/{}", _region, _bucket, _key))); 

    let s3_client = S3Client::new(Region::from_str(_region).unwrap());
    // download audio file from key
    let response = s3_client.get_object(GetObjectRequest {
                                key: _key.to_string(),
                                bucket: _bucket.to_string(),
                                ..Default::default()
                                })
                            .await;
                                
    match response {
        Ok(mut obj) => {
            println!("Getting object..."); 
            let body = obj.body.take().expect("The object has no body");
            let mut body = body.into_async_read();
        
            let mut file = File::create(template_path.clone()).await?;
            io::copy(&mut body, &mut file).await?;

            Ok(template_path)
        },
        Err(_e) => Ok("".to_string())
    }
}
pub async fn download_template_with_path(region_val: &String, bucket_val: &String, key_val : &String, path : &String) -> Result<String> {
    let region : Region = match Region::from_str(region_val) {
        Ok(value) => value,
        Err(_e) => return Err(Box::new(_e))
    };
    let s3_client = S3Client::new(region);
    // download audio file from key
    let response = s3_client.get_object(GetObjectRequest {
                                key: key_val.to_string(),
                                bucket: bucket_val.to_string(),
                                ..Default::default()
                                })
                            .await;
                                
    match response {
        Ok(mut obj) => {
            println!("Getting object..."); 
            let body = obj.body.take().expect("The object has no body");
            let mut body = body.into_async_read();

            let mut file = File::create(path).await?;
            io::copy(&mut body, &mut file).await?;

            Ok(path.to_string())
        },
        Err(_e) => Err(Box::new(_e))
    }
}
pub async fn voice_code(
    payload: String,
    //cookies: TypedHeader<Authorization<Bearer>>,
    Extension(pool): Extension<Arc<PgPool>>
)-> impl IntoResponse {
        
    let params: VoiceCodeParam = serde_json::from_str(&payload).unwrap();

    let i = &*pool;

    let mut query = "SELECT * FROM audio_names WHERE file_path = ".to_string();
    write!(query, "'{}'", params.audio_key).unwrap();
    let str_query: &str = &query[..];

    let mut voice_code_path = String::from("");

    let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*i).await.unwrap();
    if name_records.len() > 0 {
        for item in name_records.iter() {
            voice_code_path = format!("{}.csv", item.file_path.clone());
            if path_exists(&*voice_code_path){
                continue;
            }
            let voice_code_path = generate_voice_code_separate(&item.file_path);
            sqlx::query_as!(
                AudioInfo,
                r#"UPDATE audio_names SET voice_code = $1 WHERE file_path = $2 RETURNING *"#,
                voice_code_path.clone(),
                item.file_path.clone()
            )
            .fetch_one(&*i)
            .await
            .unwrap();
            println!("{} -> {} ", item.file_path, voice_code_path);
        }
    } else {
        let ret = serde_json::json!({
            "error": "audio key is not existed!".to_string(),
        });
        return into_reponse(400, ret);
    }

    let ret = serde_json::json!({
        "success": voice_code_path,
    });
    return into_reponse(200, ret);
}
pub async fn generate_audio_code(pool : Arc<sqlx::Pool<sqlx::Postgres>>) -> Result<()> {
    let i = &*pool;
    let query = "SELECT * FROM audio_names ORDER BY file_path ASC".to_string();
    let str_query: &str = &query[..];

    let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*i).await.unwrap();
    let mut count : u64 = 0;
    if name_records.len() > 0 {
        for item in name_records.iter() {
            count += 1;
            let voice_code_path = format!("{}.csv", item.file_path.clone());
            if path_exists(&*voice_code_path){
                println!("{} : {} -> {} ", count, item.file_path, voice_code_path);
                continue;
            }
            let voice_code_path = generate_voice_code_separate(&item.file_path);
            sqlx::query_as!(
                AudioInfo,
                r#"UPDATE audio_names SET voice_code = $1 WHERE file_path = $2 RETURNING *"#,
                voice_code_path.clone(),
                item.file_path.clone()
            )
            .fetch_one(&*i)
            .await
            .unwrap();
            println!("{} : {} -> {} ", count, item.file_path, voice_code_path);
        }
    }

    Ok(())
}
pub async fn remove_folder_code(pool : Arc<sqlx::Pool<sqlx::Postgres>>) -> Result<()> {
    let i = &*pool;
    let query = "SELECT * FROM audio_names ORDER BY file_path ASC".to_string();
    let str_query: &str = &query[..];

    let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*i).await.unwrap();
    if name_records.len() > 0 {
        for item in name_records.iter() {
            let path_splits = item.file_path.split("/");                                 // Names/foler/123.wav
	        let path_array: Vec<&str> = path_splits.collect(); 
            let folder_voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], "folder");
            if !path_exists(&*folder_voice_code_path.clone()){
                continue;
            }
            let _res = fs::remove_file(folder_voice_code_path.clone());
            println!("{} has been removed.", folder_voice_code_path); 
        }
    }
    println!("finished!"); 
    Ok(())
}

pub async fn generate_folder_code(pool : Arc<sqlx::Pool<sqlx::Postgres>>) -> Result<()> {
    let i = &*pool;
    let query = "SELECT * FROM audio_names ORDER BY file_path ASC".to_string();
    let str_query: &str = &query[..];

    let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*i).await.unwrap();
    if name_records.len() > 0 {
        for item in name_records.iter() {
            let path_splits = item.file_path.split("/");                                 // Names/foler/123.wav
	        let path_array: Vec<&str> = path_splits.collect(); 
            let folder_path = format!("{}/{}/", path_array[0], path_array[1]);
            let folder_voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], "folder");

            if path_exists(&*folder_voice_code_path){
                continue;
            }
            let mut folder_query = "SELECT * FROM audio_names WHERE file_path LIKE ".to_string();
            write!(folder_query, "'{}%'", folder_path).unwrap();
            let folder_str_query: &str = &folder_query[..];
            let folder_records = sqlx::query_as::<_,AudioInfo>(folder_str_query).fetch_all(&*i).await.unwrap();
            if folder_records.len() > 0 {
                let mut folder_code_vec = Vec::new();
                let mut count = 0;
                for folder_item in folder_records.iter() {
                    println!("{}", folder_item.voice_code.clone()); 
                    if !path_exists(&*folder_item.voice_code.clone()){
                        continue;
                    }
                    count += 1;
                    let folder_item_vec = read_csv(&folder_item.voice_code, false);
                    if folder_code_vec.len() == 0 {
                        folder_code_vec = folder_item_vec.clone();
                        continue;
                    }
                    for i in 0 .. folder_code_vec.len() {
                        folder_code_vec[i] += folder_item_vec[i];
                    }
                }
                for i in 0 .. folder_code_vec.len() {
                    folder_code_vec[i] = folder_code_vec[i] / count as f64;
                }
                write_csv(&folder_voice_code_path, folder_code_vec);

                println!("{} / {}", count, folder_records.len()); 
                println!("{} has been generated.", folder_voice_code_path); 
            }
        }
    }
    println!("generating folder voice code has been finished!"); 
    Ok(())
}
pub async fn clean_audio_db(pool : Arc<sqlx::Pool<sqlx::Postgres>>) {
    let threshold = 0.6;

    let i = &*pool;
    let query = "SELECT * FROM audio_names ORDER BY file_path ASC".to_string();
    let str_query: &str = &query[..];

    let name_records = sqlx::query_as::<_,AudioInfo>(str_query).fetch_all(&*i).await.unwrap();
    if name_records.len() > 0 {
        let mut count : u64 = 0;
        for item in name_records.iter() {
            count += 1;
            let path_splits = item.file_path.split("/");                                 // Names/foler/123.wav
	        let path_array: Vec<&str> = path_splits.collect(); 
            let folder_voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], "folder");
            if !path_exists(&*folder_voice_code_path){
                continue;
            }

            let mut similarity_value : f64 = 0.0;
            if path_exists(&*item.voice_code.clone()){
                similarity_value = similarity_voice_code(&folder_voice_code_path, &item.voice_code).unwrap();
                println!("{}:{} -> similarity : {}", count, item.voice_code.clone(), similarity_value); 
            } 


            if similarity_value < threshold {
                // sqlx::query!(
                //         r#"DELETE FROM audio_names WHERE file_path = $1"#,
                //         item.file_path.clone()
                //     )
                //     .execute(&*i)
                //     .await
                //     .unwrap();

                let recs = sqlx::query_as!(
                                AudioTrashInfo,
                                r#"SELECT * FROM audio_trash WHERE file_path = $1"#,
                                item.file_path.clone(),
                            )
                            .fetch_one(&*i)
                            .await;
                if !recs.is_ok() {
                    sqlx::query_as!(
                        AudioTrashInfo,
                        r#"INSERT INTO audio_trash (file_path, voice_code, similarity) VALUES ($1, $2, $3) RETURNING *"#,
                        item.file_path.clone(),
                        item.voice_code.clone(),
                        similarity_value.to_string()
                    )
                    .fetch_one(&*i)
                    .await
                    .unwrap();
                }
            }
        }
    }
}
pub async fn get_removed_names(
    Extension(pool): Extension<Arc<PgPool>>
) -> impl IntoResponse {
    let i = &*pool;

    let query = "SELECT * FROM audio_trash".to_string();
    let str_query: &str = &query[..];

    let mut output_array = Vec::new();

    let name_records = sqlx::query_as::<_,AudioTrashInfo>(str_query).fetch_all(&*i).await.unwrap();
    if name_records.len() > 0 {
        for item in name_records.iter() {
            output_array.push(item.file_path.clone());
        }
    }

    into_reponse(200, serde_json::json!(output_array))
}
pub fn detect_one_audio(key_value: &String) {
    let path = format!("{}_original", key_value);
    let _res = fs::copy(key_value, path);

    // convert any audio format to .wav format using ffmpeg
    convert_to_wav(&key_value);

    let path = format!("{}_converted", key_value);
    let _res = fs::copy(key_value, path);

    /*
    audio - processing: denoising
    */
    // denoise
    denoise_audio(&key_value);

    let path = format!("{}_denoised", key_value);
    let _res = fs::copy(key_value, path);

    let _res = remove_silence_audio(&key_value);
}
pub async fn detect_special_audios() -> Result<()> {
    let names_folder = String::from("Names/BHUMAN_NAME_RECORDINGS/");
    EnvironmentProvider::default().credentials().await.unwrap();
    
    let bucket_name = String::from("assets-bhuman-new"); //S3_BUCKET.to_string();
    
    let mut list_obj_req = ListObjectsV2Request {
        bucket: bucket_name.clone(),
        prefix: Some(names_folder),
        ..Default::default()
    };
    let mut count : u64 = 0;
    loop {
        let s3 = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());    
        let result = s3.list_objects_v2(list_obj_req.clone()).await.unwrap();

        for object in result.contents.as_ref().unwrap() {
            let s3_client = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());
            let mut key_value = match object.key {
                None => "none",
                Some(ref  key) => key,
            };  

            let key_value_modified = format!("Pauses/{}", key_value);
            
            if path_exists(&*key_value_modified) {
                count += 1;
                println!("");  
                println!("{} : {}", count, key_value); 
                continue;
            }

            // download audio file from key
            let mut obj = s3_client.get_object(GetObjectRequest {
                key: String::from(key_value),
                bucket: bucket_name.clone(),
                ..Default::default()
                })
                .await
                .map_err(anyhow_reject)
                .unwrap();

            if obj.content_type == Some(String::from("audio/wav")) {
                count += 1;
                println!("=");  
                println!("{} : {}", count, key_value); 
                key_value = &*key_value_modified;
                
                let path_splits = key_value.split("/");                                 // Names/foler/123.wav
                let path_array: Vec<&str> = path_splits.collect();                      // [Names, folder, 123.wav]

                let mut dir_path = String::from("");
                for index in 0..(path_array.len() - 1) {
                    if index == 0 {
                        dir_path = format!("{}", path_array[index]);
                    } else {
                        dir_path = format!("{}/{}", dir_path, path_array[index]);
                    }

                    if !path_exists(&*dir_path) {
                        fs::create_dir(&*dir_path)?;
                    }

                }

                println!("Getting object..."); 
                let body = obj.body.take().expect("The object has no body");
                let mut body = body.into_async_read();
                let mut file = File::create(key_value).await?;
                io::copy(&mut body, &mut file).await?;

                let is_valid : bool;
                match is_check_wav(key_value) {
                    Ok(flag) => is_valid = flag,
                    Err(_) => is_valid = false,
                }
            
                if !is_valid {
                    println!("Invalid audio..."); 
                    continue;
                }

                detect_one_audio(&key_value.to_string());

                /*let silences : Vec<Silence> = get_silences_audio(key_value.to_string());
                let mut output_array = Vec::new();
            
                for i in 0..silences.len() {
                    let start_time = (silences[i].start_time * 1000.0) as i64;
                    let end_time = (silences[i].end_time * 1000.0) as i64;
                    let mut array = Vec::new();
                    array.push(start_time);
                    array.push(end_time);
                    output_array.push(array);
                }
                println!("pauses: {:#?}", output_array);*/
            }
        }
        if result.is_truncated  == Some(false) {
            break;
        }
        println!("");  
        println!("Please wait...");  
        list_obj_req.continuation_token = result.next_continuation_token;
    }
    println!("Total counts: {}", count);  
    Ok(())
}

pub async fn download_s3_mturk(mturk_info : &MturkProcessInfo, pool :Arc<PgPool>) -> Result<(), Error> {
    EnvironmentProvider::default().credentials().await.unwrap();
    let s3_client = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());
    // download audio file from key
    let key_value = format!("{}.wav", mturk_info.s3_key.to_string());
    let mut obj = s3_client.get_object(GetObjectRequest {
        key: key_value.to_string(),
        bucket: mturk_info.s3_bucket.to_string(),
        ..Default::default()
        })
        .await
        .map_err(anyhow_reject)
        .unwrap();

    let path_splits = key_value.split("/");                                 // Names/foler/123.wav
    let path_array: Vec<&str> = path_splits.collect();                      // [Names, folder, 123.wav]

    let mut dir_path = String::from("");
    for index in 0..(path_array.len() - 1) {
        if index == 0 {
            dir_path = format!("{}", path_array[index]);
        } else {
            dir_path = format!("{}/{}", dir_path, path_array[index]);
        }

        if !path_exists(&*dir_path) {
            fs::create_dir(&*dir_path)?;
        }

    }  
    
    println!("Getting object..."); 
    let body = obj.body.take().expect("The object has no body");
    let mut body = body.into_async_read();
    let mut file = File::create(key_value.to_string()).await?;
    io::copy(&mut body, &mut file).await?;

    let file_size = std::fs::metadata(&key_value).unwrap().len();
    if file_size == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    // convert any audio format to .wav format using ffmpeg
    convert_to_wav(&key_value.to_string());

    let _is_valid : bool;
    match is_check_wav(&*key_value) {
        Ok(flag) => _is_valid = flag,
        Err(e) => return Err(std::io::Error::last_os_error().into()),
    }

    /*
    audio - processing: denoising, removing silence, 
    */
    // it has already denoised when uploading to s3 bucket.
    //denoise_audio(&key_value.to_string());
    
    //remove silences
    let _res = remove_silence_audio(&key_value.to_string());
    println!("Audio file has been removed silences.");  

    let voice_code_path = generate_voice_code(&key_value.to_string());

    let key_value_modified = format!("{}", key_value);
    let i = &*pool;
    let recs = sqlx::query_as!(
        AudioInfo,
        r#"SELECT * FROM audio_names WHERE file_path = $1"#,
        key_value_modified,
    )
    .fetch_one(&*i)
    .await;

    if recs.is_ok() {
        sqlx::query_as!(
            AudioInfo,
            r#"UPDATE audio_names SET voice_code = $1 WHERE file_path = $2 RETURNING *"#,
            voice_code_path,
            key_value
        )
        .fetch_one(&*i)
        .await
        .unwrap();
        println!("Audio file has been updated in database.");
    } else {
        sqlx::query_as!(
            AudioInfo,
            r#"INSERT INTO audio_names (file_path, voice_code) VALUES ($1, $2) RETURNING *"#,
            key_value,
            voice_code_path
        )
        .fetch_one(&*i)
        .await
        .unwrap();
        println!("Audio file has been inserted in database.");
    }

    Ok(())
}
pub async fn download_s3_names(pool : Arc<sqlx::Pool<sqlx::Postgres>>) -> Result<()> {
    let names_folder = String::from("Names/");
    EnvironmentProvider::default().credentials().await.unwrap();
    
    let bucket_name = ASSETS_BUCKET.to_string();
    
    let mut list_obj_req = ListObjectsV2Request {
        bucket: bucket_name.clone(),
        prefix: Some(names_folder),
        ..Default::default()
    };
    let mut count : u64 = 0;
    loop {
        let s3 = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());    
        let result = s3.list_objects_v2(list_obj_req.clone()).await.unwrap();
        for object in result.contents.as_ref().unwrap() {
            let s3_client = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());

            let mut key_value = match object.key {
                None => "none",
                Some(ref  key) => key,
            };  
            let key_value_modified = format!("{}", key_value);
            let i = &*pool;
            let recs = sqlx::query_as!(
                AudioInfo,
                r#"SELECT * FROM audio_names WHERE file_path = $1"#,
                key_value_modified,
            )
            .fetch_one(&*i)
            .await;

            if path_exists(&*key_value_modified) && recs.is_ok() {
                count += 1;
                println!("");  
                println!("{} : {}", count, key_value); 
                continue;
            }
            // download audio file from key
            let mut obj = s3_client.get_object(GetObjectRequest {
                                    key: String::from(key_value),
                                    bucket: bucket_name.clone(),
                                    ..Default::default()
                                    })
                        .await
                        .map_err(anyhow_reject)
                        .unwrap();

            if obj.content_type == Some(String::from("audio/wav")) {
                count += 1;
                println!("=");  
                println!("{} : {}", count, key_value); 
                key_value = &*key_value_modified;
                
                let path_splits = key_value.split("/");                                 // Names/foler/123.wav
                let path_array: Vec<&str> = path_splits.collect();                      // [Names, folder, 123.wav]

                let mut dir_path = String::from("");
                for index in 0..(path_array.len() - 1) {
                    if index == 0 {
                        dir_path = format!("{}", path_array[index]);
                    } else {
                        dir_path = format!("{}/{}", dir_path, path_array[index]);
                    }

                    if !path_exists(&*dir_path) {
                        fs::create_dir(&*dir_path)?;
                    }

                }

                println!("Getting object..."); 
                let body = obj.body.take().expect("The object has no body");
                let mut body = body.into_async_read();
                let mut file = File::create(key_value).await?;
                io::copy(&mut body, &mut file).await?;

                // convert any audio format to .wav format using ffmpeg
                convert_to_wav(&key_value.to_string());

                let is_valid : bool;
                match is_check_wav(key_value) {
                    Ok(flag) => is_valid = flag,
                    Err(_) => is_valid = false,
                }

                if !is_valid {
                    println!("Invalid audio..."); 
                    continue;
                }

                /*
                audio - processing: denoising, removing silence, 
                */
                // denoise
                denoise_audio(&key_value.to_string());
                
                //remove silences
                let _res = remove_silence_audio(&key_value.to_string());
                println!("Audio file has been removed silences.");
                /*
                println!("VST started =====================");
                let mut child = Command::new("sh")
                .arg("launch_vst.sh")
                .arg("name.wav")
                .arg("template.wav")
                .arg("test1_test2")
                .arg("custom.yaml")
                .spawn()
                .expect("Failed to VST process");
                child.wait().expect("failed to wait on processing VST");
                println!("VST finished =====================");
                */
                let voice_code_path = generate_voice_code(&key_value.to_string());

                if recs.is_ok() {
                    sqlx::query_as!(
                        AudioInfo,
                        r#"UPDATE audio_names SET voice_code = $1 WHERE file_path = $2 RETURNING *"#,
                        voice_code_path,
                        key_value
                    )
                    .fetch_one(&*i)
                    .await
                    .unwrap();
                    println!("Audio file has been updated in database.");
                } else {
                    sqlx::query_as!(
                        AudioInfo,
                        r#"INSERT INTO audio_names (file_path, voice_code) VALUES ($1, $2) RETURNING *"#,
                        key_value,
                        voice_code_path
                    )
                    .fetch_one(&*i)
                    .await
                    .unwrap();
                    println!("Audio file has been inserted in database.");
                }
            }
            // let ten_millis = time::Duration::from_millis(10);
            // let now = time::Instant::now();
            // thread::sleep(ten_millis);
            // assert!(now.elapsed() >= ten_millis);
        }
        if result.is_truncated  == Some(false) {
            break;
        }
        println!("");  
        println!("Please wait...");  
        list_obj_req.continuation_token = result.next_continuation_token;
    } 
    println!("Total counts: {}", count);  

    Ok(())
}

fn anyhow_reject<E: std::error::Error + Sync + Send + 'static>(err: E) -> anyhow::Error {
    anyhow::Error::new(err)
}

type Error = Box<dyn std::error::Error>;
type Result<T, E = Error> = std::result::Result<T, E>;
pub fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

