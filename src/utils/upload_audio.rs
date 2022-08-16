extern crate base64;
use base64::{encode};
use std::{str::FromStr};
use rusoto_core::{Region};
use rusoto_s3::{S3Client, S3, PutObjectRequest, HeadObjectRequest};

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
    static ref S3_BUCKET: String = ensure_var("S3_BUCKET").unwrap();
    static ref S3_REGION: String = ensure_var("S3_REGION").unwrap();
}

pub async fn is_existed(template_bucket : String, template_key : String, user_key : String) -> String {
    let s3_client = S3Client::new(Region::from_str(&S3_REGION).unwrap());
    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(template_key.clone()), user_key);
    let mut s3_path = format!("{}{}", names_folder, s3_name);

    let response = s3_client.head_object(HeadObjectRequest {
        key: s3_path.clone(),
        bucket: template_bucket,
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

pub async fn upload_audio(template_key : String, user_key : String, file_path : String) -> String {
    let file = File::open(file_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // upload to s3 bucket
    let s3 = S3Client::new(Region::from_str(&S3_REGION).unwrap());
    
    let names_folder = String::from("generated_audio/");
    let s3_name = format!("{}@{}.wav", encode(template_key), user_key);
    let mut s3_path = format!("{}{}", names_folder, s3_name);
    let result = s3.put_object(PutObjectRequest {
                                key: s3_path.clone(),
                                content_type: Some("audio/wav".to_string()),
                                content_disposition: Some(format!("inline; filename={}", s3_name)),
                                content_length: Some(buffer.len() as i64),
                                body: Some(buffer.into()),
                                bucket: S3_BUCKET.to_string(),
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
