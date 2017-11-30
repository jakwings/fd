
@('ff', './ff') | %{
    Register-ArgumentCompleter -Native -CommandName $_ -ScriptBlock {
        param($wordToComplete, $commandAst, $cursorPosition)

        $command = '_ff'
        $commandAst.CommandElements |
            Select-Object -Skip 1 |
            %{
                switch ($_.ToString()) {

                    'ff' {
                        $command += '_ff'
                        break
                    }

                    default { 
                        break
                    }
                }
            }

        $completions = @()

        switch ($command) {

            '_ff' {
                $completions = @('-g', '-r', '-u', '-i', '-s', '-p', '-L', '-M', '-0', '-A', '-S', '-a', '-I', '-h', '-V', '-t', '-d', '-c', '-j', '-x', '--glob', '--regex', '--unicode', '--ignore-case', '--case-sensitive', '--full-path', '--follow', '--mount', '--print0', '--absolute-path', '--sort-path', '--all', '--no-ignore', '--help', '--version', '--type', '--max-depth', '--color', '--threads', '--max-buffer-time', '--exec')
            }

        }

        $completions |
            ?{ $_ -like "$wordToComplete*" } |
            Sort-Object |
            %{ New-Object System.Management.Automation.CompletionResult $_, $_, 'ParameterValue', $_ }
    }
}
