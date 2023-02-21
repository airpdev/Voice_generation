extern crate base64;
use base64::{encode};
use axum::{extract::Extension, Json, response::IntoResponse};
use axum_macros::debug_handler;
use crate::utils::{response::into_reponse};
use uuid::Uuid;
use sqlx::PgPool;
use std::fs::Metadata;
use std::fs;
use std::io;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
extern crate walkdir;
use walkdir::WalkDir;
use serde_json::{Value};
use crate::utils::audio_process::{extract_audio_name, copy_to_wav, extract_audio_batch, get_libritts_name};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use google_cloud_storage::client::Client;
use google_cloud_storage::sign::SignedURLOptions;
use google_cloud_storage::sign::SignedURLMethod;
use google_cloud_storage::http::Error;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::upload::UploadObjectRequest;
use tokio::task::JoinHandle;
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;
use crate::models::voice_generation::{
    PodcastTranscriptInfo
};
use std::{thread, time::Duration};

pub async fn prepare_voice_data(
    payload: String,
    Extension(pool): Extension<Arc<PgPool>>
) -> impl IntoResponse {

    parse_extract_trascript().await;

    let ret = serde_json::json!({
        "status": "success".to_string(),
    });
    into_reponse(200, ret)
}

pub async fn parse_extract_trascript() {
    let mut handles = vec![];
    for i in 0 .. 8 {
        handles.push(tokio::spawn(async move {
            thread::sleep(Duration::from_millis(10));

            for file in WalkDir::new(format!("../podcasts-transcripts/spotify-podcasts-2020/podcasts-transcripts/{}", i)).into_iter().filter_map(|file| file.ok()) {
                if !file.metadata().unwrap().is_file() {
                    continue;
                }

                let mut file_path = file.path().display().to_string();
                // if let Some(part) = file_path.get(24..) {
                //     file_path = part.to_string();
                // }
                // file_path = format!("../podcasts-dataset/{}", file_path);
                
                if file_path.ends_with(".json") {
                    println!("json: {}", file_path);
                    let mut total_transcript_array: Vec<PodcastTranscriptInfo> = Vec::new();
                    let file = fs::File::open(file.path().display().to_string()).expect("file should open read only");
                    let json: serde_json::Value = serde_json::from_reader(file).expect("file should be proper JSON");
                    let results = &json["results"];
                    for i in 0 .. results.as_array().unwrap().len() {
                        let alternatives= &results[i]["alternatives"];
                        let len = alternatives.as_array().unwrap().len();
                        for j in 0 .. len {
                            if alternatives[j].to_string().eq("{}") {
                                continue;
                            }
                            let mut transcript = alternatives[j]["transcript"].to_string();
                            if transcript.eq("null") {
                                continue;
                            }
                            // a.m p.m .com
                            transcript = transcript.replace("\"", "");
                            let words = &alternatives[j]["words"];
                            let transcript_array: Vec<String> = transcript.split(". ").filter(|s| !s.is_empty()).map(|x| format!("{}.", x)).collect();
                            let mut words_index = 0;
                            for k in 0 .. transcript_array.len() {
                                let words_count = count_words(transcript_array[k].to_string());
                                let starttime = words[words_index]["startTime"].to_string().replace("\"", "").replace("s", "").parse::<f64>().unwrap();
                                let endtime = words[words_index + words_count - 1]["endTime"].to_string().replace("\"", "").replace("s", "").parse::<f64>().unwrap();
                                let info = PodcastTranscriptInfo{transcript: transcript_array[k].to_string(), start_time: starttime, end_time: endtime};
                                total_transcript_array.push(info);
                                words_index = words_index + words_count;
                            }   
                        }
                    }
                    file_path = file_path.replace("../podcasts-transcripts/spotify-podcasts-2020/podcasts-transcripts/", "").replace(".json", "");
                    // file_path = "1/0/show_10AlBXJul8JZ5bREZUXBep/1am2bPIgTuCcAfqOY3rQZ1";
                    // "1/0/show_10AlBXJul8JZ5bREZUXBep/1_0_show_10AlBXJul8JZ5bREZUXBep_1am2bPIgTuCcAfqOY3rQZ1"
                    let original_path = format!("../podcasts-audio/{}.ogg", file_path);
                    if path_exists(original_path.to_string()) {
                        println!("original_path: {}", original_path.to_string());
                        let wav_path = copy_to_wav(&original_path);

                        let directory_path = format!("../podcasts-dataset/{}", file_path);
                        check_directory_existed(directory_path.to_string());

                        for i in 0 .. total_transcript_array.len() {
                            let text_file_path = format!("../podcasts-dataset/{}_{}.txt", get_libritts_name(file_path.to_string()), i);
                            if path_exists(text_file_path.to_string()) {
                                continue;
                            }
                            write_text_file(text_file_path, total_transcript_array[i].transcript.to_string());
                            // let voice_file_path = format!("../podcasts-dataset/{}_{}.wav", file_path, i);
                            // extract_audio_name(&voice_file_path, total_transcript_array[i].start_time, total_transcript_array[i].end_time, &original_path);
                        }
                        println!("text: {}", total_transcript_array.len());
                        extract_audio_batch(total_transcript_array, &file_path, &wav_path);

                    }
                    
                    // let _res = match std::fs::remove_file(original_path) {
                    //     Ok(_value) => { println!("success to remove temp file"); },
                    //     Err(_e) => { println!("failed to remove temp file"); }
                    // };
                }

            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    println!("finished!");
}

pub fn path_exists(path: String) -> bool {
    fs::metadata(&path).is_ok()
}

fn count_words(transcript: String) -> usize {
    let array = transcript.split_whitespace();
    let splited_array: Vec<&str> = array.collect();
    let mut transcript_array: Vec<&str> = Vec::new();
    for k in 0 .. splited_array.len() {
        if splited_array[k].len() > 0 {
            transcript_array.push(splited_array[k]);
        }
    }
    transcript_array.len()
}
fn check_directory_existed(path: String) {
    let path = PathBuf::from(path);
    let dir = path.parent().unwrap();
    if !path_exists(dir.to_str().unwrap().to_string()) {
        fs::create_dir_all(dir.to_str().unwrap());
    }
}
/*
use std::io::Cursor;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
async fn download_audio_googlecloud(url: String, file_name: String) -> Result<()> {
    // Create client.
    let mut client = Client::default().await.unwrap();
    // Download the file
    let data = client.download_object(&GetObjectRequest {
        bucket: "spotify-podcasts".to_string(),
        object: url,
        ..Default::default()
    }, &Range::default(), None).await.unwrap();

    let mut file = File::create(file_name)?;
    file.write_all(&data);

    Ok(())
}*/

fn write_text_file(path: String, content: String) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

