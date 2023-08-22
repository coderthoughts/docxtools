# docxtools

A command-line utility to work with docx files. Can be useful for unix stream-based operations
such as searching through a large amount of docx files on the commandline. It can also be
useful for batch operations.

`docxtools` is written in Rust, platform-specific binary downloads can be found in the Release area.

## Usage

General usage:

```
$ ./docxtools -h
Usage: docxtools [OPTIONS] <IN_FILE> <COMMAND>

Commands:
  cat   List the text from the document to the console
  grep  Search the text in the document like 'grep'
  help  Print this message or the help of the given subcommand(s)

Arguments:
  <IN_FILE>  The docx file to operate on

Options:
  -t, --temp-dir <TEMP_DIR>  The temporary directory to use. If not specified a system temp directory will be used and cleaned after use
  -h, --help                 Print help
  -V, --version              Print version
  ```
