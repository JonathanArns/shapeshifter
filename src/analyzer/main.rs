#![feature(test, generic_const_exprs, label_break_value, async_closure)]

use tokio::task;
use reqwest;

use shapeshifter::minimax::analyzer_search::fixed_depth_full_width_search;
use shapeshifter::bitboard::Bitboard;

fn main() {
    shapeshifter::init();

    // let args = Args::parse();

    let body = reqwest::blocking::get(format!("https://engine.battlesnake.com/games/{}", "59099713-b8bc-47f6-a63e-ea62cd0dafa8").as_str())
        .unwrap()
        .json::<shapeshifter::api::GameState>()
        .unwrap();

    let last_frame = &body["LastFrame"];
    let last_turn = last_frame["Turn"].as_i64().expect("Missing Turn") as i32;
    let mut current_turn = args.search_starting_turn.unwrap_or(last_turn - 1);

    loop {
        let current_frame = get_frame_for_turn(&args.game_id, current_turn).unwrap();
        let wire_game = frame_to_game(&current_frame, &body["Game"], &args.you_name);

        if wire_game.is_ok() {
            break;
        }
        println!("You were not alive at turn {current_turn} moving backwards");

        current_turn -= 1;

        if current_turn < 0 {
            panic!("Something is wrong we made it past the end of the game");
        }
    }

    let last_living_turn = current_turn;

    println!("Ending Turn {}", &last_frame["Turn"]);
    println!("Last Living Turn {last_living_turn}");
}
