use gstreamer::prelude::*;
use std::io::Write;

use anyhow::Error;

fn is_uri_udp(uri: String) -> bool {
    return true;
}

fn play_video(u: String) -> Result<(), Error> {
    // Initialize GStreamer
    gstreamer::init()?;

    // Build the pipeline
    let uri = u;
        // "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";
    let pipe = "
    udpsrc port=5005 caps = \"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string)H264, payload=(int)96\" 
    ! rtph264depay 
    ! decodebin 
    ! autovideosink
    ";
    let pipeline = gstreamer::parse_launch(&format!("{}", pipe))?;

    // Start playing
    let res = pipeline.set_state(gstreamer::State::Playing)?;
    let is_live = res == gstreamer::StateChangeSuccess::NoPreroll;

    let main_loop = gstreamer::glib::MainLoop::new(None, false);
    let main_loop_clone = main_loop.clone();
    let pipeline_weak = pipeline.downgrade();
    let bus = pipeline.get_bus().expect("Pipeline has no bus");
    bus.add_watch(move |_, msg| {
        let pipeline = match pipeline_weak.upgrade() {
            Some(pipeline) => pipeline,
            None => return gstreamer::glib::Continue(true),
        };
        let main_loop = &main_loop_clone;
        match msg.view() {
            gstreamer::MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.get_src().map(|s| s.get_path_string()),
                    err.get_error(),
                    err.get_debug()
                );
                let _ = pipeline.set_state(gstreamer::State::Ready);
                main_loop.quit();
            }
            gstreamer::MessageView::Eos(..) => {
                // end-of-stream
                println!("End of stream!");
                let _ = pipeline.set_state(gstreamer::State::Ready);
                main_loop.quit();
            }
            gstreamer::MessageView::Buffering(buffering) => {
                // If the stream is live, we do not care about buffering
                if is_live {
                    return gstreamer::glib::Continue(true);
                }

                let percent = buffering.get_percent();
                print!("Buffering ({}%)\r", percent);
                match std::io::stdout().flush() {
                    Ok(_) => {}
                    Err(err) => eprintln!("Failed: {}", err),
                };

                // Wait until buffering is complete before start/resume playing
                if percent < 100 {
                    let _ = pipeline.set_state(gstreamer::State::Paused);
                } else {
                    let _ = pipeline.set_state(gstreamer::State::Playing);
                }
            }
            gstreamer::MessageView::ClockLost(_) => {
                // Get a new clock
                let _ = pipeline.set_state(gstreamer::State::Paused);
                let _ = pipeline.set_state(gstreamer::State::Playing);
            }
            _ => (),
        }
        gstreamer::glib::Continue(true)
    })
    .expect("Failed to add bus watch");

    main_loop.run();

    bus.remove_watch()?;
    pipeline.set_state(gstreamer::State::Null)?;

    Ok(())
}


fn main() {
    let mut line = String::new();
    println!("Enter URI to connect to: ");
    std::io::stdin().read_line(&mut line).unwrap();
    println!("Attempting to connect to {}...", line);

    match play_video(line){
        Ok(_) => {}
        Err(err) => eprintln!("Failed: {}", err)
    }

}
