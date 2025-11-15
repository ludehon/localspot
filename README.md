
# LocalSpot
localspot allows you to save your Spotify playlists into local files, for education purpose only.

## Features
- Saves a playlist into a folder containing ogg files
- Skip existing saved songs
- Parse input file to retrieve Spotify track id
## Requirements
- Spotify developper app with a client id/secret and a redirect URI to http://127.0.0.1:8898/login
- Rust 2024 or newer
- Librespot v0.8.0
*Developped with rustc 1.91.1 and librespot v0.8.0*
## Usage
1. Export your playlist into a file (you can use  [Exportify](https://exportify.app/))
2. Cargo build
3. Export environment variables SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET
4. Launch the script : ./localspot your_playlist_file.csv

Input file should contain valid track ids like :
* spotify:track:7F1lHcpnTycnimQrCxDFI5
* https://open.spotify.com/track/7F1lHcpnTycnimQrCxDFI5
* spotify:track:7F1lHcpnTycnimQrCxDFI5,"Hot","Hot","INNA",2010,217036,71,false,foobar,2025-02-27T18:39:32Z,"europop","Roton Music",0.816,0.94,7,-6.079,0,0.28,0.0798,0.444,0.431,0.596,128,4