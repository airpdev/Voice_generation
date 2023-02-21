extern crate hound;
extern crate csv;
use uuid::Uuid;
use std::process::{Command};
use std::io;
use crate::models::voice_generation::{
    Silence, PodcastTranscriptInfo
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::LineWriter;
use std::env;
use csv::Writer;
use std::fs::OpenOptions;

pub fn generate_voice_code(key_value : &String) -> String {
	let path_splits = key_value.split("/");                                 // Names/foler/123.wav
	let path_array: Vec<&str> = path_splits.collect();                      // [Names, folder, 123.wav]
	let mut voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], path_array[1]);
	if voice_code_path.contains("Names/Temp") {
		voice_code_path = format!("{}/{}/{}.csv", path_array[0], path_array[1], path_array[2]);
	}

	if !path_exists(&*voice_code_path) {
		let mut child = Command::new("bash")
							.arg("resemblyzer_voice_code.sh")
							.arg(format!("-spk1 {} -output {}", key_value, &voice_code_path))
							.spawn()
							.expect("Failed to get voice code");
		child.wait().expect("failed to wait on generating voice code");
		println!("voice_code_path : {}", &voice_code_path);
	}
	
    voice_code_path
}
pub fn generate_voice_code_separate(key_value : &String) -> String {
	let voice_code_path = format!("{}.csv", key_value);

	if !path_exists(&*voice_code_path) {
		let mut child = Command::new("bash")
								.arg("resemblyzer_voice_code.sh")
								.arg(format!("-spk1 {} -output {}", key_value, &voice_code_path))
								.spawn()
								.expect("Failed to get voice code");
		child.wait().expect("failed to wait on generating voice code");
		println!("voice_code_path : {}", &voice_code_path);
	}
	
    voice_code_path
}
pub fn launch_prosody_audio(param : String) {
	println!("launch_prosody started =====================");
	
	let mut child = Command::new("bash")
							.arg("huggingface_prosody_generate.sh")
							.arg(param)
							.spawn()
							.expect("Failed to prosody process");
	child.wait().expect("failed to wait on processing prosody");

	println!("prosody finished =====================");
}
pub fn launch_huggingface_audio(param : String) {
	println!("launch_huggingface started =====================");

	let mut child = Command::new("bash")
							.arg("huggingface_generate.sh")
							.arg(param)
							.spawn()
							.expect("Failed to launch_huggingface process");
	child.wait().expect("failed to wait on processing launch_huggingface");

	println!("launch_huggingface finished =====================");
}

pub fn read_csv(filepath: &String, has_headers: bool) -> Vec<f64> {
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
pub fn write_csv(filepath: &String, data: Vec<f64>) -> String {
	let file = OpenOptions::new()
			.write(true)
			.create(true)
			.append(true)
			.open(filepath)
			.unwrap();
	let mut writer = Writer::from_writer(file);
	for i in 0 .. data.len() {
		let _res = writer.write_field(data[i].to_string());
		let _res = writer.write_record(None::<&[u8]>);
	}
	let _res = writer.flush();

	filepath.to_string()
}
pub fn similarity_voice_code(audio1 : &String, audio2 : &String) -> io::Result<f64> {
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
    let mut command = Command::new("/root/.virtualenvs/resemblyzer/bin/python")
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
pub fn vst_generate_audio(yaml_path : &String) {
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
pub fn launch_inference_audio(yaml_path : &String) {
	println!("launch_inference started =====================");

	let mut child = Command::new("bash")
							.arg("launch_inference.sh")
							.arg(yaml_path)
							.spawn()
							.expect("Failed to launch_inference process");
	child.wait().expect("failed to wait on processing launch_inference");

	println!("launch_inference finished =====================");
}
pub fn launch_normalizing_audio(yaml_path : &String) {
	println!("launch_normalizing started =====================");

	let mut child = Command::new("bash")
							.arg("launch_normalizing.sh")
							.arg(yaml_path)
							.spawn()
							.expect("Failed to launch_normalizing process");
	child.wait().expect("failed to wait on processing launch_normalizing");

	println!("launch_normalizing finished =====================");
}
pub fn generate_yaml(vst_file_path : &String, file_path : &String, target_path : &String) -> Result<String, io::Error> {
	let yaml_path = format!("{:#?}/VST/{}.yaml", env::current_dir()?.display(), target_path).replace("\"", "");
	let file = File::create(&yaml_path)?;
    let mut file = LineWriter::new(file);
	let vst_file_path = format!("{:#?}/{}", env::current_dir()?.display(), vst_file_path);
	let file_path = format!("{:#?}/{}", env::current_dir()?.display(), file_path);
	let content = format!("{}:\n    ref:\n    - {}\n    ref_spk_name: carolyn\n    src: {}\n    src_spk_name: anusha", target_path, file_path.replace("\"", ""), vst_file_path.replace("\"", ""));
	let _res = file.write_all(content.as_bytes());
	file.flush()?;
	Ok(yaml_path)
}
pub fn denoise_audio(file_path : &String) {
	let mut child = Command::new("bash")
							.arg("deepfilternet_denoise.sh")
							.arg(format!("--log-level error -m DeepFilterNet {}", file_path))
							.spawn()
							.expect("Failed to remove noise");
	child.wait().expect("failed to wait on removing noise");
	println!("Audio file has been denoised.");
}
pub fn replace_audio(template_path : &String, audio_path : &String) {
	let output_path = format!("{}.mp4", template_path);
	// ffmpeg -i video.mp4 -i audio.wav -c:v libx264 -c:a aac -map 0:v:0 -map 1:a:0 output.mp4
	let mut command = Command::new("ffmpeg")
							.arg("-nostdin")
							.arg("-loglevel")
							.arg("error")
							.arg("-i")
							.arg(&template_path)
							.arg("-i")
							.arg(&audio_path)
							.arg("-c:v")
							.arg("libx264")
							.arg("-map")
							.arg("0:v:0")
							.arg("-map")
							.arg("1:a:0")
							.arg(&output_path)
							.spawn()
							.expect("Failed to convert using ffmpeg");
	command.wait().expect("Failed to convert using ffmpeg");

	let _res = fs::remove_file(&template_path);
	let _res = fs::rename(&output_path, &template_path);
	println!("Audio file has been replaced.");
}
pub fn extract_audio(file_path : &String) -> String {
	// extract audio files from .webm using ffmpeg
	let output_path = format!("{}.wav", file_path);
	let mut command = Command::new("ffmpeg")
							.arg("-nostdin")
							.arg("-loglevel")
							.arg("error")
							.arg("-i")
							.arg(file_path)
							.arg(&output_path)
							.spawn()
							.expect("Failed to convert using ffmpeg");
	command.wait().expect("Failed to convert using ffmpeg");
	println!("Audio file has been extracted.");
	output_path
}
pub fn copy_to_wav(file_path : &String) -> String {
	// conversion audio files into .wav using ffmpeg
	let output_path = format!("{}.wav", file_path);
	let mut command = Command::new("ffmpeg")
							.arg("-nostdin")
							.arg("-loglevel")
							.arg("error")
							.arg("-i")
							.arg(file_path)
                            .arg("-acodec")
                            .arg("pcm_s16le")
                            .arg("-ac")
                            .arg("1")
                            .arg("-ar")
                            .arg("16000")
							.arg(&output_path)
							.spawn()
							.expect("Failed to convert using ffmpeg");
	command.wait().expect("Failed to convert using ffmpeg");
	println!("Audio file has been copied.");
	output_path
}
pub fn convert_to_wav(file_path : &String) {
	// conversion audio files into .wav using ffmpeg
	let output_path = format!("{}.wav", file_path);
	let mut command = Command::new("ffmpeg")
							.arg("-nostdin")
							.arg("-loglevel")
							.arg("error")
							.arg("-i")
							.arg(file_path)
                            .arg("-acodec")
                            .arg("pcm_s16le")
                            .arg("-ac")
                            .arg("1")
                            .arg("-ar")
                            .arg("16000")
							.arg(&output_path)
							.spawn()
							.expect("Failed to convert using ffmpeg");
	command.wait().expect("Failed to convert using ffmpeg");
	let _res = fs::remove_file(file_path);
	let _res = fs::rename(output_path, file_path);
	println!("Audio file has been converted.");
}
pub fn convert_to_mp3(file_path : &String) -> String{
	// conversion audio files into .wav using ffmpeg
	let output_path = format!("{}.mp3", file_path);
	let mut command = Command::new("ffmpeg")
							.arg("-nostdin")
							.arg("-loglevel")
							.arg("error")
							.arg("-i")
							.arg(file_path)
							.arg("-acodec")
							.arg("libmp3lame")
							.arg(&output_path)
							.spawn()
							.expect("Failed to convert using ffmpeg");
	command.wait().expect("Failed to convert using ffmpeg");
	println!("Audio file has been converted.");
	output_path
}
// file_path = "1/0/show_10AlBXJul8JZ5bREZUXBep/1am2bPIgTuCcAfqOY3rQZ1";
// "1/0/show_10AlBXJul8JZ5bREZUXBep/1_0_show_10AlBXJul8JZ5bREZUXBep_1am2bPIgTuCcAfqOY3rQZ1"
pub fn get_libritts_name(path : String) -> String {
    let array = path.split("/");
    let splited_array: Vec<&str> = array.collect();
    format!("{}/{}/{}/{}", splited_array[0], splited_array[1], splited_array[2], format!("{}_{}_{}_{}", splited_array[0], splited_array[1], splited_array[2], splited_array[3]))
}

pub fn extract_audio_batch(total_transcript_array: Vec<PodcastTranscriptInfo>, target_path: &String, file_path : &String) {
	let mut audio_file = hound::WavReader::open(file_path).unwrap();
	let audio_spec = audio_file.spec();
	let raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();
	let interval = audio_spec.sample_rate * audio_spec.channels as u32;

	for i in 0 .. total_transcript_array.len() {
		let transcript_path = format!("../podcasts-dataset/{}_{}.wav", get_libritts_name(target_path.to_string()), i);
		if path_exists(&transcript_path) {
			continue;
		}
		let start_index = (interval as f64 * total_transcript_array[i].start_time) as u32;
		let mut end_index = (interval as f64 * total_transcript_array[i].end_time) as u32;
        if end_index > (raw_samples.len() as u32) {
            end_index = (raw_samples.len() as u32);
        }
		let mut writer = hound::WavWriter::create(&transcript_path, audio_spec).unwrap();
		let mut index = start_index;
		while index < end_index {
			for _k in 0..audio_spec.channels {
				writer.write_sample(raw_samples[index as usize]).unwrap();
			}	
			index = index + audio_spec.channels as u32;
		}
		let file_size = match std::fs::metadata(&transcript_path) {
			Ok(value) => value.len(),
			Err(_e) => 0,
		};
		if file_size < 1000 {
			println!("empty file has been deleted! -> {} bytes", file_size);
			let _res = match std::fs::remove_file(transcript_path) {
				Ok(_value) => {},
				Err(_e) => {}
			};
		}
	}
	let _res = match std::fs::remove_file(file_path) {
		Ok(_value) => {},
		Err(_e) => {}
	};

	println!("audio: {}", total_transcript_array.len());
}
pub fn extract_audio_name(transcript_path : &String, start_time: f64, end_time: f64, file_path : &String) {
	let mut audio_file = hound::WavReader::open(file_path).unwrap();
	let audio_spec = audio_file.spec();
	let raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();
	let interval = audio_spec.sample_rate * audio_spec.channels as u32;
	let start_index = (interval as f64 * start_time) as u32;
	let end_index = (interval as f64 * end_time) as u32;
	
	let mut writer = hound::WavWriter::create(transcript_path, audio_spec).unwrap();
	let mut index = start_index;
	while index < end_index {
		for _k in 0..=audio_spec.channels - 1 {
			writer.write_sample(raw_samples[index as usize]).unwrap();
		}	
		index = index + audio_spec.channels as u32;
	}
}
pub fn remove_silence_audio(file_path : &String) {
	let mut audio_file = match hound::WavReader::open(file_path) {
        Ok(audio_file) => audio_file,
        Err(_err) => return,
    };

	let audio_spec = audio_file.spec();
	let raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();
	let interval = audio_spec.sample_rate * audio_spec.channels as u32;
	let mut samples: Vec<i32> = Vec::new(); 

	let sensitive_silences : Vec<Silence> = get_silences_audio(file_path);
	let mut silences : Vec<Silence> = Vec::new(); 
	for k in 0..sensitive_silences.len() {
		if (sensitive_silences[k].end_time - sensitive_silences[k].start_time) > 0.5 {
			let mut silence : Silence = sensitive_silences[k];
			silence.start_time += 0.35;
			silence.start_index += (interval as f64 * 0.35) as usize;
			silences.push(silence);
			continue;
		}
		if (sensitive_silences[k].end_time - sensitive_silences[k].start_time) > 0.2 {
			silences.push(sensitive_silences[k]);
		}
	}
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
		let mut writer = match hound::WavWriter::create(file_path, audio_spec) {
			Ok(writer) => writer,
			Err(_err) => return,
		};
		let mut k = 0;
		while k < samples.len() {
			for _k in 0..=audio_spec.channels - 1 {
				let _res = writer.write_sample(samples[k as usize]);
			}	
			k += audio_spec.channels as usize;
		}
	} else {
		let mut writer = match hound::WavWriter::create(file_path, audio_spec) {
			Ok(writer) => writer,
			Err(_err) => return,
		};
		for k in 0..=raw_samples.len() - 1 {
			let _res = writer.write_sample(raw_samples[k as usize]);
		}
	}
}

pub fn get_silences_audio(file_path : &String) -> Vec<Silence> {
	let mut silences : Vec<Silence> = Vec::new(); 
	let mut audio_file = match hound::WavReader::open(file_path) {
        Ok(audio_file) => audio_file,
        Err(_err) => return silences,
    };

	let audio_spec = audio_file.spec();
	let raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();
	let interval = audio_spec.sample_rate * audio_spec.channels as u32;
	let mut threshold = -40.0;
	let silence_interval = 0.1;
	let offset = 0.02;
	let mut is_created : bool = false;
	let mut silence : Silence = Silence{start_index : 0, 
										start_time: 0.0, 
										end_index : 0, 
										end_time : 0.0};
	threshold = db_to_amplitude(threshold, audio_spec.bits_per_sample as f64);
	for i in 0..= raw_samples.len() - 1 {
		if (raw_samples[i as usize].abs() as f64) < threshold {
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
			if is_created == true && (silence.end_index as f64 - silence.start_index as f64) > silence_interval * interval as f64 {
				silence.start_time = round_val(silence.start_time) + offset;
				silence.end_time = round_val(silence.end_time) - offset;
				silences.push(silence);
				println!("{}", format!("silences : {} - {}", silence.start_time, silence.end_time));
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
	if is_created == true && (silence.end_index as f64 - silence.start_index as f64) > silence_interval * interval as f64 {
		silence.start_time = round_val(silence.start_time) + offset;
		silence.end_time = round_val(silence.end_time) - offset;
		silences.push(silence);
		println!("{}", format!("silences : {} - {}", silence.start_time, silence.end_time));
	} 
	
	silences
}
fn get_rms_audio(filename : &String) -> f64 {
	let mut reader = match hound::WavReader::open(filename) {
        Ok(reader) => reader,
        Err(_err) => return 0.0,
    };

	let sqr_sum = reader.samples::<i32>()
						.fold(0.0, |sqr_sum, s| {
						let sample = s.unwrap() as f64;
						sqr_sum + sample * sample
						});

	(sqr_sum / reader.len() as f64).sqrt()
}
pub fn adjust_amplitude_audio(template_path : &String, name_path : &String) -> bool {
	let mut audio_file = match hound::WavReader::open(name_path) {
        Ok(audio_file) => audio_file,
        Err(_err) => return false,
    };

	let name_spec = audio_file.spec();
	let mut raw_samples = audio_file.samples::<i32>().into_iter().map(|x| x.unwrap()).collect::<Vec<i32>>();

	let template_rms = get_rms_audio(template_path);
	println!("template_rms : {}", template_rms);
	let name_rms = get_rms_audio(name_path);
	println!("name_rms : {}", name_rms);
	if name_rms == 0.0 {
		return false;
	}

	let ratio = template_rms / name_rms;

	let mut writer = match hound::WavWriter::create(name_path, name_spec) {
        Ok(writer) => writer,
        Err(_err) => return false,
    };
	if raw_samples.len() <= 0 {
		return false;
	}
	for i in 0..raw_samples.len() {
		raw_samples[i as usize] = (raw_samples[i as usize] as f64 * ratio.sqrt()) as i32;
		let _res = writer.write_sample(raw_samples[i as usize]);
	}
	true
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
fn db_to_amplitude(db_value : f64, bits_value : f64) -> f64 {
	let max_value = (bits_value - 1.0).exp2() as f64;
	let value = db_value / 20.0;
	10_f64.powf(value) * max_value
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
