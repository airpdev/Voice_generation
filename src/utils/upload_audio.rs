extern crate base64;
use base64::{encode};
use std::{str::FromStr};
use rusoto_core::{Region};
use rusoto_s3::{S3Client, S3, PutObjectRequest, HeadObjectRequest, DeleteObjectRequest};

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
use sqlx::types::chrono::NaiveDateTime;

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioInfo {
    pub id: Uuid,
    pub user_id: String,
    pub file_path: String,
    pub voice_code: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
use std::io::prelude::*;
use std::fs::File;
use std::{env, ffi::OsStr};

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}
lazy_static! {
    static ref ASSETS_BUCKET: String = ensure_var("ASSETS_BUCKET").unwrap();
    static ref ASSETS_REGION: String = ensure_var("ASSETS_REGION").unwrap();
}
pub async fn is_existed_template(output_region: &String, output_bucket: &String, template_region: &String, template_bucket : &String, template_key : &String, extension : &String) -> String {
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return String::from("")
    };
    let s3_client = S3Client::new(region);

    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}.{}", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), extension);
    let mut s3_path = format!("{}{}", names_folder, s3_name);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: output_bucket.to_string(),
        ..Default::default()
        })
    .await;

    match response {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }
    s3_path
}
pub async fn is_existed_audio(output_region: &String, output_bucket: &String, template_region: &String, template_bucket : &String, template_key : &String, audio_region : &String, audio_bucket : &String, audio_key: &String) -> String {
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return String::from("")
    };
    let s3_client = S3Client::new(region);

    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), encode(format!("{}/{}/{}", audio_region, audio_bucket, audio_key)));
    let mut s3_path = format!("{}{}", names_folder, s3_name);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: output_bucket.to_string(),
        ..Default::default()
        })
    .await;

    match response {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }
    s3_path
}
pub async fn is_existed_lipsync_video(output_region: &String, output_bucket: &String, template_region: &String, template_bucket : &String, template_key : &String, audio_region : &String, audio_bucket : &String, audio_key: &String) -> String {
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return String::from("")
    };
    let s3_client = S3Client::new(region);

    let names_folder = String::from("generated_video/");
    let s3_name = format!("{}@{}.mp4", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), encode(format!("{}/{}/{}", audio_region, audio_bucket, audio_key)));
    let mut s3_path = format!("{}{}", names_folder, s3_name);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: output_bucket.to_string(),
        ..Default::default()
        })
    .await;

    match response {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }
    s3_path
}

pub async fn is_existed(template_bucket : &String, template_key : &String, user_key : &String) -> String {
    let s3_client = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());
    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(format!("{}/{}", template_bucket, template_key)), user_key);
    let mut s3_path = format!("{}{}", names_folder, s3_name);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: ASSETS_BUCKET.to_string(),
        ..Default::default()
        })
    .await;

    match response {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }
    s3_path
}
pub async fn upload_template(output_region: &String, output_bucket: &String, template_region: &String, template_bucket: &String, template_key : &String, file_path : &String, extension : &String) -> String {
    let file = File::open(file_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // upload to s3 bucket
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return _e.to_string()
    };
    let s3 = S3Client::new(region);

    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}.{}", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), extension);
    let mut s3_path = format!("{}{}", names_folder, s3_name);
    let result = s3.put_object(PutObjectRequest {
                                key: s3_path.clone(),
                                content_type: Some("*".to_string()),
                                content_disposition: Some(format!("inline; filename={}", s3_name)),
                                content_length: Some(buffer.len() as i64),
                                body: Some(buffer.into()),
                                bucket: output_bucket.to_string(),
                                acl: Some("public-read".to_string()),
                                ..Default::default()
                                })
                    .await;
    match result {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }

    s3_path
}
pub async fn upload_asset(output_region: &String, output_bucket: &String, template_region: &String, template_bucket: &String, template_key : &String, audio_region: &String, audio_bucket: &String, audio_key : &String, file_path : &String) -> String {
    let file = File::open(file_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // upload to s3 bucket
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return _e.to_string()
    };
    let s3 = S3Client::new(region);
    
    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), encode(format!("{}/{}/{}", audio_region, audio_bucket, audio_key)));
    let mut s3_path = format!("{}{}", names_folder, s3_name);
    let result = s3.put_object(PutObjectRequest {
                                key: s3_path.clone(),
                                content_type: Some("audio/wav".to_string()),
                                content_disposition: Some(format!("inline; filename={}", s3_name)),
                                content_length: Some(buffer.len() as i64),
                                body: Some(buffer.into()),
                                bucket: output_bucket.to_string(),
                                acl: Some("public-read".to_string()),
                                ..Default::default()
                                })
                    .await;
    match result {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }

    s3_path
}
pub async fn upload_video_lipsync(output_region: &String, output_bucket: &String, template_region: &String, template_bucket: &String, template_key : &String, audio_region: &String, audio_bucket: &String, audio_key : &String, file_path : &String) -> String {
    let file = File::open(file_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // upload to s3 bucket
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return _e.to_string()
    };
    let s3 = S3Client::new(region);
    
    let names_folder = String::from("generated_video/");
    let s3_name = format!("{}@{}.mp4", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), encode(format!("{}/{}/{}", audio_region, audio_bucket, audio_key)));
    let mut s3_path = format!("{}{}", names_folder, s3_name);
    let result = s3.put_object(PutObjectRequest {
                                key: s3_path.clone(),
                                content_type: Some("video/mp4".to_string()),
                                content_disposition: Some(format!("inline; filename={}", s3_name)),
                                content_length: Some(buffer.len() as i64),
                                body: Some(buffer.into()),
                                bucket: output_bucket.to_string(),
                                acl: Some("public-read".to_string()),
                                ..Default::default()
                                })
                    .await;
    match result {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }

    s3_path
}
pub async fn upload_audio(template_bucket: &String, template_key : &String, user_key : &String, file_path : &String) -> String {
    let file = File::open(file_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // upload to s3 bucket
    let s3 = S3Client::new(Region::from_str(&ASSETS_REGION).unwrap());
    
    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(format!("{}/{}", template_bucket, template_key)), user_key);
    let mut s3_path = format!("{}{}", names_folder, s3_name);
    let result = s3.put_object(PutObjectRequest {
                                key: s3_path.to_string(),
                                content_type: Some("audio/wav".to_string()),
                                content_disposition: Some(format!("inline; filename={}", s3_name)),
                                content_length: Some(buffer.len() as i64),
                                body: Some(buffer.into()),
                                bucket: ASSETS_BUCKET.to_string(),
                                acl: Some("public-read".to_string()),
                                ..Default::default()
                                })
                    .await;
    match result {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }

    s3_path
}
pub async fn is_existed_path(output_region : &String, output_bucket : &String, template_region : &String, template_bucket : &String, template_key : &String, user_key : &String) -> String {
    // upload to s3 bucket
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return _e.to_string()
    };
    let s3_client = S3Client::new(region);

    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), user_key);
    let mut s3_path = format!("{}{}", names_folder, s3_name);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: output_bucket.to_string(),
        ..Default::default()
        })
    .await;

    match response {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }
    s3_path
}
pub async fn upload_audio_path(output_region : &String, output_bucket: &String, template_region: &String, template_bucket: &String, template_key : &String, user_key : &String, file_path : &String) -> String {
    let file = File::open(file_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // upload to s3 bucket
    let region : Region = match Region::from_str(output_region) {
        Ok(value) => value,
        Err(_e) => return _e.to_string()
    };
    let s3 = S3Client::new(region);

    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(format!("{}/{}/{}", template_region, template_bucket, template_key)), user_key);
    let mut s3_path = format!("{}{}", names_folder, s3_name);
    let result = s3.put_object(PutObjectRequest {
                                key: s3_path.to_string(),
                                content_type: Some("audio/wav".to_string()),
                                content_disposition: Some(format!("inline; filename={}", s3_name)),
                                content_length: Some(buffer.len() as i64),
                                body: Some(buffer.into()),
                                bucket: output_bucket.to_string(),
                                acl: Some("public-read".to_string()),
                                ..Default::default()
                                })
                    .await;
    match result {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }

    s3_path
}

pub async fn clear_whisper_cache(transcript_bucket : &String, transcript_key: &String) {
    let audio_key = format!("{}.wav", transcript_key);
    let s3_client = S3Client::new(Region::from_str("us-east-2").unwrap());
    let mut s3_path = format!("transcript-cache-1/{}/{}.pcm", transcript_bucket, audio_key);

    println!("cache path: {:?}", s3_path);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: "tmp-dev-283501".to_string(),
        ..Default::default()
        })
    .await;

    match response {
        Ok(success) => { 
            println!("Success: {:?}", success);
            
        },
        Err(error) => {
            println!("Failure: {:?}", error);
            s3_path = String::from("");
        }
    }

    if s3_path.len() > 0 {
        let pcm_path = format!("transcript-cache-1/{}/{}.pcm", transcript_bucket, audio_key);
        let _res = s3_client.delete_object(DeleteObjectRequest{
            key: pcm_path.clone(),
            bucket: "tmp-dev-283501".to_string(),
            ..Default::default()
        }).await;

        let json_path = format!("transcript-cache-1/{}/{}.json", transcript_bucket, audio_key);
        let _res = s3_client.delete_object(DeleteObjectRequest{
            key: json_path.clone(),
            bucket: "tmp-dev-283501".to_string(),
            ..Default::default()
        }).await;
    }
}