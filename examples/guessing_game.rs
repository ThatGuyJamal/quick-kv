use std::io;

use quick_kv::prelude::*;
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct GuessingGame
{
    tries: u32,
    incorrect_guesses: Vec<u32>,
}

fn main() -> anyhow::Result<()>
{
    let mut client = QuickClient::<GuessingGame>::new(ClientConfig::default());

    // First, we create a new game.
    let game = GuessingGame {
        tries: 0,
        incorrect_guesses: vec![],
    };

    // Set the game in the client.
    client.set("game", game.clone())?;

    println!("Welcome to the Number Guessing Game!");
    println!("You have 5 attempts to guess a number between 1 and 50.");

    let secret_number = rand::thread_rng().gen_range(1..50);

    loop {
        let game_state = client.get("game")?.unwrap();
        let remaining_attempts = 5 - game_state.tries;
        if remaining_attempts == 0 {
            println!("Sorry, you have lost the game!");
            println!("The secret number was {}.", secret_number);
            client.delete("game")?;
            break;
        }

        println!("Attempts left: {}", remaining_attempts);
        println!("Please input your guess:");

        let mut guess = String::new();

        io::stdin().read_line(&mut guess).expect("Failed to read line");

        if guess.trim().to_lowercase() == "quit" {
            println!("Thanks for playing!");
            break;
        }

        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Please enter a valid number!");
                continue;
            }
        };

        if guess < 1 || guess > 50 {
            println!("The secret number is between 1 and 50.");
            continue;
        }

        if guess == secret_number {
            println!("Congratulations! You guessed the correct number!");
            client.delete("game")?;
            break;
        } else if guess < secret_number {
            println!("Sorry, your guess was too low. Try a higher number!");
        } else {
            println!("Sorry, your guess was too high. Try a lower number!");
        }

        let updated_game = GuessingGame {
            tries: game_state.tries + 1,
            incorrect_guesses: game_state.incorrect_guesses.clone(),
        };

        client.update("game", updated_game, None)?;
    }

    Ok(())
}
