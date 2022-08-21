mod imp {
    use gstreamer as gst;
    use gst::glib;
    use gst::prelude::*;
    use gst::subclass::prelude::*;

    use once_cell::sync::Lazy;


    static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
        gst::DebugCategory::new(
            "frame_filter",
            gst::DebugColorFlags::empty(),
            Some("Rust I-frame filter"),
        )
    });


    // Struct containing all the element data
    pub struct FrameFilter {
        srcpad: gst::Pad,
        sinkpad: gst::Pad,
        frame_counter: std::sync::Mutex<u32>,
    }

    impl FrameFilter {
        fn sink_chain(&self, pad: &gst::Pad, _element: &super::FrameFilter, buffer: gst::Buffer,) -> Result<gst::FlowSuccess, gst::FlowError> {
            let mut frame_counter = self.frame_counter.lock().unwrap();
            *frame_counter += 1;
            if !buffer.flags().contains(gst::BufferFlags::DELTA_UNIT) {
                gst::gst_info!(CAT, obj: pad, "Key frame number {}", frame_counter);
                return self.srcpad.push(buffer);
            }
            Ok(gst::FlowSuccess::Ok)
        }

        fn sink_event(
            &self,
            _pad: &gst::Pad,
            _element: &super::FrameFilter,
            event: gst::Event,
        ) -> bool {
            self.srcpad.push_event(event)
        }

        fn sink_query(
            &self,
            _pad: &gst::Pad,
            _element: &super::FrameFilter,
            query: &mut gst::QueryRef,
        ) -> bool {
            self.srcpad.peer_query(query)
        }

        fn src_event(
            &self,
            _pad: &gst::Pad,
            _element: &super::FrameFilter,
            event: gst::Event,
        ) -> bool {
            self.sinkpad.push_event(event)
        }

        fn src_query(
            &self,
            _pad: &gst::Pad,
            _element: &super::FrameFilter,
            query: &mut gst::QueryRef,
        ) -> bool {
            self.sinkpad.peer_query(query)
        }
    }

    // This trait registers our type with the GObject object system and
    // provides the entry points for creating a new instance and setting
    // up the class data
    #[glib::object_subclass]
    impl ObjectSubclass for FrameFilter {
        const NAME: &'static str = "FrameFilter";
        type Type = super::FrameFilter;
        type ParentType = gst::Element;

        // Called when a new instance is to be created. We need to return an instance
        // of our struct here and also get the class struct passed in case it's needed
        fn with_class(klass: &Self::Class) -> Self {
            let templ = klass.pad_template("sink").unwrap();
            let sinkpad = gst::Pad::builder_with_template(&templ, Some("sink"))
                .chain_function(|pad, parent, buffer| {
                    Self::catch_panic_pad_function(parent, || Err(gst::FlowError::Error),
                    |identity, element| identity.sink_chain(pad, element, buffer),)
                })
                .event_function(|pad, parent, event| {
                    Self::catch_panic_pad_function(parent, || false,
                    |identity, element| identity.sink_event(pad, element, event),)
                })
                .query_function(|pad, parent, query| {
                    Self::catch_panic_pad_function(parent, || false,
                    |identity, element| identity.sink_query(pad, element, query),)
                })
                .build();

            let templ = klass.pad_template("src").unwrap();
            let srcpad = gst::Pad::builder_with_template(&templ, Some("src"))
                .event_function(|pad, parent, event| {
                    Self::catch_panic_pad_function(parent, || false,
                    |identity, element| identity.src_event(pad, element, event),)
                })
                .query_function(|pad, parent, query| {
                    Self::catch_panic_pad_function(parent, || false,
                    |identity, element| identity.src_query(pad, element, query),)
                })
                .build();

            let frame_counter = std::sync::Mutex::new(0);
            // Return an instance of our struct and also include our debug category here.
            // The debug category will be used later whenever we need to put something
            // into the debug logs
            Self {
                srcpad,
                sinkpad,
                frame_counter,
            }
        }
    }

    // Implementation of glib::Object virtual methods
    impl ObjectImpl for FrameFilter {
        // Called right after construction of a new instance
        fn constructed(&self, obj: &Self::Type) {
            // Call the parent class' ::constructed() implementation first
            self.parent_constructed(obj);

            // Here we actually add the pads we created in Identity::new() to the
            // element so that GStreamer is aware of their existence.
            obj.add_pad(&self.sinkpad).unwrap();
            obj.add_pad(&self.srcpad).unwrap();
        }
    }

    impl GstObjectImpl for FrameFilter {}

    // Implementation of gst::Element virtual methods
    impl ElementImpl for FrameFilter {
        // Set the element specific metadata. This information is what
        // is visible from gst-inspect-1.0 and can also be programmatically
        // retrieved from the gst::Registry after initial registration
        // without having to load the plugin in memory.
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
                gst::subclass::ElementMetadata::new(
                    "H264 I-Frames filter",
                    "Filter/Video",
                    "Drops all frames from H264 stream except I-Frames",
                    "Bohdan Shubenok <bohdan.shubenok@sigma.software>",
                )
            });
            Some(&*ELEMENT_METADATA)
        }

        // Create and add pad templates for our sink and source pad. These
        // are later used for actually creating the pads and beforehand
        // already provide information to GStreamer about all possible
        // pads that could exist for this type.
        //
        // Actual instances can create pads based on those pad templates
        fn pad_templates() -> &'static [gst::PadTemplate] {
            static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
                // Our element can accept any possible caps on both pads
                let src_pad_template = gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
                    gst::PadPresence::Always,
                    &gst::Caps::builder("video/x-h264").build(),
                )
                .unwrap();

                let sink_pad_template = gst::PadTemplate::new(
                    "sink",
                    gst::PadDirection::Sink,
                    gst::PadPresence::Always,
                    &gst::Caps::builder("video/x-h264").build(),
                )
                .unwrap();

                vec![src_pad_template, sink_pad_template]
            });

            PAD_TEMPLATES.as_ref()
        }

        // Called whenever the state of the element should be changed. This allows for
        // starting up the element, allocating/deallocating resources or shutting down
        // the element again.
        fn change_state(
            &self,
            element: &Self::Type,
            transition: gst::StateChange,
        ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
            // Call the parent class' implementation of ::change_state()
            self.parent_change_state(element, transition)
        }
    }
}

use gst::glib;
use gst::prelude::*;
use gstreamer as gst;

// The public Rust wrapper type for our element
glib::wrapper! {
  pub struct FrameFilter(ObjectSubclass<imp::FrameFilter>) @extends gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "frame_filter",
        gst::Rank::None,
        FrameFilter::static_type(),
    )
}

// Plugin entry point that should register all elements provided by this plugin,
// and everything else that this plugin might provide (e.g. typefinders or device providers).
fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    register(plugin)
}

gst::plugin_define!(
    rstutorial,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    env!("CARGO_PKG_VERSION"),
    "MIT/X11",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    "2022"
);
