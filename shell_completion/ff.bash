_ff() {
    local i cur prev opts cmds
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${i}" in
            ff)
                cmd="ff"
                ;;
            
            ff)
                cmd+="__ff"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        ff)
            opts=" -g -r -u -i -s -p -L -M -0 -A -S -a -I -h -V -t -d -c -j -x  --glob --regex --unicode --ignore-case --case-sensitive --full-path --follow --mount --print0 --absolute-path --sort-path --all --no-ignore --help --version --type --max-depth --color --threads --max-buffer-time --exec  <DIRECTORY> <PATTERN> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
                return 0
            fi
            case "${prev}" in
                
                --type)
                    COMPREPLY=("<filetype>")
                    return 0
                    ;;
                    -t)
                    COMPREPLY=("<filetype>")
                    return 0
                    ;;
                --max-depth)
                    COMPREPLY=("<number>")
                    return 0
                    ;;
                    -d)
                    COMPREPLY=("<number>")
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto never always" -- ${cur}))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -W "auto never always" -- ${cur}))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=("<number>")
                    return 0
                    ;;
                    -j)
                    COMPREPLY=("<number>")
                    return 0
                    ;;
                --max-buffer-time)
                    COMPREPLY=("<milliseconds>")
                    return 0
                    ;;
                --exec)
                    COMPREPLY=("<program [argument]... [;]>")
                    return 0
                    ;;
                    -x)
                    COMPREPLY=("<program [argument]... [;]>")
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            return 0
            ;;
        
    esac
}

complete -F _ff -o bashdefault -o default ff
