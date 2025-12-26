mod ex_game;

use clap::Parser;
use ex_game::{FortressConfig, Game};
use fortress_rollback::{
    DesyncDetection, PlayerHandle, PlayerType, SaveMode, SessionBuilder, SessionState,
    UdpNonBlockingSocket,
};
// Note: Import SyncHealth when implementing termination logic (see comment block below)
// use fortress_rollback::SyncHealth;
use macroquad::prelude::*;
use std::net::SocketAddr;
use web_time::{Duration, Instant};

const FPS: f64 = 60.0;

/// returns a window config for macroquad to use
fn window_conf() -> Conf {
    Conf {
        window_title: "Box Game P2P".to_owned(),
        window_width: 600,
        window_height: 800,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[derive(Parser)]
struct Opt {
    #[arg(short, long)]
    local_port: u16,
    #[arg(short, long)]
    players: Vec<String>,
    #[arg(short, long)]
    spectators: Vec<SocketAddr>,
}

#[macroquad::main(window_conf)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // configure logging: output Fortress Rollback and example game logs to standard out
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish(),
    )
    .expect("setting up tracing subscriber failed");
    // forward logs from log crate to the tracing subscriber (allows seeing macroquad logs)
    tracing_log::LogTracer::init()?;

    // read cmd line arguments
    let opt = Opt::parse();
    let num_players = opt.players.len();
    assert!(num_players > 0);

    // create a Fortress Rollback session
    let mut sess_build = SessionBuilder::<FortressConfig>::new()
        .with_num_players(num_players)
        // (optional) customize desync detection interval (default: 60 frames)
        .with_desync_detection_mode(DesyncDetection::On { interval: 100 })
        // (optional) set expected update frequency
        .with_fps(FPS as usize)?
        // (optional) customize prediction window, which is how many frames ahead Fortress Rollback predicts.
        // Or set the prediction window to 0 to use lockstep netcode instead (i.e. no rollbacks).
        .with_max_prediction_window(8)
        // (optional) set input delay for the local player
        .with_input_delay(2).unwrap()
        // (optional) by default, Fortress Rollback will ask you to save the game state every frame. If your
        // saving of game state takes much longer than advancing the game state N times, you can
        // improve performance by turning sparse saving mode on (N == average number of predictions
        // Fortress Rollback must make, which is determined by prediction window, FPS and latency to clients).
        .with_save_mode(SaveMode::EveryFrame);

    // add players
    for (i, player_addr) in opt.players.iter().enumerate() {
        // local player
        if player_addr == "localhost" {
            sess_build = sess_build.add_player(PlayerType::Local, PlayerHandle::new(i))?;
        } else {
            // remote players
            let remote_addr: SocketAddr = player_addr.parse()?;
            sess_build =
                sess_build.add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(i))?;
        }
    }

    // optionally, add spectators
    for (i, spec_addr) in opt.spectators.iter().enumerate() {
        sess_build = sess_build.add_player(
            PlayerType::Spectator(*spec_addr),
            PlayerHandle::new(num_players + i),
        )?;
    }

    // start the Fortress Rollback session
    let socket = UdpNonBlockingSocket::bind_to_port(opt.local_port)?;
    let mut sess = sess_build.start_p2p_session(socket)?;

    // Create a new box game
    let mut game = Game::new(num_players);
    game.register_local_handles(sess.local_player_handles());

    // time variables for tick rate
    let mut last_update = Instant::now();
    let mut accumulator = Duration::ZERO;

    loop {
        // communicate, receive and send packets
        sess.poll_remote_clients();

        // print Fortress Rollback events
        for event in sess.events() {
            info!("Event: {:?}", event);
        }

        // -----------------------------------------------------------------------
        // Session Termination Pattern (Example)
        // -----------------------------------------------------------------------
        // If you need to terminate the session (e.g., game over), DO NOT use
        // confirmed_frame() alone! Peers may be at different frames.
        //
        // CORRECT termination pattern:
        //
        //   let target_frames = Frame::new(1000);  // Your end condition
        //   if sess.confirmed_frame() >= target_frames {
        //       // Check sync health before terminating
        //       for handle in sess.remote_player_handles() {
        //           match sess.sync_health(handle) {
        //               Some(SyncHealth::InSync) => {
        //                   // Safe to exit - checksums match
        //               }
        //               Some(SyncHealth::DesyncDetected { frame, .. }) => {
        //                   panic!("Desync detected at frame {}!", frame);
        //               }
        //               Some(SyncHealth::Pending) | None => {
        //                   // Keep polling until we have checksum verification
        //                   continue;
        //               }
        //           }
        //       }
        //       break;  // All peers verified, safe to terminate
        //   }
        //
        // See docs/user-guide.md "Common Pitfalls" for more details.
        // -----------------------------------------------------------------------

        // this is to keep ticks between clients synchronized.
        // if a client is ahead, it will run frames slightly slower to allow catching up
        let mut fps_delta = 1. / FPS;
        if sess.frames_ahead() > 0 {
            fps_delta *= 1.1;
        }

        // get delta time from last iteration and accumulate it
        let delta = Instant::now().duration_since(last_update);
        accumulator = accumulator.saturating_add(delta);
        last_update = Instant::now();

        // if enough time is accumulated, we run a frame
        while accumulator.as_secs_f64() > fps_delta {
            // decrease accumulator
            accumulator = accumulator.saturating_sub(Duration::from_secs_f64(fps_delta));

            // frames are only happening if the sessions are synchronized
            if sess.current_state() == SessionState::Running {
                // add input for all local  players
                for handle in sess.local_player_handles() {
                    sess.add_local_input(handle, game.local_input(handle))?;
                }

                match sess.advance_frame() {
                    Ok(requests) => game.handle_requests(requests, sess.in_lockstep_mode()),
                    Err(e) => return Err(Box::new(e)),
                }
            }
        }

        // render the game state
        game.render();

        // wait for the next loop (macroquad wants it so)
        next_frame().await;
    }
}
