echo test right head commands finishes first
yes | head -n 2
yes | head -n 2 | cat
yes | head -n 2 | cat | head | head
