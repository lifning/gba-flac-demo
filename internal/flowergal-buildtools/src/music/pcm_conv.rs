// Copyright (C) 2021 lifning, licensed under the GNU Affero General Public License version 3.

use flowergal_proj_config::sound_info;
use build_const::ConstWriter;
use itertools::Itertools;
use rayon::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MP3_DIR: &str = "../../assets/mp3";
const LOSSY_WAV: &str = "../../external/lossywav/lossyWAV";
const FLAC_MOD: &str = "../../external/flac/src/flac/flac";

trait CommandSuccess {
    fn actually_run(&mut self) -> Result<Output, Box<dyn Error>>;
}

impl CommandSuccess for Command {
    fn actually_run(&mut self) -> Result<Output, Box<dyn Error>> {
        let output = self.output()?;
        if !output.status.success() {
            let dir = self.get_current_dir()
                .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_owned()))
                .map(|pb| pb.to_string_lossy().to_string())
                .unwrap_or_default();
            let mut msg = format!(
                "Command failed:\n[{}]$ {:?} {:?}",
                dir,
                self.get_program(),
                self.get_args().into_iter().format(" "),
            );
            let stdout = String::from_utf8_lossy(&output.stdout).trim_end().to_owned();
            if !stdout.is_empty() {
                msg = format!("{}\n-- cmd stdout --\n{}", msg, stdout);
            }
            let stderr = String::from_utf8_lossy(&output.stderr).trim_end().to_owned();
            if !stderr.is_empty() {
                msg = format!("{}\n-- cmd stderr --\n{}", msg, stderr);
            }
            Err(msg.into())
        } else {
            Ok(output)
        }
    }
}

pub fn convert_songs_and_sfx() -> Result<(), Box<dyn Error>> {
    let mut bc_out = ConstWriter::for_build("sound_data_bc")?.finish_dependencies();

    Command::new("make")
        .arg("-j4")
        .current_dir(Path::new(LOSSY_WAV).parent().unwrap())
        .actually_run()?;

    if !Path::new(MP3_DIR).exists() {
        std::fs::create_dir_all(MP3_DIR)?;
    }

    Command::new("youtube-dl")
        .args(&["--ignore-config", "--download-archive", "downloaded-ids.txt", "32ZTjFW2RYo"])
        .current_dir(Path::new(MP3_DIR))
        .actually_run()?;

    if !Path::new(FLAC_MOD).exists() {
        let flac_dir = Path::new(FLAC_MOD).parent().unwrap().parent().unwrap().parent().unwrap();
        if !flac_dir.join("Makefile").exists() {
            Command::new("sh")
                .arg("autogen.sh")
                .current_dir(flac_dir)
                .actually_run()?;
            Command::new("sh")
                .args(&["configure", "-C"])
                .current_dir(flac_dir)
                .actually_run()?;
        }
        Command::new("make")
            .arg("-j4")
            .current_dir(flac_dir)
            .actually_run()?;
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

    Ok(())
}

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
    Command::new("ffmpeg")
        .args(&["-loglevel", "quiet", "-y", "-i"])
        .arg(mp3_path)
        .args(&["-ac", "1", "-ar"])
        .arg(format!("{}", sound_info::SAMPLE_RATE))
        .arg(&wav_path)
        .actually_run()?;

    if let Err(e) = Command::new(LOSSY_WAV)
        .arg(&wav_path)
        .args(&["--force", "--silent", "--shaping", "fixed", "--outdir"])
        .arg(lossywav_path.parent().unwrap())
        .actually_run() {
        std::mem::forget(temp_dir);
        return Err(e);
    }
    std::fs::remove_file(wav_path)?;

    if let Err(e) = Command::new(FLAC_MOD)
        .args(&["--silent", "-f", "-8", "--escape-coding", "--rice-partition-order=8", "--max-lpc-order=4",])
        .arg(format!("--blocksize={}", sound_info::PLAYBUF_SIZE))
        .arg(format!("--output-name={}", flac_path.to_string_lossy()))
        .arg(&lossywav_path)
        .actually_run() {
        std::mem::forget(temp_dir);
        return Err(e);
    }
    std::fs::remove_file(lossywav_path)?;

    Ok(flac_path)
}
