# tmux ici

Save and restore `tmux` sessions based on the working directory.

## Usage

Save current session

```sh
tici save
```

Restore the saved session for the current working directory

```sh
tici
# or
tici restore
```

## Options

Dry run
```sh
tici save -n
```

Specifying working directory
```sh
tici save -d <worker_directory>
```
