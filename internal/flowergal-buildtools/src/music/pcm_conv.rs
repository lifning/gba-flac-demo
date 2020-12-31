use flowergal_proj_config::sound_info;
use build_const::ConstWriter;
use rayon::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

const MP3_DIR: &str = "../../assets/mp3";
const LOSSY_WAV: &str = "../../external/lossywav/lossyWAV";
const FLAC_MOD: &str = "../../external/flac/src/flac/flac";

pub fn convert_songs_and_sfx() -> Result<(), Box<dyn Error>> {
    let mut bc_out = ConstWriter::for_build("sound_data_bc")?.finish_dependencies();

    Command::new("make")
        .arg("-j4")
        .current_dir(Path::new(LOSSY_WAV).parent().unwrap())
        .output()?;

    if !Path::new(MP3_DIR).exists() {
        std::fs::create_dir_all(MP3_DIR)?;
    }

    Command::new("youtube-dl")
        .args(&["--ignore-config", "--download-archive", "downloaded-ids.txt", "32ZTjFW2RYo"])
        .current_dir(Path::new(MP3_DIR))
        .output()?;

    if !Path::new(FLAC_MOD).exists() {
        let flac_dir = Path::new(FLAC_MOD).parent().unwrap().parent().unwrap().parent().unwrap();
        if !flac_dir.join("Makefile").exists() {
            Command::new("sh")
                .arg("autogen.sh")
                .current_dir(flac_dir)
                .output()?;
            Command::new("sh")
                .args(&["configure", "-C"])
                .current_dir(flac_dir)
                .output()?;
        }
        Command::new("make")
            .arg("-j4")
            .current_dir(flac_dir)
            .output()?;
    }

    let vec: Vec<String> = sound_info::SONG_FILES
        .par_iter()
        .map(|mp3_name| {
            let mp3_path = Path::new(MP3_DIR).join(mp3_name);
            let flac_path = convert_mp3_to_flac(mp3_path).unwrap();
            format!(
                "Sound::Flac(include_bytes_align_as!(u32, \"{}\"))",
                flac_path.to_string_lossy()
            )
        })
        .collect();
    let vec_ref: Vec<&str> = vec.iter().map(String::as_str).collect();
    bc_out.add_array_raw("MUSIC_DATA", "Sound", &vec_ref);
/*
    let vec: Vec<String> = sound_info::SFX_FILES
        .par_iter()
        .map(|sfx_name| {
            let sfx_path = Path::new(SFX_DIR).join(sfx_name);
            let pcm8_path = convert_mp3_to_pcm8(sfx_path).unwrap();
            format!(
                "Sound::RawPcm8(include_bytes_align_as!(u32, \"{}\"))",
                pcm8_path.to_string_lossy()
            )
        })
        .collect();
    let vec_ref: Vec<&str> = vec.iter().map(String::as_str).collect();
    bc_out.add_array_raw("SFX_DATA", "Sound", &vec_ref);
    bc_out.finish();
 */
    Ok(())
}

/*
fn convert_mp3_to_pcm8(mp3_path: impl AsRef<Path>) -> Result<PathBuf, Box<dyn Error>> {
    let mp3_path = mp3_path.as_ref();

    let mut pcm8_path =
        Path::new(&std::env::var("OUT_DIR")?).join(mp3_path.file_name().ok_or("uh oh")?);
    pcm8_path.set_extension("pcm8");
    if pcm8_path.is_file() {
        return Ok(pcm8_path);
    }

    println!("cargo:rerun-if-changed={}", mp3_path.to_string_lossy());
    let output = Command::new("ffmpeg")
        .args(&["-loglevel", "quiet", "-y", "-i"])
        .arg(mp3_path)
        .args(&["-f", "s8", "-ac", "1", "-ar"])
        .arg(format!("{}", sound_info::SAMPLE_RATE))
        .arg(&pcm8_path)
        .output()?;
    if !output.status.success() {
        return Err(format!("ffmpeg failed on {:?}", mp3_path).into());
    }

    Ok(pcm8_path)
}
*/

fn convert_mp3_to_flac(mp3_path: impl AsRef<Path>) -> Result<PathBuf, Box<dyn Error>> {
    let mp3_path = mp3_path.as_ref();

    let temp_dir = tempfile::tempdir()?;
    let wav_path = temp_dir.path().join("temp_flac_conv").with_extension("wav");
    let lossywav_path = wav_path.with_extension("lossy.wav");

    let flac_path =
        Path::new(&std::env::var("OUT_DIR")?)
            .join(mp3_path.file_name().ok_or("uh oh")?)
            .with_extension("flac");
    if flac_path.is_file() {
        return Ok(flac_path);
    }

    println!("cargo:rerun-if-changed={}", mp3_path.to_string_lossy());
    let output = Command::new("ffmpeg")
        .args(&["-loglevel", "quiet", "-y", "-i"])
        .arg(mp3_path)
        .args(&["-ac", "1", "-ar"])
        .arg(format!("{}", sound_info::SAMPLE_RATE))
        .arg(&wav_path)
        .output()?;
    if !output.status.success() {
        return Err(format!("ffmpeg failed on {:?}", mp3_path).into());
    }

    let output = Command::new(LOSSY_WAV)
        .arg(&wav_path)
        .args(&["--force", "--silent", "--shaping", "fixed", "--outdir"])
        .arg(lossywav_path.parent().unwrap())
        .output()?;
    if !output.status.success() {
        std::mem::forget(temp_dir);
        return Err(format!("{} failed on {:?}", LOSSY_WAV, wav_path).into());
    }
    std::fs::remove_file(wav_path)?;


    let output = Command::new(FLAC_MOD)
        .args(&["--silent", "-f", "-8", "--escape-coding", "--rice-partition-order=8", "--max-lpc-order=4",])
        .arg(format!("--blocksize={}", sound_info::PLAYBUF_SIZE))
        .arg(format!("--output-name={}", flac_path.to_string_lossy()))
        .arg(&lossywav_path)
        .output()?;
    /*
    let output = Command::new("java")
        .current_dir("../../external/FLAC-library-Java/src")
        .arg("io.nayuki.flac.app.EncodeWavToFlac")
        .arg(&lossywav_path)
        .arg(&flac_path)
        .output()?;
    */
    if !output.status.success() {
        std::mem::forget(temp_dir);
        return Err(format!("flac failed on {:?}", lossywav_path).into());
    }
    std::fs::remove_file(lossywav_path)?;

    Ok(flac_path)
}
