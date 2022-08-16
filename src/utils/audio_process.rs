extern crate hound;
extern crate csv;
use uuid::Uuid;
use std::process::{Command};
use std::io;
use crate::models::voice_generation::{
    Silence,
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::LineWriter;
use std::env;

pub fn generate_voice_code(key_value : String) -> String {
	let path_splits = key_value.split("/");                                 // Names/foler/123.wav
	let path_array: Vec<&str> = path_splits.collect();                      // [Names, folder, 123.wav]
	let mut voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], path_array[1]);
	if voice_code_path.contains("Names/Temp") {
		voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], path_array[2]);
	}

	if !path_exists(&*voice_code_path) {
		let mut command = Command::new("/home/ubuntu/.virtualenvs/resemblyzer/bin/python")
		.arg("generate_voice_code.py")
		.arg("-spk1")
		.arg(key_value)
		.arg("-output")
		.arg(&voice_code_path)
		.spawn()
		.expect("Failed to get voice code");
		command.wait().expect("failed to wait on generating voice code");
		println!("voice_code_path : {}", &voice_code_path);
	}
	
    voice_code_path
}
pub fn generate_voice_code_separate(key_value : String) -> String {
	let voice_code_path = format!("{}.csv", key_value);

	if !path_exists(&*voice_code_path) {
		let mut command = Command::new("/home/ubuntu/.virtualenvs/resemblyzer/bin/python")
		.arg("generate_voice_code.py")
		.arg("-spk1")
		.arg(key_value)
		.arg("-output")
		.arg(&voice_code_path)
		.spawn()
		.expect("Failed to get voice code");
		command.wait().expect("failed to wait on generating voice code");
		println!("voice_code_path : {}", &voice_code_path);
	}
	
    voice_code_path
}
fn read_csv(filepath: String, has_headers: bool) -> Vec<f64> {
	// Open file
	let file = std::fs::File::open(filepath).unwrap();
	let mut rdr = csv::ReaderBuilder::new()
					.has_headers(has_headers)
					.from_reader(file);

	let mut data_frame = Vec::new();

	// push all the records
	for result in rdr.records().into_iter() {
	   let record = result.unwrap();
	   let row: &csv::StringRecord = &record;
	   data_frame.push(row[0].parse().unwrap());
	}
	return data_frame;
}

pub fn similarity_voice_code(audio1 : String, audio2 : String) -> io::Result<f64> {
	if !path_exists("Names/Temp") {
        let _res = fs::create_dir("Names/Temp");
    }
	
	// getting similarity using rust
	let audio1_data : Vec<f64> = read_csv(audio1, false);
	let audio2_data : Vec<f64> = read_csv(audio2, false);

	let mut value : f64 = 0.0;
	for i in 0 .. audio1_data.len() {
		value += audio1_data[i] * audio2_data[i];
	}

	// getting similarity using python
    /*
	let output_file_path = format!("Names/Temp/{}.out", generate_id()); 
    let mut command = Command::new("/home/ubuntu/.virtualenvs/resemblyzer/bin/python")
                            .arg("two_speakers_similarity_from_file.py")
                            .arg("-csv1")
                            .arg(audio1)
                            .arg("-csv2")
                            .arg(audio2)
							.arg("-out")
							.arg(&output_file_path)
                            .spawn()
                            .expect("Failed to get similarity");
    command.wait().expect("failed to wait on getting similarity");

	let mut output_file = File::open(&output_file_path)?;
    let mut similarity_value = String::new();
    output_file.read_to_string(&mut similarity_value)?;
	fs::remove_file(output_file_path)?;
	let value: f64 = similarity_value.trim().parse().unwrap();
*/
	Ok(value)
}
pub fn vst_generate_audio(yaml_path : String) {
	println!("VST started =====================");
	println!("VST yaml_path : {}", yaml_path);

	let mut child = Command::new("bash")
							.arg("launch_vst.sh")
							.arg(yaml_path)
							.spawn()
							.expect("Failed to VST process");
	child.wait().expect("failed to wait on processing VST");
	println!("VST finished =====================");
}
pub fn launch_inference_audio(yaml_path : String) {
	println!("launch_inference started =====================");

	let mut child = Command::new("bash")
							.arg("launch_inference.sh")
							.arg(yaml_path)
							.spawn()
							.expect("Failed to launch_inference process");
	child.wait().expect("failed to wait on processing launch_inference");

	println!("launch_inference finished =====================");
}
pub fn launch_normalizing_audio(yaml_path : String) {
	println!("launch_normalizing started =====================");

	let mut child = Command::new("bash")
							.arg("launch_normalizing.sh")
							.arg(yaml_path)
							.spawn()
							.expect("Failed to launch_normalizing process");
	child.wait().expect("failed to wait on processing launch_normalizing");

	println!("launch_normalizing finished =====================");
}
pub fn generate_yaml(vst_file_path : String, file_path : String, target_path : String) -> Result<String, io::Error> {
	let yaml_path = format!("{:#?}/VST/{}.yaml", env::current_dir()?.display(), target_path.clone()).replace("\"", "");
	let file = File::create(yaml_path.clone())?;
    let mut file = LineWriter::new(file);
	let vst_file_path = format!("{:#?}/{}", env::current_dir()?.display(), vst_file_path.clone());
	let file_path = format!("{:#?}/{}", env::current_dir()?.display(), file_path.clone());
	let content = format!("{}:\n    ref:\n    - {}\n    ref_spk_name: carolyn\n    src: {}\n    src_spk_name: anusha", target_path.clone(), file_path.replace("\"", ""), vst_file_path.replace("\"", ""));
	let _res = file.write_all(content.as_bytes());
	file.flush()?;
	Ok(yaml_path)
}
pub fn denoise_audio(file_path : String) {
	let mut child = Command::new("/home/ubuntu/.virtualenvs/deepfilternet/bin/python")
	.arg("DeepFilterNet/DeepFilterNet/df/enhance.py")
	.arg("--log-level")
    .arg("error")
	.arg("-m")
	.arg("DeepFilterNet")
	.arg(file_path)
	.spawn()
	.expect("Failed to remove noise");
	child.wait().expect("failed to wait on removing noise");
	println!("Audio file has been denoised.");
}
pub fn convert_to_wav(file_path : String) {
	// conversion audio files into .wav using ffmpeg
	let output_path = format!("{}.wav", file_path.clone());
	let mut command = Command::new("ffmpeg")
							.arg("-nostdin")
							.arg("-loglevel")
							.arg("error")
							.arg("-i")
							.arg(file_path.clone())
							.arg(&output_path)
							.spawn()
							.expect("Failed to convert using ffmpeg");
	command.wait().expect("Failed to convert using ffmpeg");
	let _res = fs::remove_file(file_path.clone());
	let _res = fs::rename(output_path, file_path.clone());
	println!("Audio file has been converted.");
}
pub fn remove_silence_audio(file_path : String) -> Result<String, io::Error> {
	let mut audio_file = hound::WavReader::open(file_path.clone()).unwrap();
	let audio_spec = audio_file.spec();
	let raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();
	let mut samples: Vec<i32> = Vec::new(); 

	let silences : Vec<Silence> = get_silences_audio(file_path.clone());
	if silences.len() > 0 {
		for i in 0..=raw_samples.len() - 1 {
			let mut is_silence : bool = false;
			for j in 0..=silences.len() - 1 {
				if i >= silences[j].start_index && i <= silences[j].end_index {
					is_silence = true;
				}
			}
			if !is_silence {
				samples.push(raw_samples[i as usize]);	
			}
		}
		let mut writer = hound::WavWriter::create(file_path.clone(), audio_spec).unwrap();
		let mut k = 0;
		while k < samples.len() {
			for _k in 0..=audio_spec.channels - 1 {
				writer.write_sample(samples[k as usize]).unwrap();
			}	
			k += audio_spec.channels as usize;
		}
	} else {
		let mut writer = hound::WavWriter::create(file_path.clone(), audio_spec).unwrap();
		for k in 0..=raw_samples.len() - 1 {
			writer.write_sample(raw_samples[k as usize]).unwrap();
		}
	}
    Ok(file_path.clone())
}

fn get_silences_audio(file_path : String) -> Vec<Silence> {
	let mut audio_file = hound::WavReader::open(file_path).unwrap();
	let audio_spec = audio_file.spec();
	let raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();
	let mut silences : Vec<Silence> = Vec::new(); 
	let interval = audio_spec.sample_rate * audio_spec.channels as u32;
	let threshold = -40.0;
	let mut is_created : bool = false;
	let mut silence : Silence = Silence{start_index : 0, 
										start_time: 0.0, 
										end_index : 0, 
										end_time : 0.0};
	for i in 0..= raw_samples.len() - 1 {
		if get_db_audio(raw_samples[i as usize].abs() as f64, audio_spec.bits_per_sample as f64) < threshold {
			if is_created == false {
				silence = Silence { start_index : i, 
									start_time: i as f64 / interval as f64, 
									end_index : 0, 
									end_time : 0.0};
				is_created = true;
			} else {
				silence.end_index = i;
				silence.end_time = i as f64 / interval as f64;
			}
		} else {
			if is_created == true && (silence.end_index as f64 - silence.start_index as f64) > 0.4 * interval as f64 {
				silence.start_time = round_val(silence.start_time);
				silence.end_time = round_val(silence.end_time);
				silences.push(silence);
			} 
			if is_created == true {
				is_created = false;
				silence = Silence { start_index : 0, 
									start_time: 0.0, 
									end_index : 0, 
									end_time : 0.0};
			}
		}
	}
	if is_created == true && (silence.end_index as f64 - silence.start_index as f64) > 0.4 * interval as f64 {
		silence.start_time = round_val(silence.start_time);
		silence.end_time = round_val(silence.end_time);
		silences.push(silence);
	} 
	silences
}

fn get_db_audio(peak_value : f64, bits_value : f64) -> f64 {
	if peak_value.abs() < 10.0 {
		-80.0
	} else {
		let max_value = (bits_value - 1.0).exp2();
		let amplitude : f64 = peak_value.abs() / max_value;
		20.0 * amplitude.log10()
	}
}

fn round_val(value : f64) -> f64 {
	let rounded_value = (value * 1000.0).round() / 1000.0;
	rounded_value
}

pub fn generate_id() -> String {
    let  uuid_number = Uuid::new_v4();
    let str_uuid_number:String = uuid_number.to_string();
    return str_uuid_number;
}

pub fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
pub fn is_check_wav(path : &str) -> Result<bool, io::Error> {
	let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_err) => return Ok(false),
    };

	// let mut f = File::open(path).unwrap();
	let mut buffer = vec![0; 4];

	// read up to 10 bytes
	let _res = file.read(&mut buffer);
	let value: String = String::from_utf8(buffer.clone()).unwrap();
	if value.to_lowercase() == "riff" {
		Ok(true)
	} else {
		Ok(false)
	}
}

pub fn get_system_time() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
