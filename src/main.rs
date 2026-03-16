mod display;
mod event;
mod interactive;
mod pipe;
mod pty;
mod term;

fn print_usage() {
    println!("ttail — tail with scroll\n");
    println!("Usage:");
    println!("  ttail <command> [args...]   wrap a command in a pty");
    println!("  command | ttail             tail piped output\n");
    println!("Controls:");
    println!("  Tab      toggle expanded scroll view");
    println!("  j/k      scroll up/down (expanded)");
    println!("  q        quit");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let is_stdin_pipe = unsafe { libc::isatty(0) == 0 };

    if args.first().map(|a| a == "-h" || a == "--help").unwrap_or(false) {
        print_usage();
        std::process::exit(0);
    }

    if is_stdin_pipe {
        // Pipe mode: command | ttail
        pipe::run_pipe_mode();
    } else if !args.is_empty() {
        // PTY wrapper mode: ttail <command> [args...]
        pty::run_pty_mode(&args[0], &args[1..]);
    } else {
        print_usage();
        std::process::exit(1);
    }
}
