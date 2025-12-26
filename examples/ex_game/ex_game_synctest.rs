mod ex_game;

use clap::Parser;
use ex_game::Game;
use fortress_rollback::{PlayerHandle, SessionBuilder};
use macroquad::prelude::*;
use web_time::{Duration, Instant};

const FPS: f64 = 60.0;

/// returns a window config for macroquad to use
fn window_conf() -> Conf {
    Conf {
        window_title: "Box Game Synctest".to_owned(),
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
    num_players: usize,
    #[arg(short = 'd', long)]
    check_distance: usize,
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

    // create a Fortress Rollback session
    let mut sess = SessionBuilder::new()
        .with_num_players(opt.num_players)
        .with_check_distance(opt.check_distance)
        .with_input_delay(2)
        .unwrap()
        .start_synctest_session()?;

    // Create a new box game
    let mut game = Game::new(opt.num_players);
    game.register_local_handles((0..opt.num_players).map(PlayerHandle::new).collect());

    // time variables for tick rate
    let mut last_update = Instant::now();
    let mut accumulator = Duration::ZERO;
    let fps_delta = 1. / FPS;

    loop {
        // get delta time from last iteration and accumulate it
        let delta = Instant::now().duration_since(last_update);
        accumulator = accumulator.saturating_add(delta);
        last_update = Instant::now();

        // if enough time is accumulated, we run a frame
        while accumulator.as_secs_f64() > fps_delta {
            // decrease accumulator
            accumulator = accumulator.saturating_sub(Duration::from_secs_f64(fps_delta));

            // gather inputs
            for handle_idx in 0..opt.num_players {
                let handle = PlayerHandle::new(handle_idx);
                sess.add_local_input(handle, game.local_input(handle))?;
            }

            match sess.advance_frame() {
                Ok(requests) => game.handle_requests(requests, false),
                Err(e) => return Err(Box::new(e)),
            }
        }

        // render the game state
        game.render();

        // wait for the next loop (macroquads wants it so)
        next_frame().await;
    }
}
