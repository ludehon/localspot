use std::env;

use anyhow::{Context, Result, anyhow};
use librespot::{
    core::{SpotifyUri, authentication::Credentials, config::SessionConfig, session::Session},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::NoOpVolume,
        player::Player,
    },
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

    let spotify_uri = SpotifyUri::from_uri(&track_uri)
        .with_context(|| format!("Failed to parse Spotify URI '{track_uri}'"))?;
    if !spotify_uri.is_playable() {
        return Err(anyhow!(
            "URI '{track_uri}' is not a playable track or episode"
        ));
    }

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
    session_config.client_id = client_id;
    let session = Session::new(session_config, None);

    session
        .connect(credentials, false)
        .await
        .context("Failed to connect to Spotify")?;

    let backend = audio_backend::find(None).ok_or_else(|| {
        anyhow!("No audio backend found. Enable an audio backend feature in librespot.")
    })?;
    let audio_format = AudioFormat::default();
    let player_config = PlayerConfig::default();

    let player = Player::new(player_config, session, Box::new(NoOpVolume), move || {
        backend(None, audio_format)
    });

    println!("Playing '{track_uri}'...");
    player.load(spotify_uri, true, 0);
    player.await_end_of_track().await;
    println!("Playback finished.");

    Ok(())
}
