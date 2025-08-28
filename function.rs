use crate::builtin_words;
use console::{self, style};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct GameState {
    #[serde(default)]
    pub total_rounds: u32,
    #[serde(default)]
    pub games: Vec<GameRecord>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameRecord {
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub guesses: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub random: Option<bool>,
    pub difficult: Option<bool>,
    pub stats: Option<bool>,
    pub day: Option<u32>,
    pub seed: Option<u64>,
    pub final_set: Option<String>,
    pub acceptable_set: Option<String>,
    pub state: Option<String>,
    pub word: Option<String>,
}
pub fn is_valid(
    guess: &str,
    _diff_mode: bool,
    guess_history: &[String],
    state_history: &[[char; 5]],
    acceptable_words: &[String],
) -> bool {
    let trimmed_guess = guess.trim();
    let lower_guess = trimmed_guess.to_lowercase();

    if trimmed_guess.len() != 5 || !acceptable_words.contains(&lower_guess) {
        return false;
    }

    if !_diff_mode {
        return true;
    }

    let new_guess_chars: Vec<char> = lower_guess.chars().collect();
    let mut new_guess_char_counts = [0; 26];
    for &c in &new_guess_chars {
        if let Some(index) = (c as u8).checked_sub(b'a') {
            new_guess_char_counts[index as usize] += 1;
        }
    }

    for i in 0..guess_history.len() {
        let prev_guess_chars: Vec<char> = guess_history[i].to_lowercase().chars().collect();
        let prev_state = state_history[i];

        for j in 0..5 {
            if prev_state[j] == 'G' && prev_guess_chars[j] != new_guess_chars[j] {
                return false;
            }
        }

        let mut required_yellows = [0; 26];
        for j in 0..5 {
            if prev_state[j] == 'Y' {
                let char_at_yellow_pos = prev_guess_chars[j];
                if let Some(index) = (char_at_yellow_pos as u8).checked_sub(b'a') {
                    required_yellows[index as usize] += 1;
                }
            }
        }

        for j in 0..26 {
            if new_guess_char_counts[j] < required_yellows[j] {
                return false;
            }
        }
    }
    true
}

pub fn color_state(guess: &str, answer: &str) -> [char; 5] {
    let guess_chars: Vec<char> = guess.chars().collect();
    let answer_chars: Vec<char> = answer.chars().collect();
    let mut state = ['R'; 5];

    let mut answer_counts = [0; 26];
    for &c in &answer_chars {
        if let Some(index) = (c.to_ascii_uppercase() as u8).checked_sub(b'A') {
            answer_counts[index as usize] += 1;
        }
    }
    for i in 0..5 {
        if guess_chars[i] == answer_chars[i] {
            state[i] = 'G';
            if let Some(index) = (guess_chars[i].to_ascii_uppercase() as u8).checked_sub(b'A') {
                answer_counts[index as usize] -= 1;
            }
        }
    }
    for i in 0..5 {
        if state[i] != 'G'
            && let Some(index) = (guess_chars[i].to_ascii_uppercase() as u8).checked_sub(b'A')
            && answer_counts[index as usize] > 0
        {
            state[i] = 'Y';
            answer_counts[index as usize] -= 1;
        }
    }
    state
}
pub fn keyboard_state_update(keyboard_state: &mut [char; 26], guess: &str, state: [char; 5]) {
    let guess_chars: Vec<char> = guess.chars().collect();

    for i in 0..5 {
        let current_char = guess_chars[i].to_ascii_uppercase();
        let index = (current_char as u8 - b'A') as usize;

        let current_keyboard_state = keyboard_state[index];

        let new_state = state[i];

        match (current_keyboard_state, new_state) {
            ('G', _) => {}
            (_, 'G') => keyboard_state[index] = 'G',
            ('Y', _) => {}
            (_, 'Y') => keyboard_state[index] = 'Y',
            ('R', _) => {}
            (_, 'R') => keyboard_state[index] = 'R',
            _ => {}
        }
    }
}

pub fn print_result(state: [char; 5], guess: &str) {
    for (i, c) in guess.chars().enumerate() {
        let styled_char = match state[i] {
            'G' => style(c).green(),
            'Y' => style(c).yellow(),
            'R' => style(c).red(),
            _ => style(c).white(),
        };
        print!("{styled_char}");
    }
    print!(" ");
}

pub fn print_keyboard_state(keyboard_state: &[char; 26]) {
    for (i, &c) in keyboard_state.iter().enumerate() {
        let styled_char = match c {
            'G' => style((b'A' + i as u8) as char).green(),
            'Y' => style((b'A' + i as u8) as char).yellow(),
            'R' => style((b'A' + i as u8) as char).red(),
            _ => style((b'A' + i as u8) as char).white(),
        };
        print!("{styled_char}");
    }
    println!();
}
pub fn print_stats(
    is_tty: bool,
    successful_games: u32,
    failed_games: u32,
    total_successful_attempts: u32,
    guess_frequency: &HashMap<String, u32>,
) {
    let played_games = successful_games + failed_games;

    if played_games == 0 {
        if is_tty {
            println!("\n--- game statistic ---");
        }
        return;
    }

    let success_rate: f64 = successful_games as f64 / played_games as f64;
    let avg_attempts: f64 = if successful_games > 0 {
        total_successful_attempts as f64 / successful_games as f64
    } else {
        0.0
    };

    if is_tty {
        println!("\n--- game statistic ---");
        println!(
            "game played : {played_games} | success: {successful_games} | failed: {failed_games}"
        );
        println!("success rate: {:.2}%", success_rate * 100.0);
        println!("average try : {avg_attempts:.2}");
        println!("--- common guess word ---");
    } else {
        println!("{successful_games} {failed_games} {avg_attempts:.2}");
    }

    let mut frequent_guesses: Vec<_> = guess_frequency.iter().collect();
    frequent_guesses.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let top_5_guesses = frequent_guesses.iter().take(5);

    if is_tty {
        for (word, count) in top_5_guesses {
            println!("{word} ({count})");
        }
    } else {
        let stats_line: Vec<String> = top_5_guesses
            .map(|(word, count)| format!("{} {}", word.to_uppercase(), count))
            .collect();
        println!("{}", stats_line.join(" "));
    }
}
pub fn get_answer_for_day(day: u32, seed: u64) -> String {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut words: Vec<&str> = builtin_words::FINAL.to_vec();
    words.shuffle(&mut rng);
    words[(day - 1) as usize].to_string()
}

pub fn load_and_validate_word_sets(
    final_path: &str,
    acceptable_path: &str,
) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {
    let final_content = fs::read_to_string(final_path)?;
    let acceptable_content = fs::read_to_string(acceptable_path)?;

    let mut final_words_vec: Vec<String> = final_content
        .lines()
        .map(|s| s.trim().to_lowercase())
        .collect();
    let final_words_set: HashSet<String> = final_words_vec.iter().cloned().collect();
    if final_words_vec.len() != final_words_set.len() {
        return Err("more than one".into());
    }
    for word in &final_words_vec {
        if word.len() != 5 || !word.chars().all(|c| c.is_alphabetic()) {
            return Err("incorrect".into());
        }
    }
    final_words_vec.sort();

    let mut acceptable_words_vec: Vec<String> = acceptable_content
        .lines()
        .map(|s| s.trim().to_lowercase())
        .collect();
    let acceptable_words_set: HashSet<String> = acceptable_words_vec.iter().cloned().collect();
    if acceptable_words_vec.len() != acceptable_words_set.len() {
        return Err("more than one".into());
    }
    for word in &acceptable_words_vec {
        if word.len() != 5 || !word.chars().all(|c| c.is_alphabetic()) {
            return Err("incorrect".into());
        }
    }
    acceptable_words_vec.sort();

    // check subset
    if !final_words_set.is_subset(&acceptable_words_set) {
        return Err("not subset".into());
    }

    Ok((final_words_vec, acceptable_words_vec))
}
pub fn load_state(path: &str) -> Result<Option<GameState>, Box<dyn Error>> {
    let file_content = fs::read_to_string(path);

    if file_content.is_err() {
        return Ok(None);
    }

    match serde_json::from_str::<GameState>(&file_content?) {
        Ok(loaded_state) => Ok(Some(loaded_state)),
        Err(e) => {
            eprintln!("Error parsing state file: {e}");
            Err("Invalid state file format".into())
        }
    }
}

pub fn save_state(path: &str, state: &GameState) -> Result<(), Box<dyn Error>> {
    let json_string = serde_json::to_string_pretty(state)?;
    fs::write(path, json_string)?;
    Ok(())
}
