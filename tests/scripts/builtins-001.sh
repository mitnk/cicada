check cp | grep -qE "^.*/cp: .*executable$" && echo "check: test check passed"
ulimit -X >/dev/null 2>&1 || echo 'ulimit: unsupported option does not crash'
