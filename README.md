# mg

__Mini Grep__

*simple pattern search in files*

* colourful output and search indicating spinner by default 
  * disable via ```--performance``` flag
* filter by file-extension
* exclude hidden files
  * via ```--no-hidden``` flag
* show number of searched entries, search results and search time
  * via ```--stats``` flag
* only show number of search results 
  * via ```--count``` flag
* search case-insensitivly
  * via ```--case-insensitive``` flag
* set maximum search depth
  * via ```--depth``` flag
* accepts ```.``` as current directory
* no regex search (for now)

## Example

- search for the word _test_ in all files in the current directory and print search statistics at the end

```mg test . -s```

![screenshot](https://github.com/Phydon/mg/blob/master/assets/mg_test_current_s.png)

	
## Usage

### Short Usage

```
mg [OPTIONS] [PATTERN] [PATH] [COMMAND]

Commands:
  log, -L, --log  Show content of the log file
  help            Print this message or the help of the given subcommand(s)

Arguments:
  [PATTERN] [PATH]  Add a search pattern and a path

Options:
  -i, --case-insensitive           Search case insensitivly
  -c, --count                      Only print the number of search results as [files patterns]
  -D, --depth <NUMBER>             Set max search depth [default: 250]
  -e, --extension <EXTENSIONS>...  Only search in files with the given extensions
  -H, --no-hidden                  Exclude hidden files and directories from search
  -o, --override                   Override all previously set flags
  -p, --performance                Disable spinner, don`t colourize the search output and speed up the output printing
  -s, --stats                      Show search statistics at the end
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```

### Long Usage

```
mg [OPTIONS] [PATTERN] [PATH] [COMMAND]

Commands:
  log, -L, --log
          Show content of the log file
  help
          Print this message or the help of the given subcommand(s)

Arguments:
  [PATTERN] [PATH]
          Add a search pattern and a path

Options:
  -i, --case-insensitive
          Search case insensitivly

  -c, --count
          Only print the number of search results as [files patterns]
          First number shown is the number of found files with the given pattern
          Second number is the number of found patterns, including multiple hits in one file
          Can be combined with the --stats flag to only show stats and no other output

  -D, --depth <NUMBER>
          Set max search depth

          [default: 250]

  -e, --extension <EXTENSIONS>...
          Only search in files with the given extensions
          Must be provided after the pattern and the search path

  -H, --no-hidden
          Exclude hidden files and directories from search
          If a directory is hidden all its content will be skiped as well

  -o, --override
          Override all previously set flags
          This can be used when a custom alias for this command is set together with regularly used flags
          This flag allows to disable these flags and specify new ones

  -p, --performance
          Focus on performance
          Disable search indicating spinner and don`t colourize the search output
          Write the output via BufWriter

  -s, --stats
          Show search statistics at the end
          Can be combined with the --count flag to only show stats and no other output

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```


## Installation

### Windows

via Cargo or get the ![binary](https://github.com/Phydon/mg/releases)

