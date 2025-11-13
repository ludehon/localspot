use std::env;
use anyhow::{Context, Result, anyhow};
use librespot::{
    core::{SpotifyId, SpotifyUri, authentication::Credentials, config::SessionConfig, session::Session},
    metadata::{Metadata, Track},
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

    // Extract track ID from URI string
    let track_id_str = track_uri
        .strip_prefix("spotify:track:")
        .or_else(|| track_uri.splitn(2, ":track:").nth(1))
        .ok_or_else(|| anyhow!("Invalid track URI format: '{track_uri}'"))?;

    // Create SpotifyId from the track ID
    let track_id = SpotifyId::from_base62(track_id_str)
        .with_context(|| format!("Failed to parse Spotify track ID '{track_id_str}'"))?;

    // Create SpotifyUri from the SpotifyId
    let spotify_uri = SpotifyUri::Track { id: track_id };
    
    println!("track_uri: {}, track_id: {}, spotify_uri: {}", track_uri, track_id, spotify_uri);

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

    println!("Fetching metadata for URI: {}", spotify_uri);

    let track = Track::get(&session, &spotify_uri)
        .await
        .with_context(|| format!("Failed to fetch metadata for '{track_uri}'"))?;
    println!(
        "Loaded track metadata: '{}' by {}",
        track.name,
        track
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );





    

    // --------------------------- OLD CODE ---------------------------------

    // get the audio decryption key for the track
    // let key = core
    // .block_on(session.audio_key().request(track.id, *file_id))
    // .expect("Cannot get audio key");


    // open encrypted file
    // let mut encrypted_file = core
    //     .block_on(AudioFile::open(&session, *file_id, 320, true))
    //     .unwrap();


    // let mut buffer = Vec::new();
    // let mut read_all: Result<usize> = Ok(0);
    // let fetched = AtomicBool::new(false);
    // threadpool.scoped(|scope| {
    // scope.execute(|| {
    //     // Read the entire encrypted file into the buffer.
    //     read_all = encrypted_file.read_to_end(&mut buffer);
    //     // Set fetched to true to indicate that the file has been read.
    //     fetched.store(true, Ordering::Release);
    // });
    // // Wait until the file has been fully fetched. We poll the atomic
    // // flag and yield to the async runtime briefly to avoid busy-waiting.
    // while !fetched.load(Ordering::Acquire) {
    //     // Sleep for 100 milliseconds before checking again.
    //     core.block_on(async { tokio::time::sleep(Duration::from_millis(100)).await });
    //     // core.turn(Some(Duration::from_millis(100)));
    // }
    // });
    // read_all.expect("Cannot read file stream");
    // let mut decrypted_buffer = Vec::new();
    // AudioDecrypt::new(key, &buffer[..])
    //     .read_to_end(&mut decrypted_buffer)
    //     .expect("Cannot decrypt stream");
    // let fname = format!("{} - {}.ogg", artists_strs.join(", "), track.name);
    // std::fs::write(&fname, &decrypted_buffer[0xa7..]).expect("Cannot write decrypted track");
    // info!("Filename: {}", fname);













    // ------------------------------------------------------------

    // Request the audio decryption key for that track/file
    // let key = core
    //     .block_on(session.audio_key().request(track.id, *file_id))
    //     .expect("Cannot get audio key");

    // let backend = audio_backend::find(None).ok_or_else(|| {
    //     anyhow!("No audio backend found. Enable an audio backend feature in librespot.")
    // })?;
    // let audio_format = AudioFormat::default();
    // let player_config = PlayerConfig::default();

    // let player = Player::new(player_config, session, Box::new(NoOpVolume), move || {
    //     backend(None, audio_format)
    // });

    // println!("Playing '{track_uri}'...");
    // player.load(spotify_uri, true, 0);
    // player.await_end_of_track().await;
    // println!("Playback finished.");

    Ok(())
}
