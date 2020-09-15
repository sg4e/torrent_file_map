# torrent_file_map

Multiplatform command-line utility to recover the mapping of `.torrent` files to downloaded files.

By sg4e.

## Purpose

Torrent clients enable many customizations of files downloaded through torrents, such as renaming or moving to new directories. The relationship between the torrents and the files on the local disk is usually maintained in a single large file. If this critical file is lost, damaged, or corrupted, recovering the connection between the local files and their torrents is not trivially achievable. This utility seeks to assist in rebuilding these mappings where possible.

`torrent_file_map` walks specified directories and compares file sizes and names with local `.torrent` metadata files to establish possible connections. Then these connections are verified by comparing partial hashes of local files with `.torrent` piece hashes. `torrent_file_map` attempts to be robust against root-directory renames and partial downloads.

`torrent_file_map` prints successfully recovered mappings to the command line in the format:

```
torrent-file.torrent: /path/to/file/or/root-directory
```

## Usage

`torrent_file_map` handles its own file globbing. Single quotes or escaping is required so that the shell does not expand globs/wildcards. `torrent_file_map` expects exactly 2 arguments. Example:

```bash
./torrent_file_map '/home/me/torrent-metadata/**/*.torrent' /home/me/torrent-downloads
```

## TODO

`torrent_file_map` currently lacks hash checking for torrents with multiple files. A possible match is still given for these torrents.