function __fish_using_command
    set cmd (commandline -opc)
    if [ (count $cmd) -eq (count $argv) ]
        for i in (seq (count $argv))
            if [ $cmd[$i] != $argv[$i] ]
                return 1
            end
        end
        return 0
    end
    return 1
end

complete -c ff -n "__fish_using_command ff" -s t -l type -d 'Filter by type: d,directory, f,file, l,symlink, x,executable'
complete -c ff -n "__fish_using_command ff" -s d -l max-depth -d 'Set maximum search depth. [default: none]'
complete -c ff -n "__fish_using_command ff" -s c -l color -d 'When to use colors: auto, never, always [default: auto]' -r -f -a "auto never always"
complete -c ff -n "__fish_using_command ff" -s j -l threads -d 'Set number of threads for searching and command execution.'
complete -c ff -n "__fish_using_command ff" -l max-buffer-time -d 'Set time (in milliseconds) for buffering and sorting.'
complete -c ff -n "__fish_using_command ff" -s x -l exec -d 'Execute the given command for each search result.'
complete -c ff -n "__fish_using_command ff" -s g -l glob -d 'Search with a glob pattern. [default]'
complete -c ff -n "__fish_using_command ff" -s r -l regex -d 'Search with a regex pattern. [default: glob]'
complete -c ff -n "__fish_using_command ff" -s u -l unicode -d 'Match UTF-8 scalar values [default: match bytes]'
complete -c ff -n "__fish_using_command ff" -s i -l ignore-case -d 'Case-insensitive search. [default: case-sensitive]'
complete -c ff -n "__fish_using_command ff" -s s -l case-sensitive -d 'Case-sensitive search. [default]'
complete -c ff -n "__fish_using_command ff" -s p -l full-path -d 'Match full paths. [default: match filename]'
complete -c ff -n "__fish_using_command ff" -s L -l follow -d 'Follow symbolic links.'
complete -c ff -n "__fish_using_command ff" -s M -l mount -d 'Do not descend into directories on other filesystems.'
complete -c ff -n "__fish_using_command ff" -s 0 -l print0 -d 'Terminate each search result with NUL.'
complete -c ff -n "__fish_using_command ff" -s A -l absolute-path -d 'Output absolute paths instead of relative paths.'
complete -c ff -n "__fish_using_command ff" -s S -l sort-path -d 'Sort the results by pathname.'
complete -c ff -n "__fish_using_command ff" -s a -l all -d 'Include dot-files in the search.'
complete -c ff -n "__fish_using_command ff" -s I -l no-ignore -d 'Do not respect .(git)ignore files.'
complete -c ff -n "__fish_using_command ff" -s m -l multiplex -d 'All executed commands receive the same input.'
complete -c ff -n "__fish_using_command ff" -s v -l verbose -d 'Warn about I/O errors, file permissions, symlink loops, etc.'
complete -c ff -n "__fish_using_command ff" -s h -l help -d 'Prints help information. Use --help for more details.'
complete -c ff -n "__fish_using_command ff" -s V -l version -d 'Prints version information'
