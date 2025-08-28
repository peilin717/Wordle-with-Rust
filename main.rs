use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::io::{self, Write};

mod builtin_words;
mod function;
mod solver;

/// The main function for the Wordle game, implement your own logic here
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).peekable();
    let mut config_path: Option<String> = None;
    while let Some(arg) = args.peek() {
        if *arg == "-c" || *arg == "--config" {
            args.next();
            config_path = Some(args.next().expect("error: missing config file path"));
        } else {
            args.next();
        }
    }
    //second parse args
    let mut args = std::env::args().skip(1).peekable();
    let mut word_arg: Option<String> = None;
    let mut day_arg: Option<u32> = None;
    let mut seed_arg: Option<u64> = None;
    let mut final_set_path: Option<String> = None;
    let mut acceptable_set_path: Option<String> = None;
    let mut state_path: Option<String> = None;
    let default_seed: u64 = 1;
    let mut random_mode = false;
    let mut _diff_mode: bool = false;
    let mut stats_mode = false;
    let mut solver_mode = false;
    let mut solver_only = false;

    let mut successful_games: u32 = 0;
    let mut failed_games: u32 = 0;
    let mut total_successful_attempts: u32 = 0;
    let mut guess_frequency: HashMap<String, u32> = HashMap::new();
    let mut played_answers: Vec<String> = Vec::new();
    let mut games: Vec<function::GameRecord> = Vec::new();

    while let Some(arg) = args.next() {
        if arg == "-w" || arg == "--word" {
            if random_mode {
                panic!("error: cannot use --word with --random");
            }
            word_arg = Some(args.next().expect("error"));
        } else if arg == "-r" || arg == "--random" {
            if word_arg.is_some() {
                panic!("error: cannot use --random with --word");
            }
            random_mode = true;
        } else if arg == "-d" || arg == "--day" {
            day_arg = Some(args.next().expect("input days").parse().expect("error"));
        } else if arg == "-s" || arg == "--seed" {
            seed_arg = Some(args.next().expect("input seed").parse().expect("error"));
        } else if arg == "-D" || arg == "--difficult" {
            _diff_mode = true;
        } else if arg == "-t" || arg == "--stats" {
            stats_mode = true;
        } else if arg == "-f" || arg == "--final-set" {
            final_set_path = Some(args.next().expect("error"));
        } else if arg == "-a" || arg == "--acceptable-set" {
            acceptable_set_path = Some(args.next().expect("error"));
        } else if arg == "-S" || arg == "--state" {
            state_path = Some(args.next().expect("error: missing state file path"));
        } else if arg == "-v" || arg == "--solver" {
            solver_mode = true;
        } else if arg == "-so" || arg == "--solver-only" {
            solver_only = true;
        }
    }

    if solver_only {
        let _ = solver::solver_main();
        return Ok(());
    }

    if word_arg.is_some() && (day_arg.is_some() || seed_arg.is_some() || random_mode) {
        eprintln!("Error: Cannot use --word with --day, --seed, or --random.");
        std::process::exit(1);
    }

    let mut config: function::Config = function::Config::default();
    if let Some(path) = &config_path
        && let Ok(file_content) = std::fs::read_to_string(path)
    {
        match serde_json::from_str::<function::Config>(&file_content) {
            Ok(loaded_config) => {
                config = loaded_config;
            }
            Err(e) => {
                eprintln!("Error parsing config file: {e}");
                std::process::exit(1);
            }
        }
    }

    // second parse args
    while let Some(arg) = args.next() {
        if arg == "-w" || arg == "--word" {
            word_arg = Some(args.next().expect("error"));
        } else if arg == "-r" || arg == "--random" {
            random_mode = true;
        } else if arg == "-d" || arg == "--day" {
            day_arg = Some(args.next().expect("input days").parse().expect("error"));
        } else if arg == "-s" || arg == "--seed" {
            seed_arg = Some(args.next().expect("input seed").parse().expect("error"));
        } else if arg == "-D" || arg == "--difficult" {
            _diff_mode = true;
        } else if arg == "-t" || arg == "--stats" {
            stats_mode = true;
        } else if arg == "-f" || arg == "--final-set" {
            final_set_path = Some(args.next().expect("error"));
        } else if arg == "-a" || arg == "--acceptable-set" {
            acceptable_set_path = Some(args.next().expect("error"));
        } else if arg == "-S" || arg == "--state" {
            state_path = Some(args.next().expect("error: missing state file path"));
        } else if arg == "-v" || arg == "--solver" {
            solver_mode = true;
        }
    }

    if word_arg.is_none() {
        word_arg = config.word;
    }
    if !random_mode {
        random_mode = config.random.unwrap_or(false);
    }
    if day_arg.is_none() {
        day_arg = config.day;
    }
    if seed_arg.is_none() {
        seed_arg = config.seed;
    }
    if !_diff_mode {
        _diff_mode = config.difficult.unwrap_or(false);
    }
    if !stats_mode {
        stats_mode = config.stats.unwrap_or(false);
    }
    if final_set_path.is_none() {
        final_set_path = config.final_set;
    }
    if acceptable_set_path.is_none() {
        acceptable_set_path = config.acceptable_set;
    }
    if state_path.is_none() {
        state_path = config.state;
    }
    let is_tty = atty::is(atty::Stream::Stdout);
    let is_answer_from_cli = word_arg.is_some();
    let mut current_day = day_arg.unwrap_or(1);
    let current_seed = seed_arg.unwrap_or(default_seed);
    if current_day as usize > builtin_words::FINAL.len() {
        eprintln!("error");
        std::process::exit(1);
    }

    let final_words: Vec<String>;
    let acceptable_words: Vec<String>;

    if let Some(f_path) = final_set_path {
        if acceptable_set_path.is_none() {
            eprintln!("must --final-set and --acceptable-set");
            std::process::exit(1);
        }
        let a_path = acceptable_set_path.unwrap();

        (final_words, acceptable_words) = function::load_and_validate_word_sets(&f_path, &a_path)?;
    } else {
        final_words = builtin_words::FINAL
            .iter()
            .map(|&s| s.to_string())
            .collect();
        acceptable_words = builtin_words::ACCEPTABLE
            .iter()
            .map(|&s| s.to_string())
            .collect();
    }

    if let Some(path) = &state_path
        && let Some(loaded_state) = function::load_state(path)?
    {
        games = loaded_state.games;
        successful_games = 0;
        failed_games = 0;
        total_successful_attempts = 0;
        guess_frequency = HashMap::new();

        for record in &games {
            if record
                .guesses
                .last()
                .map(|g| function::color_state(g, &record.answer) == ['G', 'G', 'G', 'G', 'G'])
                .unwrap_or(false)
            {
                successful_games += 1;
                total_successful_attempts += record.guesses.len() as u32;
            } else {
                failed_games += 1;
            }
            for guess in &record.guesses {
                *guess_frequency.entry(guess.to_lowercase()).or_insert(0) += 1;
            }
        }
    }
    loop {
        let answer: String;

        if let Some(word) = &word_arg {
            if !final_words.contains(&word.trim().to_lowercase()) {
                eprintln!("error: answer word must be in final word list");
                std::process::exit(1);
            }
            answer = word.trim().to_lowercase();
        } else if day_arg.is_some() || seed_arg.is_some() {
            answer = function::get_answer_for_day(current_day, current_seed);
        } else if random_mode {
            loop {
                let mut rng = thread_rng();
                let new_answer = builtin_words::FINAL
                    .choose(&mut rng)
                    .expect("error")
                    .to_string();
                if !played_answers.contains(&new_answer) {
                    answer = new_answer;
                    played_answers.push(answer.clone());
                    break;
                }
            }
        } else {
            if is_tty {
                println!("\n please input your answer:");
            }
            let mut input_answer = String::new();
            io::stdin().read_line(&mut input_answer)?;
            if input_answer.trim().is_empty() {
                break;
            }
            answer = input_answer.trim().to_lowercase();
        }

        let mut guess_num = 0;
        let mut keyboard_state = ['X'; 26];
        let mut guess_history: Vec<String> = Vec::new();
        let mut state_history: Vec<[char; 5]> = Vec::new();

        loop {
            if is_tty {
                print!("give me your guess ({} times):", guess_num + 1);
                io::stdout().flush()?;
            }

            let mut guess = String::new();
            if io::stdin().read_line(&mut guess)? == 0 {
                return Ok(());
            }

            let trimmed_guess = guess.trim().to_lowercase();
            if !function::is_valid(
                &trimmed_guess,
                _diff_mode,
                &guess_history,
                &state_history,
                &acceptable_words,
            ) {
                println!("INVALID");
                continue;
            }
            *guess_frequency.entry(trimmed_guess.clone()).or_insert(0) += 1;
            guess_history.push(trimmed_guess.clone());

            let state = function::color_state(&trimmed_guess, &answer);
            state_history.push(state);

            function::keyboard_state_update(&mut keyboard_state, &trimmed_guess, state);
            if is_tty {
                for i in 0..guess_history.len() {
                    let history_guess = &guess_history[i].to_uppercase();
                    let history_state = state_history[i];
                    function::print_result(history_state, history_guess);
                }
                println!();
                function::print_keyboard_state(&keyboard_state);
            } else {
                println!(
                    "{} {}",
                    state.iter().collect::<String>(),
                    keyboard_state.iter().collect::<String>()
                ); //print state 
            }

            guess_num += 1;

            if state == ['G', 'G', 'G', 'G', 'G'] {
                if is_tty {
                    println!("\nYou are right! The answer is {answer}");
                }
                println!("CORRECT {guess_num}");
                successful_games += 1;
                total_successful_attempts += guess_num;
                break;
            }
            if guess_num >= 6 {
                if is_tty {
                    println!("\nYou failed ,the answer is {answer}");
                }
                println!("FAILED {}", answer.to_uppercase());
                failed_games += 1;
                break;
            }
            if solver_mode {
                println!("Solver mode active. ");
                println!("Type 'left' to show remaining words, Type 'rec' to show recommend words");
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_uppercase();
                if input.contains("LEFT") {
                    solver::print_remaining_words(
                        &acceptable_words,
                        &guess_history,
                        &state_history,
                    );
                }
                if input.contains("REC") {
                    solver::print_top_recommendations(
                        &acceptable_words,
                        &guess_history,
                        &state_history,
                    );
                }
            }
        }

        let current_game = function::GameRecord {
            answer: answer.to_uppercase(),
            guesses: guess_history.iter().map(|g| g.to_uppercase()).collect(),
        };
        games.push(current_game);

        if let Some(path) = &state_path {
            let state_to_save = function::GameState {
                total_rounds: games.len() as u32,
                games: games.clone(),
            };
            if let Err(e) = function::save_state(path, &state_to_save) {
                eprintln!("Error saving game state: {e}");
            }
        }
        if stats_mode {
            function::print_stats(
                is_tty,
                successful_games,
                failed_games,
                total_successful_attempts,
                &guess_frequency,
            );
        }

        if is_answer_from_cli {
            break;
        } else {
            current_day += 1;
            if is_tty {
                print!("\nDo you wanna play a new game ? (Y/N) ");
                io::stdout().flush()?;
            }
            let mut continue_choice = String::new();
            if io::stdin().read_line(&mut continue_choice)? == 0 {
                // EOF
                break;
            }
            if continue_choice.trim().to_lowercase() != "y" {
                break;
            }
        }
    }

    Ok(())
}
