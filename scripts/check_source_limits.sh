#!/bin/sh

set -eu

max_file_lines=500
max_function_lines=60
failed=0

for file in $(find src -type f -name '*.rs' -print | sort); do
    line_count=$(wc -l < "$file")
    if [ "$line_count" -gt "$max_file_lines" ]; then
        echo "File exceeds ${max_file_lines}-line limit: ${file} (${line_count} lines)" >&2
        failed=1
    fi

    if ! awk -v file="$file" -v limit="$max_function_lines" '
        function report_function() {
            if (function_lines > limit) {
                printf "Function exceeds %d-line limit: %s:%d (%d lines)\n", limit, file, start_line, function_lines > "/dev/stderr"
                failed = 1
            }
        }

        /^[[:space:]]*((pub([[:space:]]*\([^)]*\))?[[:space:]]+)?(const[[:space:]]+|async[[:space:]]+|unsafe[[:space:]]+)*fn[[:space:]])/ {
            if (in_function) {
                report_function()
            }
            in_function = 1
            start_line = NR
            function_lines = 0
            brace_depth = 0
        }

        in_function {
            function_lines++
            brace_depth += gsub(/\{/, "{")
            brace_depth -= gsub(/\}/, "}")
            if (brace_depth == 0 && index($0, "{") > 0) {
                report_function()
                in_function = 0
            }
        }

        END {
            if (in_function) {
                report_function()
            }
            exit failed
        }
    ' "$file"; then
        failed=1
    fi
done

exit "$failed"
