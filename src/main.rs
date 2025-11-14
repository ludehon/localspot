use std::env;
use anyhow::{Context, Result};
use librespot::{
    core::{SpotifyUri, authentication::Credentials, config::SessionConfig, session::Session},
    metadata::audio::{AudioFileFormat, AudioItem},
};
use librespot_oauth::OAuthClientBuilder;

const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8898/login";

#[tokio::main]
async fn main() -> Result<()> {
    let client_id = env::var("SPOTIFY_CLIENT_ID")
        .context("SPOTIFY_CLIENT_ID must be set in the environment")?;
    let _client_secret = env::var("SPOTIFY_CLIENT_SECRET")
        .context("SPOTIFY_CLIENT_SECRET must be set in the environment")?;

    let redirect_uri =
        env::var("SPOTIFY_REDIRECT_URI").unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());

    let track_uri = env::args()
        .nth(1)
        .or_else(|| env::var("SPOTIFY_TRACK_URI").ok())
        .context(
            "Provide a Spotify track URI as the first CLI argument or set SPOTIFY_TRACK_URI",
        )?;

    // OAuth flow
    println!("Starting OAuth flow. Your browser may open automatically.");
    let oauth_client = OAuthClientBuilder::new(&client_id, &redirect_uri, vec!["streaming"])
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

    // Connect to Spotify
    session
        .connect(credentials, false)
        .await
        .context("Failed to connect to Spotify")?;

    // Spotify URI processing
    let spotify_uri = SpotifyUri::from_uri(&track_uri)
        .with_context(|| format!("'{track_uri}' is not a valid Spotify URI"))?;

    // Fetch audio item
    println!("Fetching audio item for {track_uri} ...");
    let audio_item = AudioItem::get_file(&session, spotify_uri)
        .await
        .with_context(|| format!("Failed to fetch audio metadata for '{track_uri}'"))?;
    println!("Resolved audio item: {} {}", audio_item.name, audio_item.uri);
    println!("Duration: {} ms", audio_item.duration_ms);
    println!("Available audio files:");
    for (format, file_id) in audio_item.files.iter() {
        println!("  {:?} -> {}", format, file_id);
    }

    // Find the best available format
    let bytes_per_second = stream_data_rate(AudioFileFormat::OGG_VORBIS_320);
    println!(
        "Approximate data rate for {:?}: {} bytes/s",
        AudioFileFormat::OGG_VORBIS_320,
        bytes_per_second
    );


    Ok(())
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