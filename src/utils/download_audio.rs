extern crate hound;
extern crate base64;
use base64::{encode};
use std::{sync::Arc, str::FromStr};
use rusoto_credential::{EnvironmentProvider, ProvideAwsCredentials};
use rusoto_core::{Region};
use rusoto_s3::{GetObjectRequest, ListObjectsV2Request, S3Client, S3};
use tokio::{fs::File, io};
use std::fs;
use crate::utils::audio_process::denoise_audio;
use crate::utils::audio_process::generate_voice_code;
use crate::utils::audio_process::generate_voice_code_separate;
use crate::utils::audio_process::remove_silence_audio;
use crate::utils::audio_process::is_check_wav;
use crate::utils::audio_process::convert_to_wav;
use crate::models::voice_generation::{
    AudioInfo,
};
use std::{env, ffi::OsStr};

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref S3_BUCKET: String = ensure_var("S3_BUCKET").unwrap();
    static ref S3_REGION: String = ensure_var("S3_REGION").unwrap();
}
pub async fn download_template(template_bucket: String, template_key : String) -> Result<String> {
    let template_path = format!("Names/Temp/{}.wav", encode(template_key.clone())); 

    let s3_client = S3Client::new(Region::from_str(&S3_REGION).unwrap());
    // download audio file from key
    let response = s3_client.get_object(GetObjectRequest {
                                key: template_key,
                                bucket: template_bucket,
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
pub async fn clean_audio_db(pool : Arc<sqlx::Pool<sqlx::Postgres>>) -> Result<()> {
    let i = &*pool;
    let query = "SELECT * FROM audio_names".to_string();
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
            let voice_code_path = generate_voice_code_separate(item.file_path.clone());
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
pub async fn download_s3_names(pool : Arc<sqlx::Pool<sqlx::Postgres>>) -> Result<()> {
    let names_folder = String::from("Names/");
    EnvironmentProvider::default().credentials().await.unwrap();
    
    let bucket_name = S3_BUCKET.to_string();
    
    let mut list_obj_req = ListObjectsV2Request {
        bucket: bucket_name.clone(),
        prefix: Some(names_folder),
        ..Default::default()
    };
    let mut count : u64 = 0;
    loop {
        let s3 = S3Client::new(Region::from_str(&S3_REGION).unwrap());    
        let result = s3.list_objects_v2(list_obj_req.clone()).await.unwrap();
        for object in result.contents.as_ref().unwrap() {
            let s3_client = S3Client::new(Region::from_str(&S3_REGION).unwrap());

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
                convert_to_wav(key_value.to_string());

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
                denoise_audio(key_value.to_string());
                
                //remove silences
                let _res = remove_silence_audio(String::from(key_value));
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
                let voice_code_path = generate_voice_code(String::from(key_value));

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

