extern crate base64;
use base64::{encode};
use axum::{
    extract::{ContentLengthLimit, Multipart, Query,  Extension},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use sqlx::PgPool;
use crate::models::voice_generation::{
    SimilarityInfo,
    ExtractInfo,
    TranscriptCsvInfo,
    TranscriptInfo
};
use std::sync::{Arc};
use crate::utils::{response::into_reponse};
use serde_json::{Value};
use crate::utils::download_audio::{download_template_with_path};
use std::{env, ffi::OsStr};
use rusoto_s3::{S3Client, S3, PutObjectRequest};
use rusoto_core::{Region};
use std::str::FromStr;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use uuid::Uuid;
use crate::utils::audio_process::{replace_audio, extract_audio, denoise_audio, extract_audio_name, path_exists};
use crate::utils::upload_audio::{clear_whisper_cache};
extern crate csv;
use std::io::Cursor;
use std::io::Write;
use std::cmp::min;
use futures_util::StreamExt;
use std::process::{Command};

#[debug_handler]
pub async fn extract_transcripts_csv() -> impl IntoResponse  {

    let file = std::fs::File::open("extracts/transcripts.csv").unwrap();
	let mut rdr = csv::ReaderBuilder::new()
					.has_headers(true)
					.from_reader(file);

    let mut csv_transcripts: Vec<TranscriptCsvInfo> = Vec::new();
	// push all the records
    for result in rdr.records().into_iter() {
        let record = result.unwrap();
        let row: &csv::StringRecord = &record;
        csv_transcripts.push(TranscriptCsvInfo{ s3_links: row[0].parse().unwrap(),
                                                names: row[1].parse().unwrap(),
                                                created: row[2].parse().unwrap(),
                                                done: row[3].parse().unwrap(),
                                                modified: row[4].parse().unwrap() });
    }
    
    for index in 0 .. csv_transcripts.len() {
        let dir_path = format!("extracts/{}", csv_transcripts[index].names.to_string());
        if path_exists(&*dir_path) {
            continue;
        }
        fs::create_dir(&*dir_path);

        let mut audio_path = String::from("");
        let response = download_audio_csv(format!("https://{}", csv_transcripts[index].s3_links.to_string()), csv_transcripts[index].names.to_string()).await;
        match response {
            Ok(p) => audio_path = p,
            Err(_e) => {
                audio_path = String::from("");
            }
        };
        if audio_path.len() == 0 {
            println!("Failed to download audio!");
            continue;
        }

        let response = validation_check(csv_transcripts[index].s3_links.to_string()).await;
        if response.len() == 0 {
            continue;
        }
        let json_data : SimilarityInfo =  parse_mturk_whisper(response);

        if json_data.transcript.len() == 0 {
            println!("Failed to parse whisper response!");
            continue;
        }

        let transcript_splits = json_data.transcript.split(" ");
        let transcript_array: Vec<&str> = transcript_splits.collect();   
        let mut words_array : Vec<TranscriptInfo> = Vec::new();    
        for i in 0 .. json_data.alignments.len() {
            if i == 0 {
                let info = TranscriptInfo {transcript: transcript_array[i].to_string(), start_time: json_data.alignments[i].t[0], end_time: json_data.alignments[i].t[1]};
                words_array.push(info);
                continue;
            } 
            if transcript_array.len() <= i {
                continue;
            }
            let length = words_array.len();
            let interval = json_data.alignments[i].t[0] - words_array[length -1].end_time;
            if interval < 2400 {
                words_array[length -1].transcript = format!("{} {}", words_array[length -1].transcript, transcript_array[i]);
                words_array[length -1].end_time = json_data.alignments[i].t[1];
            } else {
                let info = TranscriptInfo {transcript: transcript_array[i].to_string(), start_time: json_data.alignments[i].t[0], end_time: json_data.alignments[i].t[1]};
                words_array.push(info);
            }
        }
        for i in 0 .. words_array.len() {
            let names = words_array[i].transcript.split(" ");
            let names: Vec<&str> = names.collect(); 
            let mut transcript = String::from("");
            if names.len() == 0 {
                transcript = names[0].replace(&['(', ')', ',', '\"', '.', ';', '!', ' ', ':', '\''][..], "");
            } else {
                for j in 1 .. names.len() {
                    transcript = format!("{} {}", transcript, names[j].replace(&['(', ')', ',', '\"', '.', ';', '!', ' ', ':', '\''][..], ""));
                }
            }
            
            if transcript.len() == 0 {
                continue;
            }
            let transcript_path = format!("{}/{}.wav", dir_path, transcript);
            let start_time = words_array[i].start_time as f64 / 16000.0;
            let end_time = words_array[i].end_time as f64 / 16000.0;
            extract_audio_name(&transcript_path, start_time, end_time, &audio_path);

            let file = File::open(&transcript_path);
            let mut file = match file{
                Ok(file) => file,
                Err(error) => panic!("Problem opening the file: {:?}", error),
            };
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).unwrap();
            let s3 = S3Client::new(Region::from_str("us-east-1").unwrap());
            let result = s3.put_object(PutObjectRequest {
                            key: transcript_path,
                            content_type: Some("*".to_string()),
                            content_disposition: Some(format!("inline; filename={}.wav", transcript)),
                            content_length: Some(buffer.len() as i64),
                            body: Some(buffer.into()),
                            bucket: "assets-bhuman-new".to_string(),
                            acl: Some("public-read".to_string()),
                            ..Default::default()
                            }).await;
        }
        // zip
        // let mut child = Command::new("zip")
        //                         .arg("-r")
        //                         .arg(format!("extracts/{}.zip", csv_transcripts[index].names.to_string()))
        //                         .arg(format!("extracts/{}", csv_transcripts[index].names.to_string()))
        //                         .spawn()
        //                         .expect("Failed to zip");
        // child.wait().expect("Failed to zip");

        // uploading zip file
        
        
        println!("{} / {} has been finished!", index + 1, csv_transcripts.len());                                   
	}

    let ret = serde_json::json!({
        "status": "success".to_string(),
    });
    return into_reponse(200, ret);
}
/*
fn count_words(s: &str) -> usize {
    let mut total = 0;
    let mut previous = char::MAX;
    for c in s.chars() {
        // If previous char is whitespace, we are on a new word.
        if previous.is_ascii_whitespace() {
            // New word has alphabetic, digit or punctuation start.
            if c.is_ascii_alphabetic() || c.is_ascii_digit() || c.is_ascii_punctuation() {
                total += 1;
            }
        }
        // Set previous.
        previous = c;
    }
    if s.len() >= 1 {
        total += 1
    }
    total
}*/
pub async fn download_audio_csv(url : String, names: String) -> Result<String, String>  {
    println!("{}: {}", names, url);
    let client = reqwest::Client::new();
    let res = client
            .get(&url)
            .send()
            .await
            .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
            .content_length()
            .ok_or(format!("Failed to get content length from '{}'", &url))?;
    
    let path = format!("extracts/{}.wav", names);
    let mut file = File::create(&path).or(Err(format!("Failed to create file '{}'", &path)))?;

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
    }

    Ok(path)
}

#[debug_handler]
pub async fn extract_audio_transcripts(payload: String) -> impl IntoResponse {
    println!("payload : {:#?}", payload);   
    let params: ExtractInfo;
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
    let mut template_path = format!("Names/Temp/{}.wav", encode(format!("{}/{}/{}", &params.region, &params.bucket, &params.key))); 
    if !path_exists(&*template_path) {
        template_path = download_template_with_path(&params.region, &params.bucket, &params.key, &template_path).await.unwrap();
        if template_path.len() == 0 {
            let ret = serde_json::json!({
                "error": "template audio is not existed!".to_string(),
            });
            return into_reponse(400, ret);
        }
        let file_size = std::fs::metadata(&template_path).unwrap().len();
        if file_size == 0 {
            let ret = serde_json::json!({
                "error": "template audio is empty!".to_string(),
            });
            return into_reponse(400, ret);
        }
    }

    let audio_path = extract_audio(&template_path);
//  denoise_audio(&audio_path);
//  uploading wav file
//    let audio_path = "extracts/jenny-names.zip";
/*    let file = File::open(audio_path);
    let mut file = match file{
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let s3 = S3Client::new(Region::from_str("us-east-2").unwrap());
    let mut s3_path = format!("{}.zip", &params.key);
    let result = s3.put_object(PutObjectRequest {
        key: s3_path.clone(),
        content_type: Some("*".to_string()),
        content_disposition: Some(format!("inline; filename={}", "s3_name.zip")),
        content_length: Some(buffer.len() as i64),
        body: Some(buffer.into()),
        bucket: "assets-dev-283501".to_string(),
        acl: Some("public-read".to_string()),
        ..Default::default()
        }).await;*/

    let response = get_validation_check(&params).await;
    // println!("response: {}", response);

    let json_data : SimilarityInfo =  parse_mturk_whisper(response);

	let transcript_splits = json_data.whisper_transcript.split(".");
	let transcript_array: Vec<&str> = transcript_splits.collect();   
    
    let dir_path = format!("extracts/{}", params.key.replace("/", ""));
    if !path_exists(&*dir_path) {
        fs::create_dir(&*dir_path);
    }
    for i in 0 .. transcript_array.len() {
        let transcripts = transcript_array[i];
        if transcripts.len() == 0 {
            continue;
        }
        let names = transcripts.split(" ");
	    let names: Vec<&str> = names.collect(); 
        let transcript = names[names.len() - 1];
        let start_time = json_data.alignments[i * 2].t[0] as f64 / 16000.0;
        let end_time = json_data.alignments[i * 2 + 1].t[1] as f64 / 16000.0;

        let transcript_path = format!("{}/{}.wav", dir_path, transcript);
        println!("{} - {} - {}", &transcript_path, start_time, end_time); 

        extract_audio_name(&transcript_path, start_time, end_time, &audio_path);
    }
    println!("finished");

    let ret = serde_json::json!({
        "status": "success".to_string(),
    });
    return into_reponse(200, ret);
}

pub fn parse_mturk_whisper(response: String) -> SimilarityInfo {
    let response_list: Vec<&str> = response.split("\n").collect();
    let similarity_data = response_list[response_list.len() - 2];
    let similarity_data = format!(r#"{}"#, similarity_data);
    let mut json_array: Vec<Value> = Vec::new();
    let json_data = serde_json::from_str(&similarity_data);
    match json_data {
        Ok(file) => json_array = file,
        Err(error) => {
            let info = SimilarityInfo{transcript: String::from(""), whisper_transcript:  String::from(""), alignments: Vec::new()};
            return info;
        }
    };
    
    let similarity_data = serde_json::to_string(&json_array[json_array.len() - 1]).unwrap().to_lowercase();

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

pub async fn get_validation_check(params: &ExtractInfo) -> String {
	let url = format!("https://whisper.dev.bhuman.ai/{}/{}", params.bucket, params.key);
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
pub async fn validation_check(path : String) -> String {
	let url = format!("https://whisper.dev.bhuman.ai/{}", path.get(17..).unwrap());
    //println!("url: {}", url);
    let client = reqwest::Client::new();
	let response = client.get(url)
						.send()
						.await
						.unwrap();
    match response.text().await {
        Ok(result) => {
            //println!("result: {}", result);
            result
        }
        Err(_e) => {
            println!("result: {}", _e.to_string());
            "".to_string()
        }
    }
}