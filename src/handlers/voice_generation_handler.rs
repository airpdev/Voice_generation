use axum::{
    extract::{ContentLengthLimit, Multipart, Query,  Extension},
    headers::{ HeaderMap, HeaderName, HeaderValue},
};
use axum_macros::debug_handler;
use sqlx::PgPool;
use std::fmt::Write;
use std::sync::Arc;
use std::io;
use std::io::prelude::*;
use std::fs::File;

use crate::models::voice_generation::{
    AudioInfo,
    UploadParam,
};

use crate::utils::audio_process::{launch_inference_audio, launch_normalizing_audio, generate_yaml, get_system_time, generate_voice_code, remove_silence_audio, similarity_voice_code, path_exists, convert_to_wav, denoise_audio};
// API
#[debug_handler]
pub async fn generate(
    params: Query<UploadParam>,
    Extension(pool): Extension<Arc<PgPool>>,
    ContentLengthLimit(mut multipart): ContentLengthLimit<Multipart, { 2500 * 1024 * 1024 }>,
) ->  (HeaderMap, Vec<u8>) {
    
    let suffix = "Names/";
    let mut headers = HeaderMap::new();

    let mut start_time : u128 = get_system_time();

    let mut target_path : String = format!("{}", start_time);
    let mut threshold : f64 = 0.0;

    if let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let mut file_name = field.file_name().unwrap().to_string();
        file_name = file_name.replace(" ", "_");
        let content_type = field.content_type().unwrap().to_string();
        let file_path = get_temp_file();
        println!("=================================================================================================");
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_str(&content_type).unwrap(),
        );

        let audio_path = format!("{}{}", suffix, file_path);

        println!("uploading `{}`, `{}`, `{}`", name, file_name, content_type);
        start_time = get_system_time();

        let bytes = field.bytes().await.unwrap().to_vec().clone();    
        let mut reader: &[u8] = &bytes;
        // Create a file 
        let mut out = File::create(audio_path.clone()).expect("failed to create file");
        //Copy data to the file
        io::copy(&mut reader, &mut out).expect("failed to copy content");

        /*
        audio - processing: converting, denoising, removing silence, 
        */
        println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

        // convert any audio format to .wav format using ffmpeg
        convert_to_wav(audio_path.clone());

        // denoise
        denoise_audio(audio_path.clone());

        // remove silences
        match remove_silence_audio(audio_path.clone()) {
            Ok(wav_path) => println!("finished removing silences : {}", wav_path),
            Err(_) => println!("failed to remove noises of {}", audio_path.clone()),
        }

        let voice_code_path = generate_voice_code(audio_path.clone());

        //get similar names from database
        let i = &*pool;
        let mut query = "SELECT * FROM audio_names WHERE lower(file_path) LIKE ".to_string();
        write!(query, "'%{}%'", params.user_name.to_lowercase()).unwrap();
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
                let similarity_value = similarity_voice_code(voice_code_path.clone(), item.voice_code.clone()).unwrap();
                if similarity_value > threshold {
                    threshold = similarity_value;
                    vst_file_path = item.file_path.clone();
                }
               
            }
            println!("similarity_value : {} ---> {}", vst_file_path, threshold);
            
            if threshold > 0.6 {
                let yaml_path = generate_yaml(vst_file_path.clone(), file_path.clone(), target_path.clone()).unwrap();
                launch_inference_audio(yaml_path.clone());
                launch_normalizing_audio(yaml_path.clone());
                //vst_generate_audio(yaml_path.clone());

                let _res = std::fs::remove_file(format!("VST/{}.yaml", target_path));
                target_path = format!("VST/custom_test/{}_gen.wav", target_path);
       
                let file = File::open(target_path.clone());
                let mut file = match file{
                    Ok(file) => file,
                    Err(error) => panic!("Problem opening the file: {:?}", error),
                };
                let mut buffer = Vec::new();
                let _res = file.read_to_end(&mut buffer);

                let _res = std::fs::remove_file(&target_path);

                println!("processing_time : {} ", get_system_time() - start_time);
                return (headers, buffer);
            }

        } 
        
    } 

    let empty: Vec<u8> = Vec::new(); 
    (headers, empty)
}

fn get_temp_file() -> String {
    if !path_exists("Names/Temp") {
        let _res = std::fs::create_dir("Names/Temp");
    }
    let path = format!("{}{}{}", "Temp/", get_system_time(), ".wav");
    //let path = format!("{}{}{}", "Temp/", "vst_", value);
    path
}
