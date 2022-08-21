/*
    Pipeline for decode mp4 with h.264 inside and for only I-frames
    Show I frames:
      ffprobe -select_streams v -show_frames -show_entries   frame=pict_type -of csv sample.mp4 | grep -n I

    Download sample:
      wget https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4 -O sample.mp4

    Prepare file with group of pixels of 10:
      ffmpeg -i sample.mp4 -vcodec libx264 -g 10 -acodec aac fat_sample.mp4
*/

use anyhow::Error;
use derive_more::{Display, Error};
use gst::prelude::*;
use gstreamer as gst;

use clap::Parser;

mod frame_filter;

/// Rust GStreamer filter app to drop all frames except I-Frames from H.264 stream
#[derive(Parser)]
struct Args {
   /// Paht to mp4 file with H.264 stream in it
   #[clap(value_parser)]
   path: String,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

fn main() -> Result<(), Error> {

    let args = Args::parse();

    gst::init().expect("gstreamer initialization failed");

    frame_filter::plugin_register_static()?;

    let pipeline = gst::Pipeline::new(Some("h264_filter_pipeline"));
    let src = gst::ElementFactory::make("filesrc", Some("source")).map_err(|_| MissingElement("filesrc"))?;
    let demux = gst::ElementFactory::make("qtdemux", Some("demux")).map_err(|_| MissingElement("qtdemux"))?;
    let parser = gst::ElementFactory::make("h264parse", Some("parser")).map_err(|_| MissingElement("h264parse"))?;
    let filter = gst::ElementFactory::make("frame_filter", Some("framefilter")).map_err(|_| MissingElement("frame_filter"))?;
    let decoder = gst::ElementFactory::make("avdec_h264", Some("decoder")).map_err(|_| MissingElement("avdec_h264"))?;
    let converter = gst::ElementFactory::make("videoconvert", Some("converter")).map_err(|_| MissingElement("videoconvert"))?;
    let auto_sink = gst::ElementFactory::make("autovideosink", Some("auto_sink")).map_err(|_| MissingElement("autovideosink"))?;

    // Tell the filesrc what file to load
    src.set_property("location", args.path);

    pipeline.add_many(&[
        &src, &demux, &parser, &filter, &decoder, &converter, &auto_sink,
    ])?;
    gst::Element::link_many(&[&src, &demux])?;
    gst::Element::link_many(&[&parser, &filter, &decoder, &converter, &auto_sink])?;

    demux.connect_pad_added(move |_, src_pad| {
        let is_h264_video = src_pad.current_caps().and_then(|caps| {
            caps.structure(0).map(|s|s.name().starts_with("video/x-h264"))
            }).unwrap_or(false);

        if is_h264_video {
            let sink_pad = parser.static_pad("sink").expect("demux has no sinkpad");
            src_pad.link(&sink_pad).expect("Failed to link pad");
        }
    });

    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| String::from(s.path_string()))
                        .unwrap_or_else(|| String::from("None")),
                    error: err.error().to_string(),
                    debug: err.debug(),
                    source: err.error(),
                }
                .into());
            }
            MessageView::StateChanged(s) => {
                println!(
                    "State changed from {:?}: {:?} -> {:?} ({:?})",
                    s.src().map(|s| s.path_string()),
                    s.old(),
                    s.current(),
                    s.pending()
                );
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
