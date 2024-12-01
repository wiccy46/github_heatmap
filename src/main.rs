use git2::Repository;
use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc, Weekday};
use crossterm::{
    execute,
    style::{Color, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};
use std::collections::HashMap;
use std::env;
use std::io::{stdout, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();
    let repo_path = if args.len() > 1 {
        &args[1]
    } else {
        "."
    };

    // Optional year argument
    let specified_year = if args.len() > 2 {
        args[2].parse::<i32>().ok()
    } else {
        None
    };

    // Use the specified year or default to the current year
    let current_year = Utc::now().year();
    let year = specified_year.unwrap_or(current_year);

    // Open the specified Git repository
    let repo = Repository::open(repo_path)?;

    // Initialize a revwalk to iterate over commits
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    // Collect commit dates
    let mut commit_counts: HashMap<NaiveDate, u32> = HashMap::new();
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let timestamp = commit.time().seconds();
        let datetime = NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap();
        let date = datetime.date();
        if date.year() == year {
            *commit_counts.entry(date).or_insert(0) += 1;
        }
    }

    // Generate dates for the specified year
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    // Adjust start_date to the nearest previous Sunday
    let mut adjusted_start_date = start_date;
    while adjusted_start_date.weekday() != Weekday::Sun {
        adjusted_start_date -= chrono::Duration::days(1);
    }

    // Adjust end_date to the nearest next Saturday
    let mut adjusted_end_date = end_date;
    while adjusted_end_date.weekday() != Weekday::Sat {
        adjusted_end_date += chrono::Duration::days(1);
    }

    // Collect dates into weeks and keep track of month changes
    let mut weeks: Vec<Vec<Option<NaiveDate>>> = Vec::new();
    let mut week_months: Vec<u32> = Vec::new(); // Month of the first day in each week
    let mut date = adjusted_start_date;

    while date <= adjusted_end_date {
        let mut week = Vec::new();
        let mut week_month = 0;
        for _ in 0..7 {
            if date >= start_date && date <= end_date {
                week.push(Some(date));
                if week_month == 0 {
                    // Set the week_month to the month of the first valid date in the week
                    week_month = date.month();
                }
            } else {
                week.push(None);
            }
            date += chrono::Duration::days(1);
        }
        weeks.push(week);
        week_months.push(week_month);
    }

    // Prepare month labels aligned to the first day of the month
    let mut month_labels: Vec<String> = vec!["  ".to_string(); weeks.len()];
    let mut last_month = 0;
    for i in 0..weeks.len() {
        let week_month = week_months[i];
        if week_month != last_month && week_month != 0 {
            // Place the month label at this position
            month_labels[i] = format!("{:<2}", week_month);
            last_month = week_month;
        }
    }

    // Clear the terminal
    execute!(stdout(), Clear(ClearType::All))?;

    // Print month labels
    print!("     "); // Align with weekday labels
    for (i, label) in month_labels.iter().enumerate() {
        print!("{}", label);
        if i < weeks.len() - 1 {
            // Check if month changes after this week
            if week_months[i] != week_months[i + 1] && week_months[i + 1] != 0 {
                print!("|"); // Separator between months
            } else {
                print!(" "); // Space between weeks
            }
        }
    }
    println!();

    // Weekday labels
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    // Display the heatmap
    for (weekday_index, weekday_label) in weekdays.iter().enumerate() {
        // Print weekday label with spacing
        print!("{:<4}", weekday_label); // Width 4 to include space

        // Print each day's cell with gaps between days
        for i in 0..weeks.len() {
            if let Some(Some(date)) = weeks[i].get(weekday_index) {
                let count = *commit_counts.get(&date).unwrap_or(&0);

                // Adjusted color scheme using shades of green
                let color = match count {
                    0 => Color::DarkGrey,
                    1 => Color::Green,
                    2..=3 => Color::DarkGreen,
                    4..=5 => Color::Rgb { r: 0, g: 255, b: 0 }, // Bright Green
                    _ => Color::White, // For very high commit counts
                };

                let styled_cell = "  ".on(color); // Two spaces with background color
                execute!(stdout(), PrintStyledContent(styled_cell))?;
            } else {
                // No date (outside the specified year)
                print!("  "); // Two spaces
            }

            if i < weeks.len() - 1 {
                // Check if month changes after this week
                if week_months[i] != week_months[i + 1] && week_months[i + 1] != 0 {
                    print!("|"); // Separator between months
                } else {
                    print!(" "); // Space between weeks
                }
            }
        }
        println!();

        // Add a blank line to create a gap between weekdays
        println!();
    }

    Ok(())
}

