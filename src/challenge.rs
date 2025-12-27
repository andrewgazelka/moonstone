use std::io::{self, Write};
use std::time::{Duration, Instant};
use rand::Rng;

const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

/// Generate a random string of specified length
fn generate_challenge(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

/// Run the emergency disable challenge
/// User must type continuously for the specified duration
/// Returns true if they complete the challenge
pub fn run_challenge(duration_seconds: u32) -> bool {
    let duration = Duration::from_secs(duration_seconds as u64);

    println!("\n=== MOONSTONE EMERGENCY DISABLE ===\n");
    println!("To disable Moonstone, you must type continuously for {} seconds.", duration_seconds);
    println!("Type each challenge string exactly as shown.");
    println!("If you stop or make too many mistakes, the challenge resets.\n");
    println!("Press ENTER to begin...");

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    let start = Instant::now();
    let mut consecutive_errors = 0;
    const MAX_ERRORS: u32 = 3;

    while start.elapsed() < duration {
        let remaining = duration.saturating_sub(start.elapsed());
        let remaining_secs = remaining.as_secs();

        // Generate a challenge of 8-12 characters
        let challenge_len = rand::thread_rng().gen_range(8..=12);
        let challenge = generate_challenge(challenge_len);

        print!(
            "\r[{:>3}s remaining] Type: {}  > ",
            remaining_secs, challenge
        );
        io::stdout().flush().unwrap();

        input.clear();
        if io::stdin().read_line(&mut input).is_err() {
            return false;
        }

        let typed = input.trim();

        if typed == challenge {
            consecutive_errors = 0;
            println!("  OK");
        } else if typed == "ABORT" || typed == "abort" {
            println!("\nChallenge aborted.");
            return false;
        } else {
            consecutive_errors += 1;
            println!("  WRONG ({}/{})", consecutive_errors, MAX_ERRORS);

            if consecutive_errors >= MAX_ERRORS {
                println!("\nToo many errors. Challenge failed.");
                println!("Wait 60 seconds before trying again.\n");
                return false;
            }
        }
    }

    println!("\n=== CHALLENGE COMPLETE ===");
    println!("Moonstone will be disabled until the next block period.\n");

    true
}

/// A simpler verification for less critical operations
/// Just requires typing a specific phrase
pub fn simple_confirm(phrase: &str) -> bool {
    println!("Type '{}' to confirm:", phrase);
    print!("> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    input.trim() == phrase
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_challenge() {
        let c = generate_challenge(10);
        assert_eq!(c.len(), 10);
        assert!(c.chars().all(|ch| ch.is_ascii_alphanumeric()));
    }
}
