use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use clap::{Arg, ArgAction, Command};
use colored::*;
use flexi_logger::{detailed_format, Duplicate, FileSpec, Logger};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use log::{error, warn};
use walkdir::{DirEntry, WalkDir};

use std::{
    env,
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    os::windows::prelude::MetadataExt,
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant},
};

struct Config {
    no_hidden_flag: bool,
    performance_flag: bool,
    stats_flag: bool,
    count_flag: bool,
    depth_flag: u32,
    pattern: String,
    pattern_ac: AhoCorasick,
    extensions: Vec<String>,
}

impl Config {
    fn new(
        no_hidden_flag: bool,
        performance_flag: bool,
        stats_flag: bool,
        count_flag: bool,
        depth_flag: u32,
        pattern: &Vec<&str>,
        pattern_ac: AhoCorasick,
        extensions: Vec<&String>,
    ) -> Self {
        let pattern = pattern[0].to_string();
        let extensions = extensions.into_iter().map(|e| e.to_string()).collect();

        Self {
            no_hidden_flag,
            performance_flag,
            stats_flag,
            count_flag,
            depth_flag,
            pattern,
            pattern_ac,
            extensions,
        }
    }
}

fn main() {
    // don`t lock stdout, otherwise unable to handle ctrl-c
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout);

    // handle Ctrl+C
    ctrlc::set_handler(move || {
        println!(
            "{} {} {} {}",
            "Received Ctrl-C!".bold().red(),
            "🤬",
            "Exit program!".bold().red(),
            "☠",
        );
        process::exit(0)
    })
    .expect("Error setting Ctrl-C handler");

    // get config dir
    let config_dir = check_create_config_dir().unwrap_or_else(|err| {
        error!("Unable to find or create a config directory: {err}");
        process::exit(1);
    });

    // initialize the logger
    let _logger = Logger::try_with_str("info") // log warn and error
        .unwrap()
        .format_for_files(detailed_format) // use timestamp for every log
        .log_to_file(
            FileSpec::default()
                .directory(&config_dir)
                .suppress_timestamp(),
        ) // change directory for logs, no timestamps in the filename
        .append() // use only one logfile
        .duplicate_to_stderr(Duplicate::Info) // print infos, warnings and errors also to the console
        .start()
        .unwrap();

    // handle arguments
    let matches = sf().get_matches();
    let mut no_hidden_flag = matches.get_flag("no-hidden");
    let mut performance_flag = matches.get_flag("performance");
    let mut stats_flag = matches.get_flag("stats");
    let mut count_flag = matches.get_flag("count");
    let mut case_insensitive_flag = matches.get_flag("case-insensitive");
    let override_flag = matches.get_flag("override");

    // set default search depth
    let mut depth_flag = 250;
    if let Some(d) = matches.get_one::<String>("depth") {
        match d.parse() {
            Ok(depth) => depth_flag = depth,
            Err(err) => {
                error!("Expected an integer for the search depth: {err}");
                process::exit(1);
            }
        }
    }

    // if override flag is set -> reset everything to default values
    if override_flag {
        no_hidden_flag = false;
        performance_flag = false;
        stats_flag = false;
        count_flag = false;
        depth_flag = 250;
        case_insensitive_flag = false;
    }

    if let Some(args) = matches
        .get_many::<String>("args")
        .map(|a| a.collect::<Vec<_>>())
    {
        // get search pattern from arguments
        let pattern = vec![args[0].as_str()];
        // store search pattern in aho-corasick builder
        // handle case-insensitive flag
        let pattern_ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(case_insensitive_flag)
            .build(&pattern);

        // get search path from arguments
        let path = Path::new(&args[1]).to_path_buf();

        // get possible file extensions for filtering
        let mut extensions = Vec::new();
        if let Some(mut ext) = matches
            .get_many::<String>("extension")
            .map(|a| a.collect::<Vec<_>>())
        {
            extensions.append(&mut ext);
        }

        // construct Config
        let config = Config::new(
            no_hidden_flag,
            performance_flag,
            stats_flag,
            count_flag,
            depth_flag,
            &pattern,
            pattern_ac,
            extensions,
        );

        // start search
        search(&mut handle, &path, &config);

        // empty bufwriter
        handle
            .flush()
            .unwrap_or_else(|err| error!("Error flushing writer: {err}"));
    } else {
        // handle commands
        match matches.subcommand() {
            Some(("log", _)) => {
                if let Ok(logs) = show_log_file(&config_dir) {
                    println!("{}", "Available logs:".bold().yellow());
                    println!("{}", logs);
                } else {
                    error!("Unable to read logs");
                    process::exit(1);
                }
            }
            _ => {
                unreachable!();
            }
        }
    }
}

// build cli
fn sf() -> Command {
    Command::new("mg")
        .bin_name("mg")
        .before_help(format!(
            "{}\n{}",
            "Mini Grep".bold().truecolor(250, 0, 104),
            "Leann Phydon <leann.phydon@gmail.com>".italic().dimmed()
        ))
        .about("Simple pattern search in files")
        .before_long_help(format!(
            "{}\n{}",
            "Mini Grep".bold().truecolor(250, 0, 104),
            "Leann Phydon <leann.phydon@gmail.com>".italic().dimmed()
        ))
        .long_about(format!(
            "{}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n\n{}",
            "Simple pattern search in files",
            "- colourful output and search indicating spinner by default ",
            "- filter by file-extension",
            "- exclude hidden files",
            "- show search statistics at the end",
            "- accepts \'.\' as current directory",
            "- search case insensitive",
            "- no regex search",
            "Note: every set filter slows down the search".truecolor(250, 0, 104)
        ))
        // TODO update version
        .version("1.1.0")
        .author("Leann Phydon <leann.phydon@gmail.com>")
        .arg_required_else_help(true)
        .arg(
            Arg::new("args")
                .help("Add a search pattern and a path")
                .action(ArgAction::Set)
                .num_args(2)
                .value_names(["PATTERN", "PATH"]),
        )
        .arg(
            Arg::new("case-insensitive")
                .short('i')
                .long("case-insensitive")
                .help("Search case insensitivly")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .help("Only print the number of search results as [files patterns]")
                .long_help(format!(
                    "{}\n{}\n{}\n{}",
                    "Only print the number of search results as [files patterns]",
                    "First number shown is the number of found files with the given pattern",
                    "Second number is the number of found patterns, including multiple hits in one file",
                    "Can be combined with the --stats flag to only show stats and no other output",
                ))
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("depth")
                .short('D')
                .long("depth")
                .help("Set max search depth")
                .long_help(format!(
                    "{}",
                    "Set max search depth",
                ))
                .default_value("250")
                .action(ArgAction::Set)
                .num_args(1)
                .value_name("NUMBER"),
        )
        .arg(
            Arg::new("extension")
                .short('e')
                .long("extension")
                .help("Only search in files with the given extensions")
                .long_help(format!(
                    "{}\n{}",
                    "Only search in files with the given extensions",
                    "Must be provided after the pattern and the search path"
                ))
                .action(ArgAction::Set)
                .num_args(1..)
                .value_name("EXTENSIONS"),
        )
        .arg(
            Arg::new("no-hidden")
                .short('H')
                .long("no-hidden")
                .help("Exclude hidden files and directories from search")
                .long_help(format!(
                    "{}\n{}",
                    "Exclude hidden files and directories from search",
                    "If a directory is hidden all its content will be skiped as well",
                ))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("override")
                .short('o')
                .long("override")
                .help("Override all previously set flags")
                .long_help(format!(
                    "{}\n{}\n{}",
                    "Override all previously set flags",
                    "This can be used when a custom alias for this command is set together with regularly used flags",
                    "This flag allows to disable these flags and specify new ones"
                ))
                // TODO if new args -> add here to this list to override if needed
                .overrides_with_all(["stats", "extension", "no-hidden", "performance", "count"])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("performance")
                .short('p')
                .long("performance")
                .help("Disable spinner, don`t colourize the search output and speed up the output printing")
                .long_help(format!(
                    "{}\n{}\n{}",
                    "Focus on performance",
                    "Disable search indicating spinner and don`t colourize the search output",
                    "Write the output via BufWriter",
                ))
                .conflicts_with("stats")
                .conflicts_with("count")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("stats")
                .short('s')
                .long("stats")
                .help("Show search statistics at the end")
                .long_help(format!(
                    "{}\n{}",
                    "Show search statistics at the end",
                    "Can be combined with the --count flag to only show stats and no other output",
                ))
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("log")
                .short_flag('L')
                .long_flag("log")
                .about("Show content of the log file"),
        )
}

fn search<W: Write>(handle: &mut W, path: &PathBuf, config: &Config) {
    let start = Instant::now();
    let mut entry_count = 0;
    let mut search_hits = 0;
    let mut pattern_hits = 0;

    // disable the search indicating spinner and colourful output
    if config.performance_flag {
        forwards_search_and_catch_errors(
            handle,
            path,
            &config,
            &mut search_hits,
            &mut pattern_hits,
            &mut entry_count,
            None,
        );
    } else {
        // spinner
        let spinner_style = ProgressStyle::with_template("{spinner:.red} {msg}").unwrap();
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(spinner_style);
        pb.set_message(format!("{}", "searching".truecolor(250, 0, 104)));

        forwards_search_and_catch_errors(
            handle,
            path,
            &config,
            &mut search_hits,
            &mut pattern_hits,
            &mut entry_count,
            Some(pb.clone()),
        );

        pb.finish_and_clear();
    }

    // print output
    if config.count_flag && !config.stats_flag {
        println!("{} {}", search_hits.to_string(), pattern_hits.to_string());
    } else if config.stats_flag {
        get_search_hits(search_hits, pattern_hits, entry_count, start);
    }
}

fn forwards_search_and_catch_errors<W: Write>(
    handle: &mut W,
    path: &PathBuf,
    config: &Config,
    search_hits: &mut u64,
    pattern_hits: &mut u64,
    entry_count: &mut u64,
    pb: Option<ProgressBar>,
) {
    if let Err(err) = forwards_search(
        handle,
        path,
        &config,
        search_hits,
        pattern_hits,
        entry_count,
        pb.clone(),
    ) {
        match err.kind() {
            io::ErrorKind::NotFound => {
                warn!("\'{}\' not found: {}", path.display(), err);
            }
            io::ErrorKind::PermissionDenied => {
                warn!(
                    "You don`t have access to a source in \'{}\': {}",
                    path.display(),
                    err
                );
            }
            _ => {
                error!(
                    "Error while scanning entries for {} in \'{}\': {}",
                    config.pattern.italic(),
                    path.display(),
                    err
                );
            }
        }
    };
}

fn forwards_search<W: Write>(
    handle: &mut W,
    path: &PathBuf,
    config: &Config,
    search_hits: &mut u64,
    pattern_hits: &mut u64,
    entry_count: &mut u64,
    pb: Option<ProgressBar>,
) -> io::Result<()> {
    let mut search_path = Path::new(&path).to_path_buf();

    // accept "." as current directory
    if path.as_path().to_string_lossy().to_string() == "." {
        let current_dir = env::current_dir().unwrap_or_else(|err| {
            error!("Unable to get current directory: {err}");
            process::exit(1);
        });
        search_path.push(current_dir);
    }

    // filter files
    let valid_entries = WalkDir::new(search_path)
        .max_depth(config.depth_flag as usize) // set maximum search depth
        .into_iter()
        // TODO bottleneck if it has to filter out hidden files
        .filter_entry(|e| file_check(e, &config)); // handle hidden flag

    for entry in valid_entries {
        let entry = entry?;

        if !entry.file_type().is_file() {
            continue;
        }

        // count searched entries
        *entry_count += 1;

        // handle possible file extensions
        if !config.extensions.is_empty() {
            // get entry extension
            let mut entry_extension = String::new();
            if let Some(extension) = entry.path().extension() {
                entry_extension.push_str(&extension.to_string_lossy().to_string());

                // check if entry_extension matches any given extensions via extensions flag
                if config.extensions.iter().any(|it| &entry_extension == it) {
                    if let Err(err) = match_pattern_and_print(
                        handle,
                        &entry.path().to_path_buf(),
                        &config,
                        pb.clone(),
                        search_hits,
                        pattern_hits,
                    ) {
                        match err.kind() {
                            io::ErrorKind::Interrupted => {
                                warn!("Interrupted: {err}");
                                process::exit(0)
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                }
            }
        } else {
            if let Err(err) = match_pattern_and_print(
                handle,
                &entry.path().to_path_buf(),
                &config,
                pb.clone(),
                search_hits,
                pattern_hits,
            ) {
                match err.kind() {
                    io::ErrorKind::Interrupted => {
                        warn!("Interrupted: {err}");
                        process::exit(0)
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
    }

    Ok(())
}

fn match_pattern_and_print<W: Write>(
    handle: &mut W,
    path: &PathBuf,
    config: &Config,
    pb: Option<ProgressBar>,
    search_hits: &mut u64,
    pattern_hits: &mut u64,
) -> io::Result<()> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut content = String::new();
    buf_reader.read_to_string(&mut content)?;

    // check for pattern match in file via aho-corasick algorithm
    if config.pattern_ac.is_match(&content) {
        *search_hits += 1;

        if config.performance_flag {
            if !config.count_flag {
                writeln!(handle, "\n{}", format!("PATH: {}", path.display())).unwrap_or_else(
                    |err| {
                        error!("Error writing to stdout: {err}");
                    },
                );

                let mut linenumber = 0;
                for line in content.lines() {
                    linenumber += 1;
                    if config.pattern_ac.is_match(&line) {
                        writeln!(handle, "{}", format!(" {}: {}", linenumber, &line))
                            .unwrap_or_else(|err| {
                                error!("Error writing to stdout: {err}");
                            });
                    }
                }
            }
        } else {
            match pb.clone() {
                Some(pb) => {
                    if !config.count_flag {
                        pb.println(format!(
                            "\n{}",
                            path.display().to_string().bold().truecolor(59, 179, 140),
                        ));
                    }

                    let mut linenumber = 0;
                    for line in content.lines() {
                        linenumber += 1;
                        if config.pattern_ac.is_match(&line) {
                            *pattern_hits += 1;

                            if !config.count_flag {
                                let line_with_hi_pattern =
                                    highlight_pattern_in_line(&line, &config);
                                pb.println(format!(
                                    " {}: {}\n",
                                    linenumber.to_string().truecolor(250, 0, 104),
                                    line_with_hi_pattern,
                                ))
                            }
                        }
                    }
                }
                None => {}
            }
        }
    }

    Ok(())
}

fn get_search_hits(search_hits: u64, pattern_hits: u64, entry_count: u64, start: Instant) {
    println!(
        "\n{} {}",
        entry_count.to_string().dimmed(),
        "entries searched".dimmed()
    );

    if search_hits == 0 {
        println!(
            "{}",
            format!(
                "found {} files\nfound {} matches",
                search_hits.to_string().truecolor(250, 0, 104).bold(),
                pattern_hits.to_string().truecolor(250, 0, 104).bold()
            )
        );
    } else if search_hits == 1 {
        println!(
            "{}",
            format!(
                "found {} file\nfound {} match",
                search_hits.to_string().truecolor(59, 179, 140).bold(),
                pattern_hits.to_string().truecolor(59, 179, 140).bold()
            )
        );
    } else {
        println!(
            "{}",
            format!(
                "found {} files\nfound {} matches",
                search_hits.to_string().truecolor(59, 179, 140).bold(),
                pattern_hits.to_string().truecolor(59, 179, 140).bold()
            )
        );
    }

    println!(
        "{}",
        HumanDuration(start.elapsed())
            .to_string()
            .truecolor(112, 110, 255)
    );
}

fn highlight_pattern_in_line(line: &str, config: &Config) -> String {
    // find first byte of pattern in filename
    let pat_in_file = line.find(&config.pattern).unwrap_or_else(|| 9999999999);

    if pat_in_file == 9999999999 {
        // if no pattern found return just the filename
        return line.to_string();
    } else {
        let first_from_name = &line[..pat_in_file];
        let last_from_name = &line[(pat_in_file + config.pattern.len())..];
        // colourize the pattern in the filename
        let highlighted_pattern = config.pattern.truecolor(112, 110, 255).to_string();

        let mut result = String::from(first_from_name);
        result.push_str(&highlighted_pattern);
        result.push_str(last_from_name);

        result.to_string()
    }
}

// check entries if hidden and compare to hidden flag
fn file_check(entry: &DirEntry, config: &Config) -> bool {
    // TODO bottleneck
    if config.no_hidden_flag && is_hidden(&entry.path().to_path_buf()).unwrap_or(false) {
        return false;
    }

    return true;
}

// TODO bottleneck
fn is_hidden(file_path: &PathBuf) -> std::io::Result<bool> {
    let metadata = fs::metadata(file_path)?;
    let attributes = metadata.file_attributes();

    if (attributes & 0x2) > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn check_create_config_dir() -> io::Result<PathBuf> {
    let mut new_dir = PathBuf::new();
    match dirs::config_dir() {
        Some(config_dir) => {
            new_dir.push(config_dir);
            new_dir.push("sf");
            if !new_dir.as_path().exists() {
                fs::create_dir(&new_dir)?;
            }
        }
        None => {
            error!("Unable to find config directory");
        }
    }

    Ok(new_dir)
}

fn show_log_file(config_dir: &PathBuf) -> io::Result<String> {
    let log_path = Path::new(&config_dir).join("sf.log");
    match log_path.try_exists()? {
        true => {
            return Ok(format!(
                "{} {}\n{}",
                "Log location:".italic().dimmed(),
                &log_path.display(),
                fs::read_to_string(&log_path)?
            ));
        }
        false => {
            return Ok(format!(
                "{} {}",
                "No log file found:"
                    .truecolor(250, 0, 104)
                    .bold()
                    .to_string(),
                log_path.display()
            ))
        }
    }
}
