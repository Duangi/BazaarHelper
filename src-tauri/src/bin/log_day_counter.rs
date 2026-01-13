use anyhow::{Context, Result};
use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Simple CLI to infer current in-game day from Player.log
#[derive(Parser, Debug)]
#[command(author, version, about = "Infer current day from Player.log", long_about = None)]
struct Args {
    /// Path to Player.log
    #[arg(short, long)]
    log: PathBuf,

    /// Hours per in-game day (default: 6)
    #[arg(short = 'h', long, default_value_t = 6)]
    hours_per_day: u32,

    /// Retro mode: only consider lines after the most recent NetMessageRunInitialized
    #[arg(long, default_value_t = false)]
    retro: bool,
}

struct GameStatus {
    current_day: u32,
    in_pvp: bool,
    hour_count: u32,
    hours_per_day: u32,
}

impl GameStatus {
    fn new(hours_per_day: u32) -> Self {
        Self {
            current_day: 0,
            in_pvp: false,
            hour_count: 0,
            hours_per_day,
        }
    }

    fn handle_line(&mut self, line: &str) {
        // Start a new run -> reset to day 1
        if line.contains("NetMessageRunInitialized") {
            self.current_day = 1;
            self.in_pvp = false;
            self.hour_count = 0;
            return;
        }

        // Mark entering PVP
        if line.contains("to [PVPCombatState]") {
            self.in_pvp = true;
            return;
        }

        // Increment hour on encounter (auxiliary clue)
        if line.contains("to [EncounterState]") || line.contains("to [ShopState]") {
            // avoid overflow; hours are small
            self.hour_count = self.hour_count.saturating_add(1);
            return;
        }

        // Leaving PVP into ChoiceState/LevelUpState means day ends
        if self.in_pvp && line.contains("State changed") &&
            (line.contains("to [ChoiceState]") || line.contains("to [LevelUpState]") || line.contains("to [EndOfRunState]"))
        {
            // If we haven't initialized day yet, set to 1 then increment (safe-guard)
            if self.current_day == 0 {
                self.current_day = 1;
            }
            self.current_day = self.current_day.saturating_add(1);
            self.in_pvp = false;
            self.hour_count = 0;
            println!(">>> 播报：进入第 {} 天 <<<", self.current_day);
            return;
        }

        // Auxiliary rule: if hours_per_day reached then next significant non-PVP event may indicate day end
        if self.hour_count >= self.hours_per_day && line.contains("to [ChoiceState]") {
            if self.current_day == 0 { self.current_day = 1; }
            self.current_day = self.current_day.saturating_add(1);
            self.hour_count = 0;
            println!(">>> 依据小时数判定：进入第 {} 天 <<<", self.current_day);
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let file = File::open(&args.log).with_context(|| format!("Failed to open log file: {:?}", args.log))?;
    let reader = BufReader::new(file);

    // Optionally find the most recent NetMessageRunInitialized
    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        let l = line.unwrap_or_default();
        lines.push(l);
    }

    let start_idx = if args.retro {
        // find last occurrence
        lines.iter().rposition(|l| l.contains("NetMessageRunInitialized")).unwrap_or(0)
    } else {
        0
    };

    let mut status = GameStatus::new(args.hours_per_day);

    for line in &lines[start_idx..] {
        status.handle_line(line);
    }

    if status.current_day == 0 {
        println!("No run initialized found; day unknown (0). Try --retro or provide a full log.");
    } else {
        println!("Final detected day: {}", status.current_day);
    }

    Ok(())
}
