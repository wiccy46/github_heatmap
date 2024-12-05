use clap::Parser;
use git2::Repository;
use chrono::{Datelike, NaiveDate, TimeZone, Utc, Weekday};
use crossterm::{
    execute,
    style::{Color, PrintStyledContent, Stylize},
};
use std::collections::HashMap;
use std::io::stdout;

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const EMPTY_LABEL: &str = "  ";
const DAYS_IN_WEEK: usize = 7;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    repo: Option<String>,

    #[arg(short, long)]
    year: Option<i32>
}

fn adjust_start_and_end_dates(start_date: &NaiveDate, end_date: &NaiveDate) -> (NaiveDate, NaiveDate) {
    // Adjust start_date to the nearest previous Sunday
    let mut adjusted_start_date = *start_date;
    while adjusted_start_date.weekday() != Weekday::Sun {
        adjusted_start_date -= chrono::Duration::days(1);
    }

    // Adjust end_date to the nearest next Saturday
    let mut adjusted_end_date = *end_date;
    while adjusted_end_date.weekday() != Weekday::Sat {
        adjusted_end_date += chrono::Duration::days(1);
    }

    return (adjusted_start_date, adjusted_end_date);
}

fn collect_commit_counts(
    repo: &Repository,
    year: i32
) -> Result<HashMap<NaiveDate, u32>, Box<dyn std::error::Error>> {
    // Initialize a revwalk to iterate over commits
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    // Collect commit dates
    let mut commit_counts: HashMap<NaiveDate, u32> = HashMap::new();
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let timestamp = commit.time().seconds();
        let datetime = Utc.timestamp_opt(timestamp, 0).single().ok_or("Invalid timestamp")?;
        let date = datetime.date_naive();
        if date.year() == year {
            *commit_counts.entry(date).or_insert(0) += 1;
        }
    }

    Ok(commit_counts)
}

fn organize_weeks(
    adjusted_start_date: &NaiveDate,
    adjusted_end_date: &NaiveDate,
    start_date: &NaiveDate,
    end_date: &NaiveDate
) -> (Vec<Vec<Option<NaiveDate>>>, Vec<u32>) {

    // Collect dates into weeks and keep track of month changes
    let mut weeks: Vec<Vec<Option<NaiveDate>>> = Vec::new();
    let mut week_months: Vec<u32> = Vec::new(); // Month of the first day in each week
    let mut date = *adjusted_start_date;

    while date <= *adjusted_end_date {
        let mut week = Vec::new();
        let mut week_month = 0;
        for _ in 0..DAYS_IN_WEEK {
            if date >= *start_date && date <= *end_date {
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
    return (weeks, week_months);
}

fn get_commit_color(count: u32) -> Color {
    match count {
        0 => Color::DarkGrey,
        1 => Color::Green,
        2..=3 => Color::DarkGreen,
        4..=5 => Color::Rgb { r: 0, g: 255, b: 0 }, // Bright Green
        _ => Color::White, // For very high commit counts
    }
}

fn print_heatmap(
    weeks: &Vec<Vec<Option<NaiveDate>>>,
    week_months: &Vec<u32>,
    commit_counts: &HashMap<NaiveDate, u32>
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Print month labels
    const MONTH_SEPARATOR: &str = "|";
    print!("     "); // Align with weekday labels
    for (i, label) in month_labels.iter().enumerate() {
        print!("{}", label);
        if i < weeks.len() - 1 {
            // Check if month changes after this week
            if week_months[i] != week_months[i + 1] && week_months[i + 1] != 0 {
                print!("{}", MONTH_SEPARATOR); // Separator between months
            } else {
                print!(" "); // Space between weeks
            }
        }
    }
    println!();

    // Display the heatmap
    for (weekday_index, weekday_label) in WEEKDAYS.iter().enumerate() {
        // Print weekday label with spacing
        print!("{:<4}", weekday_label);

        for i in 0..weeks.len() {
            if let Some(Some(date)) = weeks[i].get(weekday_index) {
                let count = *commit_counts.get(&date).unwrap_or(&0);

                // Adjusted color scheme using shades of green
                let color = get_commit_color(count);

                let styled_cell = EMPTY_LABEL.on(color);
                execute!(stdout(), PrintStyledContent(styled_cell))?;
            } else {
                // No date (outside the specified year)
                let styled_cell = EMPTY_LABEL.on(Color::DarkGrey);
                execute!(stdout(), PrintStyledContent(styled_cell))?;
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


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let repo_path = args.repo.unwrap_or_else(|| ".".to_string());
    let year = args.year.unwrap_or_else(|| Utc::now().date_naive().year());

    let repo = Repository::open(repo_path)?;
    let commit_counts = collect_commit_counts(&repo, year)?;


    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
    let (adjusted_start_date, adjusted_end_date) = 
        adjust_start_and_end_dates(&start_date, &end_date);

    let (weeks, week_months) = 
        organize_weeks(&adjusted_start_date, &adjusted_end_date, &start_date, &end_date);
    
    print_heatmap(&weeks, &week_months, &commit_counts)?;

    Ok(())
}

