WIP

https://docs.rs/librespot-playback/latest/src/librespot_playback/player.rs.html

let encrypted_file = AudioFile::open(&self.session, file_id, bytes_per_second);

let encrypted_file = match encrypted_file.await {
    Ok(encrypted_file) => encrypted_file,
    Err(e) => {
        error!("Unable to load encrypted file: {e:?}");
        return None;
    }
};

let is_cached = encrypted_file.is_cached();

let stream_loader_controller = encrypted_file.get_stream_loader_controller().ok()?;

// Not all audio files are encrypted. If we can't get a key, try loading the track
// without decryption. If the file was encrypted after all, the decoder will fail
// parsing and bail out, so we should be safe from outputting ear-piercing noise.
let key = match self.session.audio_key().request(track_id, file_id).await {
    Ok(key) => Some(key),
    Err(e) => {
        warn!("Unable to load key, continuing without decryption: {e}");
        None
    }
};

let mut decrypted_file = AudioDecrypt::new(key, encrypted_file);

let is_ogg_vorbis = AudioFiles::is_ogg_vorbis(format);
let (offset, mut normalisation_data) = if is_ogg_vorbis {
    // Spotify stores normalisation data in a custom Ogg packet instead of Vorbis comments.
    let normalisation_data =
        NormalisationData::parse_from_ogg(&mut decrypted_file).ok();
    (SPOTIFY_OGG_HEADER_END, normalisation_data)
} else {
    (0, None)
};

let audio_file = match Subfile::new(
    decrypted_file,
    offset,
    stream_loader_controller.len() as u64,
) {
    Ok(audio_file) => audio_file,
    Err(e) => {
        error!("PlayerTrackLoader::load_track error opening subfile: {e}");
        return None;
    }
};
