# docxtools

A command-line utility to work with docx files. Can be useful for unix stream-based operations
such as searching through a large amount of docx files on the commandline. It can also be
useful for batch operations.

`docxtools` is written in Rust, platform-specific binary downloads can be found in the Release area: https://github.com/coderthoughts/docxtools/releases

## Usage

General usage:

```
$ .docxtools
Usage: docxtools [OPTIONS] <IN_FILE> <COMMAND>

Commands:
  cat            List the text from the document to the console
  grep           Search the text in the document like 'grep'
  replace        Search and replace in document text and tables
  replace-links  Search and replace hyperlinks in the document
  help           Print this message or the help of the given subcommand(s)

Arguments:
  <IN_FILE>  The docx file to operate on

Options:
  -t, --temp-dir <TEMP_DIR>  The temporary directory to use. If not specified a system temp directory will be used and cleaned after use
  -h, --help                 Print help
  -V, --version              Print version
```

## Example usage:

### List text contents of a docx file

```
$ docxtools docs/test.docx cat
docs/test.docx: Normal.dotm
...
docs/test.docx: Microsoft Office Word
...
docs/test.docx: 2023-08-25T10:13:00Z
docs/test.docx: A test document written in Microsoft Word.
```

### Search a directory of docx files for a specific text

The `grep` subcommand supports regex syntax to find text.

```
$ find docs/folder -name "*.docx" -exec docxtools {} grep '[tT]ext' \;
docs/folder/sample1.docx: text
docs/folder/sample1.docx: Text
docs/folder/sample2.docx: This is a different file that also contains some textual content.
```

### Replace all occurrences of a word with another

Change the word 'Test' or 'test' into zzzz and write the modifications to a new file `test_mod1.docx`:

```
$ docxtools docs/test.docx replace '[Tt]est' zzzz docs/test_mod1.docx
```

### Replace all occurrences of a hyperlink with another

Replace all occurrences of `https://main--test--hlxsites.hlx.page` with `https://foo.bar.com`. Any subpaths after the
URLs are kept, so `https://main--test--hlxsites.hlx.page/contact` will become `https://foo.bar.com/contact`.

```
$ docxtools docs/links.docx replace-links https://main--test--hlxsites.hlx.page https://foo.bar.com
```
