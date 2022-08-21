/*
    Pipeline for decode mp4 with h.264 inside and for only I-frames
    Show I frames:
      ffprobe -select_streams v -show_frames -show_entries   frame=pict_type -of csv sample.mp4 | grep -n I

    Download sample:
      wget https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4 -O sample.mp4

    Prepare file with GOP of 10:
      ffmpeg -i sample.mp4 -vcodec libx264 -g 10 -acodec aac fat_sample.mp4

    Compile app:
      gcc mp4-h264-key.c gstframefilter.c  -o hwfilter `pkg-config --cflags --libs gstreamer-1.0`
*/

#include <gst/gst.h>
#include <stdio.h>

GST_PLUGIN_STATIC_DECLARE(frame_filter);

typedef struct {
  GstElement *pipeline;

  GstElement *source;

  GstElement *demuxer;
  GstElement *parser;
  GstElement *filter;
  GstElement *decoder;
  GstElement *convert;

  GstElement *sink;
  GstElement *auto_sink;
} PipelineData;

/* This function will be called by the pad-added signal */
static void pad_added_handler (GstElement *src, GstPad *new_pad, PipelineData *data) {

  GstPad *sink_pad = gst_element_get_static_pad (data->parser, "sink");

  GstPadLinkReturn ret;
  GstCaps *new_pad_caps = NULL;
  GstStructure *new_pad_struct = NULL;
  const gchar *new_pad_type = NULL;

  g_print ("Received new pad '%s' from '%s':\n", GST_PAD_NAME (new_pad), GST_ELEMENT_NAME (src));

  /* Check the new pad's type */
  new_pad_caps = gst_pad_get_current_caps (new_pad);
  new_pad_struct = gst_caps_get_structure (new_pad_caps, 0);
  new_pad_type = gst_structure_get_name (new_pad_struct);
  if (!g_str_has_prefix (new_pad_type, "video/x-h264")) {
    goto exit;
  }

  /* If our converter is already linked, we have nothing to do here */
  if (gst_pad_is_linked (sink_pad)) {
    g_print ("We are already linked. Ignoring.\n");
    goto exit;
  }

  /* Attempt the link */
  ret = gst_pad_link (new_pad, sink_pad);
  if (GST_PAD_LINK_FAILED (ret)) {
    g_print ("Type is '%s' but link failed.\n", new_pad_type);
  } else {
    g_print ("Link succeeded (type '%s').\n", new_pad_type);
  }

exit:
  /* Unreference the new pad's caps, if we got them */
  if (new_pad_caps != NULL) {
    gst_caps_unref (new_pad_caps);
  }

  /* Unreference the sink pad */
  gst_object_unref (sink_pad);
}

int main(int argc, char *argv[]) {
  PipelineData data;
  GstBus *bus;
  GstMessage *msg;
  GstStateChangeReturn ret;
  gboolean terminate = FALSE;

  /* Initialize GStreamer */
  gst_init (&argc, &argv);

  if (argc < 2) {
    g_printerr("No input file set\n");
    return -1;
  }

  GST_PLUGIN_STATIC_REGISTER(frame_filter);

  /* Create the elements */
  data.source = gst_element_factory_make ("filesrc", "source");

  data.demuxer = gst_element_factory_make("qtdemux", "demuxer");
  data.parser = gst_element_factory_make("h264parse", "parser");

  data.filter = gst_element_factory_make("frame_filter", "framefilter");

  data.decoder = gst_element_factory_make("avdec_h264", "decoder");
  data.convert = gst_element_factory_make ("videoconvert", "converter");
  data.auto_sink = gst_element_factory_make ("autovideosink", "auto_sink");

  /* Create the empty pipeline */
  data.pipeline = gst_pipeline_new ("h264-filter-pipeline");

  if (!data.pipeline || !data.source || !data.demuxer || !data.parser || !data.filter ||
      !data.decoder  || !data.convert || !data.auto_sink) {
    g_printerr("Not all elements could be created.\n");
    return -1;
  }

  // char uri[256] = {0};
  // snprintf(uri, sizeof(uri), "%s", argv[1]);
  g_object_set(data.source, "location", argv[1], NULL);

  /* Build the pipeline. Note that we are NOT linking the source at this
   * point. We will do it later. */
  gst_bin_add_many (GST_BIN (data.pipeline), data.source, data.demuxer, data.parser, data.filter,
    data.decoder, data.convert, data.auto_sink, NULL);

  if (!gst_element_link_many (data.source, data.demuxer, NULL)) {
    g_printerr ("Elements could not be linked.\n");
    gst_object_unref (data.pipeline);
    return -1;
  }

  if (!gst_element_link_many (data.parser, data.filter, data.decoder, data.convert, data.auto_sink, NULL)) {
    g_printerr ("Elements could not be linked.\n");
    gst_object_unref (data.pipeline);
    return -1;
  }

  /* Connect to the pad-added signal */
  g_signal_connect (data.demuxer, "pad-added", G_CALLBACK (pad_added_handler), &data);

  /* Start playing */
  ret = gst_element_set_state (data.pipeline, GST_STATE_PLAYING);
  if (ret == GST_STATE_CHANGE_FAILURE) {
    g_printerr ("Unable to set the pipeline to the playing state.\n");
    gst_object_unref (data.pipeline);
    return -1;
  }

  /* Listen to the bus */
  bus = gst_element_get_bus (data.pipeline);
  do {
    msg = gst_bus_timed_pop_filtered (bus, GST_CLOCK_TIME_NONE,
        GST_MESSAGE_STATE_CHANGED | GST_MESSAGE_ERROR | GST_MESSAGE_EOS);

    /* Parse message */
    if (msg != NULL) {
      GError *err;
      gchar *debug_info;

      switch (GST_MESSAGE_TYPE (msg)) {
        case GST_MESSAGE_ERROR:
          gst_message_parse_error (msg, &err, &debug_info);
          g_printerr ("Error received from element %s: %s\n", GST_OBJECT_NAME (msg->src), err->message);
          g_printerr ("Debugging information: %s\n", debug_info ? debug_info : "none");
          g_clear_error (&err);
          g_free (debug_info);
          terminate = TRUE;
          break;
        case GST_MESSAGE_EOS:
          g_print ("End-Of-Stream reached.\n");
          terminate = TRUE;
          break;
        case GST_MESSAGE_STATE_CHANGED:
          /* We are only interested in state-changed messages from the pipeline */
          if (GST_MESSAGE_SRC (msg) == GST_OBJECT (data.pipeline)) {
            GstState old_state, new_state, pending_state;
            gst_message_parse_state_changed (msg, &old_state, &new_state, &pending_state);
            g_print ("Pipeline state changed from %s to %s:\n",
                gst_element_state_get_name (old_state), gst_element_state_get_name (new_state));
          }
          break;
        default:
          /* We should not reach here */
          g_printerr ("Unexpected message received.\n");
          break;
      }
      gst_message_unref (msg);
    }
  } while (!terminate);

  /* Free resources */
  gst_object_unref (bus);
  gst_element_set_state (data.pipeline, GST_STATE_NULL);
  gst_object_unref (data.pipeline);
  return 0;
}
