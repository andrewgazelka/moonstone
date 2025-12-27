use crate::config::{BlockPeriod, ScheduleConfig};
use chrono::{Local, NaiveTime, Timelike};

pub struct Schedule {
    blocks: Vec<ParsedBlock>,
}

struct ParsedBlock {
    start: NaiveTime,
    end: NaiveTime,
    crosses_midnight: bool,
}

impl Schedule {
    pub fn new(config: &ScheduleConfig) -> Self {
        let blocks = config
            .blocks
            .iter()
            .filter_map(|b| Self::parse_block(b))
            .collect();
        Self { blocks }
    }

    fn parse_block(block: &BlockPeriod) -> Option<ParsedBlock> {
        let start = NaiveTime::parse_from_str(&block.start, "%H:%M").ok()?;
        let end = NaiveTime::parse_from_str(&block.end, "%H:%M").ok()?;
        let crosses_midnight = end < start;
        Some(ParsedBlock {
            start,
            end,
            crosses_midnight,
        })
    }

    /// Returns true if we are currently in a block period
    pub fn is_blocked(&self) -> bool {
        let now = Local::now();
        let current_time = NaiveTime::from_hms_opt(
            now.hour(),
            now.minute(),
            now.second(),
        ).unwrap();

        for block in &self.blocks {
            if block.crosses_midnight {
                // e.g., 17:10 to 03:59
                // Blocked if time >= start OR time <= end
                if current_time >= block.start || current_time <= block.end {
                    return true;
                }
            } else {
                // e.g., 04:00 to 17:00
                // Blocked if start <= time <= end
                if current_time >= block.start && current_time <= block.end {
                    return true;
                }
            }
        }
        false
    }

    /// Returns seconds until the current block ends, or None if not blocked
    pub fn seconds_until_unblock(&self) -> Option<u64> {
        if !self.is_blocked() {
            return None;
        }

        let now = Local::now();
        let current_time = NaiveTime::from_hms_opt(
            now.hour(),
            now.minute(),
            now.second(),
        ).unwrap();

        let mut min_seconds = u64::MAX;

        for block in &self.blocks {
            let in_this_block = if block.crosses_midnight {
                current_time >= block.start || current_time <= block.end
            } else {
                current_time >= block.start && current_time <= block.end
            };

            if in_this_block {
                let seconds_to_end = if block.crosses_midnight && current_time >= block.start {
                    // We're in the first part (after start, before midnight)
                    // Need to wait until end time tomorrow
                    let to_midnight = 86400 - current_time.num_seconds_from_midnight();
                    let from_midnight = block.end.num_seconds_from_midnight();
                    (to_midnight + from_midnight) as u64
                } else if block.crosses_midnight {
                    // We're in the second part (after midnight, before end)
                    (block.end.num_seconds_from_midnight() - current_time.num_seconds_from_midnight()) as u64
                } else {
                    // Normal block, just time until end
                    (block.end.num_seconds_from_midnight() - current_time.num_seconds_from_midnight()) as u64
                };

                min_seconds = min_seconds.min(seconds_to_end);
            }
        }

        if min_seconds == u64::MAX {
            None
        } else {
            Some(min_seconds)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_parsing() {
        let config = ScheduleConfig {
            blocks: vec![
                BlockPeriod {
                    start: "04:00".to_string(),
                    end: "17:00".to_string(),
                },
            ],
        };
        let schedule = Schedule::new(&config);
        assert_eq!(schedule.blocks.len(), 1);
    }

    #[test]
    fn test_crosses_midnight() {
        let config = ScheduleConfig {
            blocks: vec![
                BlockPeriod {
                    start: "22:00".to_string(),
                    end: "06:00".to_string(),
                },
            ],
        };
        let schedule = Schedule::new(&config);
        assert!(schedule.blocks[0].crosses_midnight);
    }
}
