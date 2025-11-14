use std::env;
use anyhow::{Context, Result, anyhow};
use librespot::{
    core::{SpotifyUri, authentication::Credentials, config::SessionConfig, session::Session},
    metadata::audio::AudioItem,
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
    let mut session_config = SessionConfig::default();
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
    println!("Resolved audio item: {}", audio_item.name);
    println!("Duration: {} ms", audio_item.duration_ms);
    // println!("Available audio files:");
    // for (format, file_id) in audio_item.files.iter() {
    //     println!("  {:?} -> {}", format, file_id);
    // }


    Ok(())
}
