#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include <gst/gst.h>

#define GST_TYPE_FRAMEFILTER (gst_frame_filter_get_type())
G_DECLARE_FINAL_TYPE (GstFrameFilter, gst_frame_filter, GST, FRAMEFILTER, GstElement)
struct _GstFrameFilter
{
  GstElement element;
  GstPad *sinkpad, *srcpad;
};

GST_DEBUG_CATEGORY_STATIC(gst_frame_filter_debug);
#define GST_CAT_DEFAULT gst_frame_filter_debug

// The capabilities of the inputs and outputs
static GstStaticPadTemplate
  sink_factory = GST_STATIC_PAD_TEMPLATE(
    "sink",
    GST_PAD_SINK,
    GST_PAD_ALWAYS,
    GST_STATIC_CAPS("video/x-h264")),

  src_factory = GST_STATIC_PAD_TEMPLATE(
    "src",
    GST_PAD_SRC,
    GST_PAD_ALWAYS,
    GST_STATIC_CAPS("video/x-h264"));

#define gst_frame_filter_parent_class parent_class
G_DEFINE_TYPE(GstFrameFilter, gst_frame_filter, GST_TYPE_ELEMENT);

GST_ELEMENT_REGISTER_DEFINE(frame_filter, "frame_filter", GST_RANK_NONE, GST_TYPE_FRAMEFILTER);

static GstFlowReturn gst_frame_filter_chain(GstPad *pad, GstObject *parent, GstBuffer *buf);

/* initialize the framefilter's class */
static void gst_frame_filter_class_init(GstFrameFilterClass *klass)
{
  GObjectClass *gobject_class;
  GstElementClass *gstelement_class;

  gobject_class = (GObjectClass *)klass;
  gstelement_class = (GstElementClass *)klass;

  gst_element_class_set_details_simple(gstelement_class,
                                       "FrameFilter",
                                       "FIXME:Generic",
                                       "FIXME:Generic Template Element", " <<user@hostname.org>>");

  gst_element_class_add_pad_template(gstelement_class, gst_static_pad_template_get(&src_factory));
  gst_element_class_add_pad_template(gstelement_class, gst_static_pad_template_get(&sink_factory));
}

/* initialize the new element
 * instantiate pads and add them to element
 * set pad callback functions
 * initialize instance structure
 */
static void gst_frame_filter_init(GstFrameFilter *filter)
{

  filter->sinkpad = gst_pad_new_from_static_template(&sink_factory, "sink");
  gst_pad_set_chain_function(filter->sinkpad, GST_DEBUG_FUNCPTR(gst_frame_filter_chain));
  gst_element_add_pad(GST_ELEMENT(filter), filter->sinkpad);

  filter->srcpad = gst_pad_new_from_static_template(&src_factory, "src");
  gst_element_add_pad(GST_ELEMENT(filter), filter->srcpad);

  // Set pad to proxy caps, so that all caps-related events and queries are proxied down- or upstream
  // to the other side of the element automatically.
  GST_PAD_SET_PROXY_CAPS(filter->srcpad);
  GST_PAD_SET_PROXY_CAPS(filter->sinkpad);

}

static GstFlowReturn gst_frame_filter_chain(GstPad *pad, GstObject *parent, GstBuffer *buf) {
  static guint chain_called = 0;

  GstFrameFilter *filter = GST_FRAMEFILTER(parent);
  chain_called++;

  if (!GST_BUFFER_FLAG_IS_SET(buf, GST_BUFFER_FLAG_DELTA_UNIT)) {
    g_print("Key frame number %u\n", chain_called);
    /* just push out the incoming buffer without touching it */
    return gst_pad_push(filter->srcpad, buf);
  }

  return GST_FLOW_OK;
}

/* entry point to initialize the plug-in
 * initialize the plug-in itself register the element factories and other features
 */
static gboolean framefilter_init(GstPlugin *framefilter)
{
  GST_DEBUG_CATEGORY_INIT(gst_frame_filter_debug, "frame_filter", 0, "H264 I frame filter");
  return GST_ELEMENT_REGISTER(frame_filter, framefilter);
}

/* PACKAGE: this is usually set by meson depending on some _INIT macro
 * in meson.build and then written into and defined in config.h, but we can
 * just set it ourselves here in case someone doesn't use meson to
 * compile this code. GST_PLUGIN_DEFINE needs PACKAGE to be defined.
 */
#ifndef PACKAGE
#define PACKAGE "h264Iframefilter"
#endif

/* gstreamer looks for this structure to register framefilters
 *
 * exchange the string 'Template framefilter' with your framefilter description
 */
GST_PLUGIN_DEFINE(GST_VERSION_MAJOR,
                  GST_VERSION_MINOR,
                  frame_filter,
                  "frame_filter",
                  framefilter_init,
                  "1", "Proprietary", "H264 I-frame filter", "GST_PACKAGE_ORIGIN")