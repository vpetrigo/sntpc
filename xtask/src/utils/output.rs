use crate::Colorize;

pub fn print_header(message: &str) {
    println!("{}", message.bright_blue().bold());
}

pub fn print_success(message: &str) {
    println!("{}", message.bright_green().bold());
}

pub fn print_warning(message: &str) {
    println!("{}", message.bright_yellow());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.bright_red().bold());
}

pub fn print_step(action: &str, target: &str) {
    println!("  {} {}", action.bright_blue(), target);
}

pub fn print_step_success(target: &str) {
    println!("  {} {}", "✓".bright_green(), target);
}

pub fn print_step_warning(message: &str) {
    println!("  {}", message.bright_yellow());
}

pub fn print_step_error(message: &str) {
    eprintln!("  {}", message.bright_red());
}
