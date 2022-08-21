# GStreamer plugins and test APPs for I-Frame filtering

Repository contains GStreamer plugin along with test APP for filtering out all frames except I-frames out of H.264 video stream

```
c                               # Filter written in C
   |-- gstframefilter.c
   |-- mp4-h264-key.c
fat_sample.mp4                  # Vieo file example
rust                            # Filter written in Rust
   |-- h264_iframe_filter
   |   |-- Cargo.lock
   |   |-- Cargo.toml
   |   |-- src
   |   |   |-- frame_filter.rs
   |   |   |-- main.rs
```

Filter requires GStreamer 1.18+

## Preparing example

For result to be clearly visiable some preparations needed. As an example we will use free video sample reencoded
to have group of pictures set to 10. This will ensure that enoughframes will be shown at the screen

Download sample:
```
wget https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4 -O sample.mp4
```

Prepare file with GOP of 10:
```
ffmpeg -i sample.mp4 -vcodec libx264 -g 10 -acodec aac fat_sample.mp4
```

## Building C APP
```
gcc mp4-h264-key.c gstframefilter.c  -o hwfilter `pkg-config --cflags --libs gstreamer-1.0`
```