use anyhow::{Context, Result};
use librespot::{
    audio::{AudioDecrypt, AudioFile},
    core::{
        SpotifyUri, authentication::Credentials, config::SessionConfig, session::Session,
        spotify_id::SpotifyId,
    },
    metadata::{
        audio::{AudioFileFormat, AudioItem},
        Track, Metadata,
    },
};
use librespot_oauth::OAuthClientBuilder;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};


const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8898/login";

/// Starts an OAuth flow with Spotify and returns an authenticated session.
/// 
/// # Arguments
/// * `client_id` - The Spotify application's client ID.
/// * `redirect_uri` - The redirect URI for OAuth authorization.
/// 
/// # Returns
/// A Result containing the authenticated `Session` on success, or an error otherwise.
async fn create_oauth_session(client_id: &str, redirect_uri: &str) -> Result<Session> {
    println!("Starting OAuth flow. Your browser may open automatically.");
    let oauth_client = OAuthClientBuilder::new(client_id, redirect_uri, vec!["streaming"])
        .open_in_browser()
        .build()
        .context("Failed to construct Spotify OAuth client")?;
    
    let token = oauth_client
        .get_access_token()
        .context("Failed to complete Spotify OAuth flow")?;
    
    println!("OAuth flow completed. Scopes granted: {:?}", token.scopes);
    
    let credentials = Credentials::with_access_token(token.access_token);
    let session_config = SessionConfig::default();
    let session = Session::new(session_config, None);
    
    session
        .connect(credentials, false)
        .await
        .context("Failed to connect to Spotify")?;
    
    Ok(session)
}

fn stream_data_rate(format: AudioFileFormat) -> usize {
    let kbps = match format {
        AudioFileFormat::OGG_VORBIS_96 => 12.,
        AudioFileFormat::OGG_VORBIS_160 => 20.,
        AudioFileFormat::OGG_VORBIS_320 => 40.,
        AudioFileFormat::MP3_256 => 32.,
        AudioFileFormat::MP3_320 => 40.,
        AudioFileFormat::MP3_160 => 20.,
        AudioFileFormat::MP3_96 => 12.,
        AudioFileFormat::MP3_160_ENC => 20.,
        AudioFileFormat::AAC_24 => 3.,
        AudioFileFormat::AAC_48 => 6.,
        AudioFileFormat::AAC_160 => 20.,
        AudioFileFormat::AAC_320 => 40.,
        AudioFileFormat::MP4_128 => 16.,
        AudioFileFormat::OTHER5 => 40.,
        AudioFileFormat::FLAC_FLAC => 112., // assume 900 kbit/s on average
        AudioFileFormat::XHE_AAC_12 => 1.5,
        AudioFileFormat::XHE_AAC_16 => 2.,
        AudioFileFormat::XHE_AAC_24 => 3.,
        AudioFileFormat::FLAC_FLAC_24BIT => 3.,
    };
    let data_rate: f32 = kbps * 1024.;
    data_rate.ceil() as usize
}

/// Downloads a Spotify track.
/// 
/// # Parameters
/// - `session`: Spotify session
/// - `track_uri`: The Spotify track URI
/// - `output_dir`: Directory where the track will be saved
async fn download_track(session: &Session, track_uri: String, output_dir: &Path) -> Result<()> {
    // Spotify URI processing
    let spotify_uri = SpotifyUri::from_uri(&track_uri)
        .with_context(|| format!("'{track_uri}' is not a valid Spotify URI"))?;

    // Fetch track metadata
    // println!("Fetching track metadata for {track_uri} ...");
    let track = Track::get(&session, &spotify_uri)
        .await
        .context("Failed to fetch track metadata")?;
    
    let track_id = SpotifyId::try_from(&spotify_uri).context("Spotify URI must reference a track ID")?;
    
    // Get artist names
    let artists_strs: Vec<String> = track.artists.iter().map(|a| a.name.clone()).collect();
    println!("Track {} by {}", track.name, artists_strs.join(", "));
    
    // Check if file already exists
    let filename = format!("{} - {}.ogg", artists_strs.join(", "), track.name)
        .replace('/', "-")  // Replace forward slashes
        .replace('\\', "-") // Replace backslashes (for Windows compatibility)
        .replace(':', "-")  // Replace colons (invalid on Windows)
        .replace('?', "")   // Remove question marks
        .replace('*', "")   // Remove asterisks
        .replace('"', "'"); // Replace quotes
    let output_path = output_dir.join(&filename);
    
    if output_path.exists() {
        println!("File already exists, skipping download: {}", filename);
        return Ok(());
    }
    
    // Fetch audio item
    // println!("Fetching audio item for {track_uri} ...");
    let audio_item = AudioItem::get_file(&session, spotify_uri.clone())
        .await
        .with_context(|| format!("Failed to fetch audio metadata for '{track_uri}'"))?;
    // println!(
    //     "Resolved audio item: {} {}",
    //     audio_item.name, audio_item.uri
    // );
    // println!("Duration: {} ms", audio_item.duration_ms);
    // println!("Available audio files:");
    // for (format, file_id) in audio_item.files.iter() {
    //     println!("  {:?} -> {}", format, file_id);
    // }
    if audio_item.files.is_empty() {
        eprintln!("Warning: No audio files available for '{}' ({})", 
            audio_item.name, track_uri);
        eprintln!("This track may be unavailable in your region or require Premium.");
        return Ok(()); // Skip this track gracefully
    }

    // println!("Available audio files:");
    // for (format, file_id) in audio_item.files.iter() {
    //     println!("  {:?} -> {}", format, file_id);
    // }


    // get file_id for OGG_VORBIS_320
    let file_id = match audio_item.files.get(&AudioFileFormat::OGG_VORBIS_320) {
        Some(file_id) => file_id,
        None => {
            return Err(anyhow::anyhow!(
                "OGG_VORBIS_320 format not available for '{}'",
                audio_item.name
            ));
        }
    };

    // get bytes_per_second for OGG_VORBIS_320
    let bytes_per_second = stream_data_rate(AudioFileFormat::OGG_VORBIS_320);

    // Open the encrypted file
    let encrypted_file = AudioFile::open(&session, *file_id, bytes_per_second)
        .await
        .context("Unable to load encrypted file")?;
    let is_cached = encrypted_file.is_cached();
    // println!("Encrypted file cached: {is_cached}");
    let _stream_loader_controller = encrypted_file
        .get_stream_loader_controller()
        .context("Stream loader controller unavailable")?;

    // Get key for file_id
    let audio_key = session
        .audio_key()
        .request(track_id, *file_id)
        .await
        .context("Failed to request audio key")?;
    // println!("Audio key: {:?}", audio_key);

    // Decrypt file
    let mut decrypted_file = AudioDecrypt::new(Some(audio_key), encrypted_file);
    // println!("Successfully created decrypted audio file");

    // Read decrypted file into a buffer
    let mut buffer = Vec::new();
    decrypted_file.read_to_end(&mut buffer).context("Failed to read decrypted audio data")?;
    // println!("Read {} bytes of decrypted audio", buffer.len());

    // Write the buffer into a ogg file in the output directory
    std::fs::write(&output_path, &buffer[0xa7..]).expect("Cannot write decrypted track");
    // println!("Saved to: {}", output_path.display());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let client_id = env::var("SPOTIFY_CLIENT_ID")
        .context("SPOTIFY_CLIENT_ID must be set in the environment")?;
    let _client_secret = env::var("SPOTIFY_CLIENT_SECRET")
        .context("SPOTIFY_CLIENT_SECRET must be set in the environment")?;
    let redirect_uri = env::var("SPOTIFY_REDIRECT_URI").unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());

    // Get CSV filename from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        anyhow::bail!("Usage: {} <playlist_csv_file>", args[0]);
    }
    let csv_filename = &args[1];
    
    // Extract the base name without extension for the output directory
    let csv_path = Path::new(csv_filename);
    let dir_name = csv_path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid CSV filename")?;
    
    // Create output directory
    let output_dir = PathBuf::from(dir_name);
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;
    println!("Output directory: {}", output_dir.display());

    // Create OAuth session
    let session = create_oauth_session(&client_id, &redirect_uri).await?;
    
    // Open and read track URIs from CSV file, one per line
    println!("Reading track URIs from {}...", csv_filename);
    let file = File::open(csv_filename)
        .with_context(|| format!("Failed to open CSV file: {}", csv_filename))?;
    let reader = BufReader::new(file);
    
    // Create regex patterns to extract track IDs
    let spotify_uri = Regex::new(r"spotify:track:([[:alnum:]]+)").unwrap();
    let spotify_url = Regex::new(r"open\.spotify\.com/track/([[:alnum:]]+)").unwrap();
    
    // Read all lines into a vector to get the total count
    let all_lines: Vec<(usize, String)> = reader
        .lines()
        .enumerate()
        .filter_map(|(i, line_result)| {
            line_result.ok().map(|line| (i, line))
        })
        .collect();
    
    let total_lines = all_lines.len();
    println!("Found {} lines in CSV file", total_lines);
    
    for (line_number, line) in all_lines {
        let line = line.trim();
        
        // Skip empty lines
        if line.is_empty() {
            continue;
        }
        
        // Extract track ID from the line using regex
        let track_id = if let Some(captures) = spotify_uri.captures(line) {
            captures.get(1).map(|m| m.as_str())
        } else if let Some(captures) = spotify_url.captures(line) {
            captures.get(1).map(|m| m.as_str())
        } else {
            None
        };
        
        // If we couldn't extract a track ID, skip this line
        let track_id = match track_id {
            Some(id) => id,
            None => {
                eprintln!("Warning: Could not extract track ID from line {}: '{}'", line_number + 1, line);
                eprintln!("Continuing with next line...");
                continue;
            }
        };
        
        // Construct the proper Spotify URI
        let track_uri = format!("spotify:track:{}", track_id);
        
        println!("\n--- Processing track {}/{} ({}) ---", line_number + 1, total_lines, track_uri);
        
        // Download the track, but continue on error
        if let Err(e) = download_track(&session, track_uri.clone(), &output_dir).await {
            eprintln!("Error downloading {}: {:?}", track_uri, e);
            eprintln!("Continuing with next track...");
        }
    }
    
    println!("\n--- Finished processing all tracks ---");

    Ok(())
}
