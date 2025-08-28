use crate::builtin_words;
use crate::function;
use console::{self, style};
use std::collections::HashSet;
use std::io::{self, Write};

pub fn find_remaining_words(
    acceptable_words: &[String],
    guess_history: &[String],
    state_history: &[[char; 5]],
) -> Vec<String> {
    let mut remaining_words: HashSet<String> = acceptable_words.iter().cloned().collect();

    for (guess, state) in guess_history.iter().zip(state_history.iter()) {
        let mut new_remaining_words = HashSet::new();
        for word in &remaining_words {
            if function::color_state(guess, word) == *state {
                new_remaining_words.insert(word.clone());
            }
        }
        remaining_words = new_remaining_words;
    }

    let mut sorted_words: Vec<String> = remaining_words.into_iter().collect();
    sorted_words.sort();
    sorted_words
}

pub fn print_remaining_words(
    acceptable_words: &[String],
    guess_history: &[String],
    state_history: &[[char; 5]],
) {
    let sorted_words = find_remaining_words(acceptable_words, guess_history, state_history);
    println!("-------------------");
    println!("Possible answers ({}):", sorted_words.len());
    if sorted_words.len() <= 50 {
        println!(
            "{}",
            sorted_words
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        );
    } else {
        println!("Too many to display. (Showing first 50)");
        println!(
            "{}",
            sorted_words
                .iter()
                .take(50)
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        );
    }
}

pub fn print_top_recommendations(
    acceptable_words: &[String],
    guess_history: &[String],
    state_history: &[[char; 5]],
) {
    println!("rec start");
    let remaining_words = find_remaining_words(acceptable_words, guess_history, state_history);

    if remaining_words.len() <= 1 {
        println!(
            "No recommendations needed. Remaining words: {}",
            remaining_words.len()
        );
        return;
    }

    let mut scores: Vec<(String, f64)> = Vec::new();

    let search_set: Vec<String> = if remaining_words.len() <= 500 {
        remaining_words.clone()
    } else {
        acceptable_words.to_vec()
    };

    for guess in &search_set {
        let mut partition_sizes: std::collections::HashMap<[char; 5], usize> =
            std::collections::HashMap::new();
        for answer in &remaining_words {
            let state = function::color_state(guess, answer);
            *partition_sizes.entry(state).or_insert(0) += 1;
        }

        let mut score = 0.0;
        let total_size = remaining_words.len() as f64;
        for size in partition_sizes.values() {
            let p = *size as f64 / total_size;
            if p > 0.0 {
                score += p * p.log2();
            }
        }

        scores.push((guess.clone(), -score));
    }

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("-------------------");
    println!("Top 5 recommended words:");
    for (i, (word, score)) in scores.iter().take(5).enumerate() {
        println!("{}. {} (Score: {:.2})", i + 1, word.to_uppercase(), score);
    }
}

pub fn solver_main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", style("Welcome to Wordle Solver!").bold().green());
    println!("Please enter your guess and the resulting color state after each turn.");
    println!("Example: 'crane GGYRR' (G: Green, Y: Yellow, R: Red/Grey)");
    println!(
        "Type 'rec' for a recommendation, 'left' to see remaining words, 'win' if you won, or 'quit' to exit."
    );

    let acceptable_words: Vec<String> = builtin_words::ACCEPTABLE
        .iter()
        .map(|&s| s.to_string())
        .collect();

    let mut guess_history: Vec<String> = Vec::new();
    let mut state_history: Vec<[char; 5]> = Vec::new();

    println!("\n--- Initial Recommendation ---");
    println!(
        "The best initial guess is: {}",
        style("AEROS").bold().green()
    );

    loop {
        println!("\n--- Enter your guess ---");
        print!("[{}] Enter your guess and state: ", guess_history.len() + 1);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed_input = input.trim().to_lowercase();

        match trimmed_input.as_str() {
            "quit" => {
                println!("Exiting solver.");
                break;
            }
            "win" => {
                println!("Congratulations! You won the game.");
                break;
            }
            "rec" => {
                print_top_recommendations(&acceptable_words, &guess_history, &state_history);
                continue;
            }
            "left" => {
                print_remaining_words(&acceptable_words, &guess_history, &state_history);
                continue;
            }
            _ => {
                let parts: Vec<&str> = trimmed_input.split_whitespace().collect();
                if parts.len() != 2 {
                    println!(
                        "{}",
                        style("Invalid input format. Please use 'guess state'.").red()
                    );
                    continue;
                }

                let guess = parts[0].to_lowercase();
                let state_str = parts[1].to_uppercase();

                if guess.len() != 5 || state_str.len() != 5 {
                    println!(
                        "{}",
                        style("Guess and state must be 5 letters/characters long.").red()
                    );
                    continue;
                }

                let state: [char; 5] = match state_str.chars().collect::<Vec<char>>().try_into() {
                    Ok(s) => s,
                    Err(_) => {
                        println!(
                            "{}",
                            style("Invalid state format. Use only 'G', 'Y', 'R'.").red()
                        );
                        continue;
                    }
                };

                if !acceptable_words.contains(&guess) {
                    println!(
                        "{}",
                        style("Warning: This guess is not in the acceptable word list.").yellow()
                    );
                }

                guess_history.push(guess);
                state_history.push(state);

                let remaining =
                    find_remaining_words(&acceptable_words, &guess_history, &state_history);

                if remaining.len() == 1 {
                    println!("\n{}", style("Found the answer! The word is:").green());
                    println!("{}", style(&remaining[0].to_uppercase()).bold().green());
                    break;
                }

                if remaining.is_empty() {
                    println!(
                        "\n{}",
                        style(
                            "No possible words found with these inputs. Please check your entries!"
                        )
                        .red()
                    );
                    break;
                }

                println!("\n{} possible words remain.", remaining.len());
                print_top_recommendations(&acceptable_words, &guess_history, &state_history);
            }
        }
    }

    Ok(())
}
